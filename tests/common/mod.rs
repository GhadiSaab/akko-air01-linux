#![allow(dead_code)]
use air01::{
    protocol::Action,
    snapshot::{ProfileSnapshot, Snapshot},
    transport::Wire,
};
use anyhow::{Result, bail};
use std::{cell::RefCell, rc::Rc, time::Duration};
#[derive(Clone)]
pub struct MemoryWire(pub Rc<RefCell<State>>);
pub struct State {
    pub snapshot: Snapshot,
    pub device_id: u32,
    pub gen2: bool,
    pub receiver: bool,
    pub lighting: [u8; 7],
    pub writes: Vec<[u8; 65]>,
    pub fail_at: Option<usize>,
    pub fail_from: Option<usize>,
    pub fail_page_after_write: Option<usize>,
    pub ignore_at: Option<usize>,
    pub always_ignore: bool,
}
impl MemoryWire {
    pub fn new() -> Self {
        let mut base = (0..128)
            .map(|i| Action([0, 0, 4 + (i % 26) as u8, 0]))
            .collect::<Vec<_>>();
        base[5] = Action::FN;
        base[11] = Action::CTRL;
        Self(Rc::new(RefCell::new(State {
            snapshot: Snapshot {
                schema: 1,
                device_id: 3013,
                active_profile: 2,
                profiles: (0..4)
                    .map(|i| ProfileSnapshot {
                        index: i,
                        base: base.clone(),
                        fn_win: vec![Action::DISABLED; 128],
                        fn_mac: vec![Action::DISABLED; 128],
                    })
                    .collect(),
            },
            device_id: 3013,
            gen2: true,
            receiver: false,
            lighting: [1, 4, 4, 8, 255, 0, 89],
            writes: vec![],
            fail_at: None,
            fail_from: None,
            fail_page_after_write: None,
            ignore_at: None,
            always_ignore: false,
        })))
    }
}
impl Wire for MemoryWire {
    fn is_receiver(&self) -> bool {
        self.0.borrow().receiver
    }
    fn query(&mut self, op: u8, args: &[u8]) -> Result<[u8; 64]> {
        let mut s = self.0.borrow_mut();
        if op == 0x8a && s.fail_page_after_write == Some(s.writes.len()) {
            s.fail_page_after_write = None;
            bail!("transient page read failure after landed write");
        }
        let mut r = [0; 64];
        match op {
            0x8f => {
                r[0] = op;
                r[1..5].copy_from_slice(&s.device_id.to_le_bytes());
                r[7] = 0x10;
                r[8] = 1;
            }
            0x87 => {
                r[0] = op;
                r[1..8].copy_from_slice(&s.lighting);
            }
            0x89 => {
                r[0] = op;
            }
            0x92 => {
                r[0] = op;
            }
            0x84 => {
                r[0] = op;
                r[1] = s.snapshot.active_profile;
            }
            0x8a => {
                if !s.gen2 {
                    return Ok([255; 64]);
                }
                let p = &s.snapshot.profiles[args[0] as usize];
                let start = args[2] as usize * 16;
                for (i, a) in p.base[start..start + 16].iter().enumerate() {
                    r[i * 4..i * 4 + 4].copy_from_slice(&a.0);
                }
            }
            0x90 => {
                let p = &s.snapshot.profiles[args[1] as usize];
                let layer = if args[0] == 0 { &p.fn_win } else { &p.fn_mac };
                let start = args[3] as usize * 16;
                for (i, a) in layer[start..start + 16].iter().enumerate() {
                    r[i * 4..i * 4 + 4].copy_from_slice(&a.0);
                }
            }
            _ => bail!("unexpected fake query {op:x}"),
        }
        Ok(r)
    }
    fn send(&mut self, p: [u8; 65]) -> Result<()> {
        let mut s = self.0.borrow_mut();
        s.writes.push(p);
        let n = s.writes.len();
        if s.fail_at == Some(n) || s.fail_from.is_some_and(|from| n >= from) {
            bail!("simulated send failure");
        }
        if s.ignore_at == Some(n) || s.always_ignore {
            return Ok(());
        }
        match p[1] {
            0x0a => {
                let profile = p[2] as usize;
                let slot = p[3] as usize;
                assert!(p[7] == 0 && p[6] == 1);
                s.snapshot.profiles[profile].base[slot] = Action(p[9..13].try_into().unwrap());
            }
            0x10 => {
                let profile = p[3] as usize;
                let slot = p[4] as usize;
                let layer = if p[2] == 0 {
                    &mut s.snapshot.profiles[profile].fn_win
                } else {
                    &mut s.snapshot.profiles[profile].fn_mac
                };
                layer[slot] = Action(p[9..13].try_into().unwrap());
            }
            0x07 => s.lighting = p[2..9].try_into().unwrap(),
            0x04 => s.snapshot.active_profile = p[2],
            0x12 => {}
            _ => bail!("unexpected fake write {:x}", p[1]),
        }
        Ok(())
    }
    fn wait(&mut self, _: Duration) {}
}
