use std::process::Command;
fn cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_air01"))
        .args(args)
        .output()
        .unwrap()
}
#[test]
fn help_and_version_work_without_a_keyboard() {
    let o = cli(&["--help"]);
    assert!(o.status.success());
    let s = String::from_utf8(o.stdout).unwrap();
    assert!(s.contains("swap-fn-ctrl") && s.contains("restore"));
    let o = cli(&["--version"]);
    assert!(o.status.success());
    assert!(String::from_utf8(o.stdout).unwrap().contains("0.1.0"));
}
#[test]
fn invalid_profile_and_slot_are_rejected_by_parser() {
    assert_eq!(cli(&["--profile", "4", "keymap"]).status.code(), Some(2));
    assert_eq!(cli(&["remap", "128", "A"]).status.code(), Some(2));
}
#[test]
fn malformed_snapshot_is_rejected_without_touching_hardware() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("foreign.json");
    std::fs::write(
        &p,
        r#"{"schema":1,"device_id":99,"active_profile":0,"profiles":[]}"#,
    )
    .unwrap();
    let o = cli(&["restore", p.to_str().unwrap()]);
    assert!(!o.status.success());
    assert!(String::from_utf8(o.stderr).unwrap().contains("snapshot"));
}
#[test]
fn dry_run_cannot_enter_a_writable_tui() {
    let o = cli(&["--device", "/nonexistent", "--dry-run", "tui"]);
    assert!(!o.status.success());
    assert!(
        String::from_utf8(o.stderr)
            .unwrap()
            .contains("dry-run is not supported with tui")
    );
}
#[test]
fn dry_run_backup_creates_no_file_and_needs_no_hardware() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("backup.json");
    let o = cli(&[
        "--device",
        "/nonexistent",
        "--dry-run",
        "backup",
        p.to_str().unwrap(),
    ]);
    assert!(o.status.success());
    assert!(!p.exists());
}
