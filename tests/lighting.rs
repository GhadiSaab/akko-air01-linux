mod common;
use air01::{
    device::Board,
    lighting::{self, Lighting, LightingSnapshot},
    transport::Wire,
};
use common::MemoryWire;
fn apply(
    b: &mut Board<MemoryWire>,
    before: &LightingSnapshot,
    next: Lighting,
) -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    lighting::apply_in(b, before, next, dir.path()).map(|_| ())
}
#[test]
fn lighting_packets_use_bit8_and_normalize_colors() {
    let l = Lighting::new(1, [255, 255, 255], 4, 2, false).unwrap();
    let p = l.packet();
    assert_eq!(&p[1..9], &[7, 1, 2, 4, 7, 250, 250, 250]);
    assert_eq!(p[1..10].iter().map(|x| *x as u16).sum::<u16>() & 255, 255);
    assert_eq!(l.color(), [255, 255, 255]);
    assert!(Lighting::new(13, [1, 2, 3], 4, 2, false).is_err());
    assert!(Lighting::new(1, [1, 2, 3], 5, 2, false).is_err());
    assert_eq!(lighting::parse_color("#90AAb0").unwrap(), [144, 170, 176]);
    assert!(lighting::parse_color("1234567").is_err());
}
#[test]
fn apply_verifies_without_touching_maps_and_rejects_stale_state() {
    let w = MemoryWire::new();
    let mut b = Board::connect(w.clone(), true).unwrap();
    let before = lighting::read(&mut b).unwrap();
    let maps = w.0.borrow().snapshot.clone();
    let next = Lighting::new(2, [34, 120, 240], 3, 1, false).unwrap();
    apply(&mut b, &before, next).unwrap();
    assert_eq!(lighting::read(&mut b).unwrap().lighting, next);
    assert_eq!(
        serde_json::to_string(&maps).unwrap(),
        serde_json::to_string(&w.0.borrow().snapshot).unwrap()
    );
    assert!(apply(&mut b, &before, next).is_err());
    assert_eq!(w.0.borrow().writes.len(), 1);
    let current = lighting::read(&mut b).unwrap();
    apply(&mut b, &current, next).unwrap();
    assert_eq!(w.0.borrow().writes.len(), 1);
}
#[test]
fn ignored_write_fails_and_original_is_verified() {
    let w = MemoryWire::new();
    let mut b = Board::connect(w.clone(), true).unwrap();
    let before = lighting::read(&mut b).unwrap();
    w.0.borrow_mut().ignore_at = Some(1);
    let e = apply(
        &mut b,
        &before,
        Lighting::new(1, [20, 30, 40], 4, 2, false).unwrap(),
    )
    .unwrap_err();
    assert!(format!("{e:#}").contains("original lighting verified"));
    assert_eq!(lighting::read(&mut b).unwrap(), before);
}
#[test]
fn lighting_files_roundtrip_and_refuse_invalid_or_overwrite() {
    let d = tempfile::tempdir().unwrap();
    let w = MemoryWire::new();
    let mut b = Board::connect(w, true).unwrap();
    let s = lighting::read(&mut b).unwrap();
    let p = d.path().join("theme.json");
    lighting::save(&s, &p).unwrap();
    assert_eq!(lighting::load(&p).unwrap(), s);
    assert!(lighting::save(&s, &p).is_err());
    let bad = LightingSnapshot { schema: 2, ..s };
    assert!(lighting::save(&bad, &d.path().join("bad.json")).is_err());
    assert!(lighting::theme_path(d.path(), "../bad").is_err());
    assert!(lighting::theme_path(d.path(), "   ").is_err());
}
#[test]
fn active_profile_changes_and_wireless_never_write() {
    let mut w = MemoryWire::new();
    let mut b = Board::connect(w.clone(), true).unwrap();
    let s = lighting::read(&mut b).unwrap();
    w.send(air01::protocol::packet(4, &[3], None).unwrap())
        .unwrap();
    assert!(
        apply(
            &mut b,
            &s,
            Lighting::new(1, [1, 2, 3], 4, 2, false).unwrap()
        )
        .is_err()
    );
    assert_eq!(w.0.borrow().writes.len(), 1);
    b.wired = false;
    let s = lighting::read(&mut b).unwrap();
    assert!(apply(&mut b, &s, s.lighting).is_err());
}
#[test]
fn broken_preset_does_not_hide_valid_presets() {
    let dir = tempfile::tempdir().unwrap();
    let w = MemoryWire::new();
    let mut b = Board::connect(w, true).unwrap();
    let s = lighting::read(&mut b).unwrap();
    lighting::save(&s, &dir.path().join("good.json")).unwrap();
    std::fs::write(dir.path().join("broken.json"), "bad").unwrap();
    let report = lighting::library_from(dir.path()).unwrap();
    assert!(report.presets.iter().any(|p| p.name == "good"));
    assert_eq!(report.warnings.len(), 1);
}
#[test]
fn custom_color_exits_spectrum_and_case_only_names_are_rejected() {
    let spectrum = Lighting::new(3, [1, 2, 3], 4, 2, false).unwrap();
    let fixed = spectrum.with_color([10, 20, 30]);
    assert_eq!(fixed.mode(), 1);
    assert!(!fixed.rainbow());
    assert_eq!(fixed.color(), [10, 20, 30]);
    let d = tempfile::tempdir().unwrap();
    let mut b = Board::connect(MemoryWire::new(), true).unwrap();
    let s = lighting::read(&mut b).unwrap();
    lighting::save_theme_in(d.path(), "Desk", &s).unwrap();
    assert!(lighting::save_theme_in(d.path(), "desk", &s).is_err());
    assert!(lighting::save_theme_in(d.path(), "mint", &s).is_err());
}
