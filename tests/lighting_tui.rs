mod common;
use air01::{
    device::Board,
    lighting,
    tui::{Intent, UiState, draw},
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{backend::TestBackend, prelude::*};
fn press(s: &mut UiState, c: KeyCode) -> Intent {
    s.handle(KeyEvent::new(c, KeyModifiers::NONE))
}
fn state() -> UiState {
    let mut b = Board::connect(common::MemoryWire::new(), true).unwrap();
    let mut s = UiState::new(b.snapshot().unwrap());
    s.set_lighting(lighting::read(&mut b).unwrap(), lighting::presets());
    s
}
#[test]
fn preset_preview_and_controls_are_keyboard_accessible_without_writes() {
    let mut s = state();
    press(&mut s, KeyCode::Char('2'));
    let before = s.lighting.as_ref().unwrap().original.clone();
    press(&mut s, KeyCode::Down);
    press(&mut s, KeyCode::Enter);
    assert_ne!(s.lighting.as_ref().unwrap().draft, before.lighting);
    assert_eq!(s.lighting.as_ref().unwrap().original, before);
    assert_eq!(press(&mut s, KeyCode::Char('a')), Intent::ApplyLighting);
    press(&mut s, KeyCode::Char('1'));
    assert!(!s.is_lighting());
}
#[test]
fn mouse_controls_share_geometry_and_hex_is_validated() {
    let mut s = state();
    press(&mut s, KeyCode::Char('2'));
    let area = Rect::new(0, 0, 80, 24);
    let g = air01::tui::lighting::LightLayout::new(area);
    let r = g.sliders[0];
    s.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: r.right() - 1,
            row: r.y,
            modifiers: KeyModifiers::NONE,
        },
        area,
    );
    assert_eq!(s.lighting.as_ref().unwrap().draft.color()[0], 255);
    press(&mut s, KeyCode::Char('x'));
    press(&mut s, KeyCode::Char('u'));
    press(&mut s, KeyCode::Esc);
    assert_eq!(press(&mut s, KeyCode::Char('s')), Intent::None);
}
#[test]
fn lighting_workspace_fits_small_and_large_terminals() {
    for (w, h) in [(80, 24), (120, 34)] {
        let mut s = state();
        press(&mut s, KeyCode::Char('2'));
        let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
        t.draw(|f| draw(f, &s)).unwrap();
        let text = t
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        for label in [
            "LIGHTING",
            "Mint",
            "Apply",
            "Brightness",
            "Speed",
            "Save",
            "Undo",
        ] {
            assert!(text.contains(label), "missing {label} at {w}x{h}");
        }
    }
}
#[test]
fn lighting_preview_keeps_all_six_physical_rows() {
    let mut s = state();
    press(&mut s, KeyCode::Char('2'));
    let mut t = Terminal::new(TestBackend::new(120, 34)).unwrap();
    t.draw(|f| draw(f, &s)).unwrap();
    let g = air01::tui::lighting::LightLayout::new(Rect::new(0, 0, 120, 34));
    let rows = (g.preview.y + 1..g.preview.bottom() - 1)
        .filter(|y| {
            (g.preview.x + 1..g.preview.right() - 1)
                .any(|x| t.backend().buffer()[(x, *y)].bg != Color::Rgb(16, 23, 35))
        })
        .count();
    assert_eq!(rows, 6);
}
#[test]
fn save_editor_keeps_numeric_text_and_reports_invalid_hex() {
    let mut s = state();
    press(&mut s, KeyCode::Char('2'));
    press(&mut s, KeyCode::Char('x'));
    s.handle(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
    for c in "bad".chars() {
        press(&mut s, KeyCode::Char(c));
    }
    press(&mut s, KeyCode::Enter);
    assert!(s.status.contains("six hex digits"));
    assert!(s.lighting.as_ref().unwrap().editor.is_some());
    press(&mut s, KeyCode::Esc);
    press(&mut s, KeyCode::Char('s'));
    for c in "desk-12".chars() {
        press(&mut s, KeyCode::Char(c));
    }
    assert_eq!(
        press(&mut s, KeyCode::Enter),
        Intent::SaveLighting("desk-12".into())
    );
    assert!(s.is_lighting());
}
#[test]
fn error_notice_is_readable_and_closes_without_discarding_draft() {
    let mut s = state();
    press(&mut s, KeyCode::Char('2'));
    let draft = s.lighting.as_ref().unwrap().draft;
    s.show_error("A long recovery message with the complete backup path".into());
    let mut t = Terminal::new(TestBackend::new(80, 24)).unwrap();
    t.draw(|f| draw(f, &s)).unwrap();
    let text = t
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect::<String>();
    assert!(text.contains("complete backup path"));
    press(&mut s, KeyCode::Esc);
    assert!(matches!(s.mode, air01::tui::Mode::Browse));
    assert_eq!(s.lighting.as_ref().unwrap().draft, draft);
}
