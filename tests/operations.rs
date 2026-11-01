mod common;
use air01::{
    device::{Board, Layer},
    operations::*,
    protocol::Action,
};
use common::MemoryWire;
#[test]
fn swap_is_idempotent_preserves_unknowns_and_active_profile() {
    let w = MemoryWire::new();
    w.0.borrow_mut().snapshot.profiles[0].base[100] = Action([98, 1, 2, 3]);
    let mut b = Board::connect(w.clone(), true).unwrap();
    let s = b.snapshot().unwrap();
    let plan = plan_swap(&s, &[0, 1, 2, 3]).unwrap();
    assert_eq!(plan.len(), 8);
    assert_eq!(apply(&mut b, &s, &plan, false).unwrap(), 8);
    let after = b.snapshot().unwrap();
    assert_eq!(after.active_profile, 2);
    assert_eq!(after.profiles[0].base[100], Action([98, 1, 2, 3]));
    for p in &after.profiles {
        assert_eq!(p.base[5], Action::CTRL);
        assert_eq!(p.base[11], Action::FN);
    }
    let plan = plan_swap(&after, &[0, 1, 2, 3]).unwrap();
    assert!(plan.is_empty());
    let n = w.0.borrow().writes.len();
    apply(&mut b, &after, &plan, false).unwrap();
    assert_eq!(w.0.borrow().writes.len(), n);
    // Replaying the actual wire writes must never leave a profile without Fn.
    let mut replay = s.clone();
    for p in &w.0.borrow().writes {
        let pr = p[2] as usize;
        replay.profiles[pr].base[p[3] as usize] = Action(p[9..13].try_into().unwrap());
        assert!(replay.profiles[pr].base.contains(&Action::FN));
    }
}
#[test]
fn rejects_no_fn_and_stale_plan_before_sending() {
    let w = MemoryWire::new();
    let mut b = Board::connect(w.clone(), true).unwrap();
    let s = b.snapshot().unwrap();
    let plan = plan_remap(&s, 2, Layer::Base, 5, Action::CTRL).unwrap();
    assert!(apply(&mut b, &s, &plan, false).is_err());
    assert!(w.0.borrow().writes.is_empty());
    let plan = plan_swap(&s, &[2]).unwrap();
    w.0.borrow_mut().snapshot.profiles[2].base[11] = Action([0, 0, 5, 0]);
    assert!(apply(&mut b, &s, &plan, false).is_err());
    assert!(w.0.borrow().writes.is_empty());
}
#[test]
fn foreign_or_incomplete_restore_fails_before_io() {
    let w = MemoryWire::new();
    let mut b = Board::connect(w.clone(), true).unwrap();
    let s = b.snapshot().unwrap();
    let mut foreign = s.clone();
    foreign.device_id = 42;
    assert!(plan_restore(&s, &foreign).is_err());
    let mut short = s.clone();
    short.profiles[0].fn_win.pop();
    assert!(plan_restore(&s, &short).is_err());
    assert!(w.0.borrow().writes.is_empty());
}
#[test]
fn failed_second_write_restores_original_configuration() {
    let w = MemoryWire::new();
    let mut b = Board::connect(w.clone(), true).unwrap();
    let s = b.snapshot().unwrap();
    let plan = plan_swap(&s, &[2]).unwrap();
    w.0.borrow_mut().fail_at = Some(2);
    let err = apply(&mut b, &s, &plan, false).unwrap_err().to_string();
    assert!(err.contains("rollback verified"), "{err}");
    assert_eq!(b.snapshot().unwrap(), s);
}
#[test]
fn ignored_write_is_not_reported_as_success() {
    let w = MemoryWire::new();
    let mut b = Board::connect(w.clone(), true).unwrap();
    let s = b.snapshot().unwrap();
    let plan = plan_swap(&s, &[2]).unwrap();
    w.0.borrow_mut().ignore_at = Some(1);
    let err = apply(&mut b, &s, &plan, false).unwrap_err().to_string();
    assert!(err.contains("read-back"), "{err}");
    assert_eq!(b.snapshot().unwrap(), s);
}
#[test]
fn identity_or_profile_change_prevents_write_and_wireless_is_readonly() {
    for mode in 0..3 {
        let w = MemoryWire::new();
        let mut b = Board::connect(w.clone(), mode != 2).unwrap();
        let s = b.snapshot().unwrap();
        let plan = plan_swap(&s, &[2]).unwrap();
        if mode == 0 {
            w.0.borrow_mut().device_id = 99;
        }
        if mode == 1 {
            w.0.borrow_mut().snapshot.active_profile = 1;
        }
        assert!(apply(&mut b, &s, &plan, false).is_err());
        assert!(w.0.borrow().writes.is_empty());
    }
}
#[test]
fn duplicate_coordinates_and_forged_before_values_are_rejected() {
    let w = MemoryWire::new();
    let mut b = Board::connect(w.clone(), true).unwrap();
    let s = b.snapshot().unwrap();
    let mut plan = plan_swap(&s, &[2]).unwrap();
    plan.push(plan[0].clone());
    assert!(apply(&mut b, &s, &plan, false).is_err());
    assert!(w.0.borrow().writes.is_empty());
}
#[test]
fn swap_keeps_existing_equivalent_ctrl_encoding_without_flash_writes() {
    let w = MemoryWire::new();
    {
        let mut s = w.0.borrow_mut();
        s.snapshot.profiles[0].base[5] = Action([0, 224, 0, 0]);
        s.snapshot.profiles[0].base[11] = Action::FN;
    }
    let mut b = Board::connect(w.clone(), true).unwrap();
    let s = b.snapshot().unwrap();
    assert!(plan_swap(&s, &[0]).unwrap().is_empty());
}
#[test]
fn backup_is_private_and_never_overwrites_recovery_data() {
    use std::os::unix::fs::PermissionsExt;
    let w = MemoryWire::new();
    let mut b = Board::connect(w, true).unwrap();
    let s = b.snapshot().unwrap();
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("backup.json");
    save_snapshot(&s, &p).unwrap();
    assert_eq!(
        std::fs::metadata(&p).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(load_snapshot(&p).unwrap(), s);
    assert!(save_snapshot(&s, &p).is_err());
    assert_eq!(load_snapshot(&p).unwrap(), s);
}
#[test]
fn recovery_failure_is_reported_as_incomplete() {
    let w = MemoryWire::new();
    let mut b = Board::connect(w.clone(), true).unwrap();
    let s = b.snapshot().unwrap();
    let plan = plan_swap(&s, &[2]).unwrap();
    {
        let mut state = w.0.borrow_mut();
        state.fail_at = Some(2);
        state.fail_from = Some(3);
    }
    let err = apply(&mut b, &s, &plan, false).unwrap_err().to_string();
    assert!(err.contains("rollback INCOMPLETE"), "{err}");
    assert_ne!(b.snapshot().unwrap(), s);
}
#[test]
fn select_profile_is_verified_and_noop_does_not_write() {
    let w = MemoryWire::new();
    let mut b = Board::connect(w.clone(), true).unwrap();
    select_profile(&mut b, 2).unwrap();
    assert!(w.0.borrow().writes.is_empty());
    select_profile(&mut b, 1).unwrap();
    assert_eq!(b.active_profile().unwrap(), 1);
}
#[test]
fn failed_fn_recovery_retains_replacement_instead_of_removing_last_fn() {
    let w = MemoryWire::new();
    let mut b = Board::connect(w.clone(), true).unwrap();
    let s = b.snapshot().unwrap();
    let plan = plan_swap(&s, &[2]).unwrap();
    {
        let mut state = w.0.borrow_mut();
        state.fail_page_after_write = Some(2);
        state.fail_at = Some(3);
    }
    let err = apply(&mut b, &s, &plan, false).unwrap_err().to_string();
    assert!(err.contains("rollback INCOMPLETE"));
    let actual = b.snapshot().unwrap();
    assert!(
        actual.profiles[2].base.contains(&Action::FN),
        "rollback removed the last Fn: {err}"
    );
    assert_eq!(actual.profiles[2].base[11], Action::FN);
}
#[test]
fn ui_backup_refresh_preserves_intervening_unrelated_bindings() {
    let w = MemoryWire::new();
    let mut b = Board::connect(w.clone(), true).unwrap();
    let cached = b.snapshot().unwrap();
    let plan = plan_swap(&cached, &[2]).unwrap();
    w.0.borrow_mut().snapshot.profiles[1].base[90] = Action([0, 0, 69, 0]);
    let fresh = fresh_for_plan(&mut b, &cached, &plan).unwrap();
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("fresh.json");
    save_snapshot(&fresh, &p).unwrap();
    assert_eq!(
        load_snapshot(&p).unwrap().profiles[1].base[90],
        Action([0, 0, 69, 0])
    );
    assert!(w.0.borrow().writes.is_empty());
    w.0.borrow_mut().snapshot.profiles[2].base[11] = Action([0, 0, 4, 0]);
    assert!(fresh_for_plan(&mut b, &cached, &plan).is_err());
    assert!(w.0.borrow().writes.is_empty());
}
#[test]
fn ui_refresh_rejects_changed_active_profile() {
    let w = MemoryWire::new();
    let mut b = Board::connect(w.clone(), true).unwrap();
    let cached = b.snapshot().unwrap();
    w.0.borrow_mut().snapshot.active_profile = 3;
    assert!(fresh_for_plan(&mut b, &cached, &[]).is_err());
    assert!(w.0.borrow().writes.is_empty());
}
#[test]
fn receiver_remaps_only_the_active_profile_and_can_select_a_profile() {
    let w = MemoryWire::new();
    w.0.borrow_mut().receiver = true;
    let mut b = Board::connect(w.clone(), false).unwrap();
    let s = b.snapshot().unwrap();
    let changes = air01::operations::plan_remap(
        &s,
        2,
        air01::device::Layer::Base,
        42,
        air01::protocol::Action([0, 0, 4, 0]),
    )
    .unwrap();
    air01::operations::apply(&mut b, &s, &changes, false).unwrap();
    assert!(!w.0.borrow().writes.is_empty());
    let s = b.snapshot().unwrap();
    let changes = air01::operations::plan_remap(
        &s,
        1,
        air01::device::Layer::Base,
        42,
        air01::protocol::Action([0, 0, 4, 0]),
    )
    .unwrap();
    let before = w.0.borrow().writes.len();
    assert!(air01::operations::apply(&mut b, &s, &changes, false).is_err());
    assert_eq!(w.0.borrow().writes.len(), before);
    air01::operations::select_profile(&mut b, 3).unwrap();
    assert_eq!(b.active_profile().unwrap(), 3);
}

#[test]
fn receiver_rollback_never_writes_a_profile_that_became_inactive() {
    let w = MemoryWire::new();
    w.0.borrow_mut().receiver = true;
    let mut b = Board::connect(w.clone(), false).unwrap();
    let s = b.snapshot().unwrap();
    let changes = air01::operations::plan_remap(
        &s,
        2,
        air01::device::Layer::Base,
        42,
        air01::protocol::Action([0, 0, 4, 0]),
    )
    .unwrap();
    w.0.borrow_mut().switch_after_write = Some((1, 0));
    let error = air01::operations::apply(&mut b, &s, &changes, false).unwrap_err();
    assert!(format!("{error:#}").contains("INCOMPLETE"));
    assert_eq!(w.0.borrow().writes.len(), 1);
}
