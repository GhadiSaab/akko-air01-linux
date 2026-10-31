use air01::{protocol::*, snapshot::*};
#[test]
fn packets_match_gen2_wire_bytes() {
    let p = packet(0x0a, &[2, 5, 0, 0, 1, 0], Some(Action::CTRL)).unwrap();
    assert_eq!(&p[..9], &[0, 0x0a, 2, 5, 0, 0, 1, 0, 0xed]);
    assert_eq!(&p[9..13], &[0, 0, 224, 0]);
    let r = packet(0x8a, &[2, 255, 3, 0], None).unwrap();
    assert_eq!(&r[..9], &[0, 0x8a, 2, 255, 3, 0, 0, 0, 0x71]);
    assert!(packet(1, &[0; 7], None).is_err());
}
#[test]
fn rejects_out_of_range_profile_and_slot() {
    assert!(check_profile(3).is_ok());
    assert!(check_profile(4).is_err());
    assert!(check_slot(127).is_ok());
    assert!(check_slot(128).is_err());
}
#[test]
fn decodes_both_ctrl_encodings_and_preserves_unknown() {
    assert_eq!(Action([0, 224, 0, 0]).label(), "LeftCtrl");
    assert_eq!(Action::CTRL.label(), "LeftCtrl");
    assert_eq!(Action::FN.label(), "Fn");
    assert!(Action([99, 2, 3, 4]).label().contains("63:02:03:04"));
}
#[test]
fn family_comes_from_answers_not_chip_name() {
    let mut options = [0; 64];
    options[0] = 0x89;
    let mut keys = [0; 64];
    for i in 0..16 {
        keys[i * 4 + 2] = 4 + i as u8;
    }
    assert!(detect_gen2(&options, &keys, &[0; 64]));
    assert!(!detect_gen2(&keys, &options, &[0; 64]));
    assert!(!detect_gen2(&options, &[255; 64], &[0; 64]));
}
fn snapshot() -> Snapshot {
    Snapshot {
        schema: 1,
        device_id: 3013,
        active_profile: 2,
        profiles: (0..4)
            .map(|i| ProfileSnapshot {
                index: i,
                base: vec![Action::FN; 128],
                fn_win: vec![Action([0; 4]); 128],
                fn_mac: vec![Action([0; 4]); 128],
            })
            .collect(),
    }
}
#[test]
fn snapshot_rejects_foreign_incomplete_and_duplicate_profiles() {
    let good = snapshot();
    assert!(good.validate().is_ok());
    let mut b = good.clone();
    b.device_id = 42;
    assert!(b.validate().is_err());
    let mut b = good.clone();
    b.profiles[0].base.pop();
    assert!(b.validate().is_err());
    let mut b = good.clone();
    b.profiles[1].index = 0;
    assert!(b.validate().is_err());
    let mut b = good.clone();
    b.active_profile = 4;
    assert!(b.validate().is_err());
    let mut b = good;
    b.schema = 2;
    assert!(b.validate().is_err());
}
