mod common;
use air01::{
    device::{Board, Layer},
    tui::{Intent, Mode, UiState},
};
use common::MemoryWire;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
fn key(c: KeyCode) -> KeyEvent {
    KeyEvent::new(c, KeyModifiers::NONE)
}
fn state() -> UiState {
    let mut b = Board::connect(MemoryWire::new(), true).unwrap();
    UiState::new(b.snapshot().unwrap())
}
#[test]
fn browsing_profiles_is_not_activation_and_wraps_slots() {
    let mut s = state();
    assert_eq!(s.selected_profile, 2);
    assert_eq!(s.handle(key(KeyCode::Right)), Intent::None);
    assert_eq!(s.selected_profile, 3);
    assert_eq!(s.snapshot.active_profile, 2);
    s.handle(key(KeyCode::Up));
    assert_eq!(s.selected_slot, 127);
    s.handle(key(KeyCode::Down));
    assert_eq!(s.selected_slot, 0);
    s.handle(key(KeyCode::Tab));
    assert_eq!(s.layer, Layer::FnWin);
}
#[test]
fn edits_and_writes_require_explicit_confirmation() {
    let mut s = state();
    s.handle(key(KeyCode::Char('e')));
    assert!(matches!(s.mode, Mode::Edit(_)));
    s.handle(key(KeyCode::Esc));
    assert!(matches!(s.mode, Mode::Browse));
    s.confirm("two keys".into());
    assert_eq!(s.handle(key(KeyCode::Char('n'))), Intent::Confirm(false));
    s.confirm("two keys".into());
    assert_eq!(s.handle(key(KeyCode::Char('y'))), Intent::Confirm(true));
    assert_eq!(s.handle(key(KeyCode::Char('s'))), Intent::Swap);
}
#[test]
fn terminal_view_marks_active_profile_separately_from_preview() {
    let mut s = state();
    s.handle(key(KeyCode::Right));
    let backend = ratatui::backend::TestBackend::new(100, 28);
    let mut t = ratatui::Terminal::new(backend).unwrap();
    t.draw(|f| air01::tui::draw(f, &s)).unwrap();
    let text = t
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect::<String>();
    assert!(text.contains("active: 2"));
    assert!(text.contains("viewing: 3"));
    assert!(text.contains("Fn"));
    assert!(text.contains("q quit"));
}
