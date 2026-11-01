use air01::{
    protocol::packet,
    receiver::{self, RawReports, Status},
};
use anyhow::Result;
use std::{collections::VecDeque, time::Duration};
struct Fake {
    reads: VecDeque<Vec<u8>>,
    sent: Vec<[u8; 65]>,
    waits: Vec<Duration>,
}
impl RawReports for Fake {
    fn send(&mut self, p: &[u8; 65]) -> Result<()> {
        self.sent.push(*p);
        Ok(())
    }
    fn read(&mut self) -> Result<Vec<u8>> {
        Ok(self.reads.pop_front().expect("unexpected read"))
    }
    fn wait(&mut self, d: Duration) {
        self.waits.push(d);
    }
}
fn status(ready: bool, online: bool, send: bool) -> Vec<u8> {
    let mut r = vec![0; 65];
    r[1] = u8::from(ready);
    r[2] = 87;
    r[4] = u8::from(!online);
    r[6] = u8::from(send);
    r[7] = 1;
    r
}
#[test]
fn relay_selects_keyboard_once_and_releases_a_query_reply() {
    let mut reply = vec![0; 65];
    reply[1] = 0x8f;
    reply[2..6].copy_from_slice(&3013u32.to_le_bytes());
    let mut f = Fake {
        reads: [
            status(false, true, true),
            status(false, true, true),
            status(true, true, true),
            reply.clone(),
        ]
        .into(),
        sent: vec![],
        waits: vec![],
    };
    assert_eq!(
        receiver::exchange(&mut f, packet(0x8f, &[], None).unwrap(), true).unwrap(),
        reply
    );
    assert_eq!(
        f.sent.iter().map(|p| p[1]).collect::<Vec<_>>(),
        [0xf7, 0xf6, 0x8f, 0xf7, 0xf7, 0xfc]
    );
    assert_eq!(f.sent[1][2], 10);
    assert_eq!(f.sent.iter().filter(|p| p[1] == 0x8f).count(), 1);
}
#[test]
fn relay_write_does_not_wait_for_a_nonexistent_reply() {
    let mut f = Fake {
        reads: [status(false, true, true)].into(),
        sent: vec![],
        waits: vec![],
    };
    assert!(
        receiver::exchange(&mut f, packet(4, &[1], None).unwrap(), false)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        f.sent.iter().map(|p| p[1]).collect::<Vec<_>>(),
        [0xf7, 0xf6, 4]
    );
}
#[test]
fn offline_busy_and_short_status_never_forward_a_keyboard_packet() {
    for replies in [
        vec![status(false, false, true)],
        vec![status(false, true, false); 10],
        vec![vec![0; 5]],
    ] {
        let mut f = Fake {
            reads: replies.into(),
            sent: vec![],
            waits: vec![],
        };
        assert!(receiver::exchange(&mut f, packet(0x8f, &[], None).unwrap(), true).is_err());
        assert!(f.sent.iter().all(|p| p[1] == 0xf7));
    }
}
#[test]
fn missing_reply_has_only_one_bounded_read_retry() {
    let mut replies = vec![status(false, true, true)];
    replies.extend(vec![status(false, true, true); 10]);
    replies.push(status(false, true, true));
    replies.extend(vec![status(false, true, true); 10]);
    let mut f = Fake {
        reads: replies.into(),
        sent: vec![],
        waits: vec![],
    };
    assert!(receiver::exchange(&mut f, packet(0x87, &[], None).unwrap(), true).is_err());
    assert_eq!(f.sent.iter().filter(|p| p[1] == 0x87).count(), 2);
    assert!(!f.sent.iter().any(|p| p[1] == 0xfc));
}
#[test]
fn status_rejects_unpaired_or_booting_receivers() {
    let mut r = status(false, true, true);
    r[7] = 2;
    assert!(Status::parse(&r).unwrap().ensure_keyboard().is_err());
    r[7] = 1;
    r[8] = 1;
    r[9] = 1;
    assert!(Status::parse(&r).unwrap().ensure_keyboard().is_err());
}
#[test]
fn unused_status_tail_is_not_mistaken_for_rf_boot() {
    let mut r = status(false, true, true);
    r[8] = 109;
    assert!(Status::parse(&r).unwrap().ensure_keyboard().is_ok());
}
#[test]
fn session_selects_target_once_and_keeps_quiet_gaps() {
    let mut f = Fake {
        reads: [status(false, true, true), status(false, true, true)].into(),
        sent: vec![],
        waits: vec![],
    };
    let mut session = receiver::Receiver::default();
    session
        .exchange(&mut f, packet(4, &[1], None).unwrap(), false)
        .unwrap();
    session
        .exchange(&mut f, packet(4, &[2], None).unwrap(), false)
        .unwrap();
    assert_eq!(f.sent.iter().filter(|p| p[1] == 0xf6).count(), 1);
    assert!(
        f.waits
            .iter()
            .filter(|d| **d >= Duration::from_millis(200))
            .count()
            >= 2,
        "each logical command needs quiet radio time"
    );
}
#[test]
fn queued_reply_is_drained_before_a_new_command() {
    let mut f = Fake {
        reads: [status(true, true, true), vec![0; 65]].into(),
        sent: vec![],
        waits: vec![],
    };
    receiver::exchange(&mut f, packet(4, &[1], None).unwrap(), false).unwrap();
    assert_eq!(
        f.sent.iter().map(|p| p[1]).collect::<Vec<_>>(),
        [0xf7, 0xfc, 0xf6, 4]
    );
}

#[test]
fn dropped_read_can_recover_without_repeating_writes() {
    let mut replies = vec![status(false, true, true)];
    replies.extend(vec![status(false, true, true); 10]);
    replies.push(status(false, true, true));
    replies.push(status(true, true, true));
    let mut answer = vec![0; 65];
    answer[1] = 0x87;
    replies.push(answer.clone());
    let mut f = Fake {
        reads: replies.into(),
        sent: vec![],
        waits: vec![],
    };
    assert_eq!(
        receiver::exchange(&mut f, packet(0x87, &[], None).unwrap(), true).unwrap(),
        answer
    );
    assert_eq!(f.sent.iter().filter(|p| p[1] == 0x87).count(), 2);
}
