mod common;
use air01::{device::Board, transport::decode_report};
use common::MemoryWire;
#[test]
fn unknown_identity_and_protocol_are_rejected_before_writes() {
    let w = MemoryWire::new();
    w.0.borrow_mut().device_id = 77;
    assert!(Board::connect(w.clone(), true).is_err());
    assert!(w.0.borrow().writes.is_empty());
    let w = MemoryWire::new();
    w.0.borrow_mut().gen2 = false;
    assert!(Board::connect(w.clone(), true).is_err());
    assert!(w.0.borrow().writes.is_empty());
}
#[test]
fn complete_snapshot_reads_all_profiles_and_os_fn_layers() {
    let w = MemoryWire::new();
    let mut b = Board::connect(w.clone(), true).unwrap();
    let s = b.snapshot().unwrap();
    assert_eq!(s, w.0.borrow().snapshot);
    assert_eq!(s.profiles.len(), 4);
    assert_eq!(s.active_profile, 2);
    assert!(w.0.borrow().writes.is_empty());
}
#[test]
fn short_or_wrong_report_id_is_an_error() {
    assert!(decode_report(&[0; 64]).is_err());
    let mut r = [0; 65];
    r[0] = 3;
    assert!(decode_report(&r).is_err());
    r[0] = 0;
    r[1] = 143;
    assert_eq!(decode_report(&r).unwrap()[0], 143);
}
