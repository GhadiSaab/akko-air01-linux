//! Receiver relay researched from sharkfin; see NOTICE. No firmware/pair commands.
use crate::{protocol::packet, transport::decode_report};
use anyhow::{Result, ensure};
use std::time::Duration;
pub trait RawReports {
    fn send(&mut self, packet: &[u8; 65]) -> Result<()>;
    fn read(&mut self) -> Result<Vec<u8>>;
    fn wait(&mut self, d: Duration) {
        std::thread::sleep(d)
    }
}
#[derive(Debug, Clone, Copy)]
pub struct Status {
    pub reply_ready: bool,
    pub battery: u8,
    pub online: bool,
    pub can_send: bool,
    pub has_keyboard: bool,
    pub booting: bool,
}
impl Status {
    pub fn parse(report: &[u8]) -> Result<Self> {
        let r = decode_report(report)?;
        ensure!([1, 2, 3].contains(&r[6]), "invalid receiver status");
        ensure!(
            [0, 1].contains(&r[0]) && [0, 1].contains(&r[3]) && [0, 1].contains(&r[5]),
            "malformed receiver flags"
        );
        Ok(Self {
            reply_ready: r[0] == 1,
            battery: r[1],
            online: r[3] == 0,
            can_send: r[5] == 1,
            has_keyboard: matches!(r[6], 1 | 3),
            booting: r[7] == 1 && r[8] == 1,
        })
    }
    pub fn ensure_keyboard(&self) -> Result<()> {
        ensure!(self.has_keyboard, "receiver has no paired keyboard");
        ensure!(!self.booting, "receiver is in RF boot mode; reconnect it");
        ensure!(
            self.online,
            "keyboard is asleep or switched off; press a key and try again"
        );
        Ok(())
    }
}
fn send<I: RawReports>(io: &mut I, p: &[u8; 65]) -> Result<()> {
    io.send(p)?;
    io.wait(Duration::from_millis(20));
    Ok(())
}
fn status<I: RawReports>(io: &mut I) -> Result<Status> {
    send(io, &packet(0xf7, &[], None)?)?;
    Status::parse(&io.read()?)
}
#[derive(Debug)]
struct ReplyTimeout;
impl std::fmt::Display for ReplyTimeout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "receiver did not return a keyboard reply")
    }
}
impl std::error::Error for ReplyTimeout {}
/// One relay session. No idle polling, and no duplicate selects on each query.
#[derive(Default)]
pub struct Receiver {
    selected: bool,
}
impl Receiver {
    pub fn exchange<I: RawReports>(
        &mut self,
        io: &mut I,
        p: [u8; 65],
        read: bool,
    ) -> Result<Vec<u8>> {
        // Leave radio time for typing between logical configuration commands.
        io.wait(Duration::from_millis(200));
        let mut result = self.exchange_inner(io, p, read);
        if read
            && result
                .as_ref()
                .is_err_and(|e| e.downcast_ref::<ReplyTimeout>().is_some())
        {
            self.selected = false;
            io.wait(Duration::from_millis(250));
            result = self.exchange_inner(io, p, true);
        }
        if result.is_err() {
            self.selected = false;
        }
        result
    }
    fn exchange_inner<I: RawReports>(
        &mut self,
        io: &mut I,
        p: [u8; 65],
        read: bool,
    ) -> Result<Vec<u8>> {
        let mut ready = false;
        let mut last = None;
        for _ in 0..10 {
            let s = status(io)?;
            s.ensure_keyboard()?;
            last = Some(s);
            // Drain a previous interrupted read before accepting another command.
            if s.reply_ready {
                send(io, &packet(0xfc, &[], None)?)?;
                decode_report(&io.read()?)?;
            }
            if s.can_send {
                ready = true;
                break;
            }
            io.wait(Duration::from_millis(100));
        }
        ensure!(
            ready,
            "receiver remained busy; no keyboard command sent (last status: {last:?})"
        );
        if !self.selected {
            send(io, &packet(0xf6, &[10], None)?)?;
            self.selected = true;
        }
        send(io, &p)?;
        if !read {
            return Ok(vec![]);
        }
        // A keyboard reply takes about100ms. Do not hammer empty status reads.
        io.wait(Duration::from_millis(100));
        for _ in 0..10 {
            let s = status(io)?;
            s.ensure_keyboard()?;
            if s.reply_ready {
                send(io, &packet(0xfc, &[], None)?)?;
                let r = io.read()?;
                decode_report(&r)?;
                return Ok(r);
            }
            io.wait(Duration::from_millis(100));
        }
        Err(ReplyTimeout.into())
    }
}
/// Single-operation helper; the HID worker retains a Receiver for its session.
pub fn exchange<I: RawReports>(io: &mut I, p: [u8; 65], read: bool) -> Result<Vec<u8>> {
    Receiver::default().exchange(io, p, read)
}
