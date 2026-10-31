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
    assert_eq!(s.handle(key(KeyCode::PageDown)), Intent::None);
    assert_eq!(s.selected_profile, 3);
    assert_eq!(s.snapshot.active_profile, 2);
    s.selected_slot = 5;
    s.handle(key(KeyCode::Right));
    assert_eq!(s.selected_slot, 11);
    s.handle(key(KeyCode::Left));
    assert_eq!(s.selected_slot, 5);
    s.handle(key(KeyCode::Tab));
    assert_eq!(s.layer, Layer::FnWin);
}
#[test]
fn edits_and_writes_require_explicit_confirmation() {
    let mut s = state();
    s.handle(key(KeyCode::Char('e')));
    assert!(matches!(s.mode, Mode::Picker { .. }));
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
    s.handle(key(KeyCode::PageDown));
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
#[test]
fn clicking_key_opens_picker_and_selection_only_returns_a_plan_intent() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    use ratatui::layout::Rect;
    let area = Rect::new(0, 0, 120, 34);
    let mut s = state();
    let layout = air01::tui::UiLayout::new(area);
    let (_, r) = air01::keyboard::rectangles(layout.keyboard, layout.large)
        .into_iter()
        .find(|(slot, _)| *slot == 5)
        .unwrap();
    let intent = s.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: r.x + r.width / 2,
            row: r.y + 1,
            modifiers: KeyModifiers::NONE,
        },
        area,
    );
    assert_eq!(intent, Intent::None);
    assert_eq!(s.selected_slot, 5);
    assert!(matches!(s.mode, Mode::Picker { .. }));
    for c in "LeftCtrl".chars() {
        s.handle(key(KeyCode::Char(c)));
    }
    assert_eq!(
        s.handle(key(KeyCode::Enter)),
        Intent::Remap("raw:00:00:e0:00".into())
    );
}
#[test]
fn clickable_profile_and_layer_tabs_only_preview() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    use ratatui::layout::Rect;
    let area = Rect::new(0, 0, 120, 34);
    let mut s = state();
    let layout = air01::tui::UiLayout::new(area);
    let (_, r) = layout.profile_tabs.iter().find(|(p, _)| *p == 0).unwrap();
    assert_eq!(
        s.handle_mouse(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: r.x,
                row: r.y,
                modifiers: KeyModifiers::NONE
            },
            area
        ),
        Intent::None
    );
    assert_eq!(s.selected_profile, 0);
    assert_eq!(s.snapshot.active_profile, 2);
    let (_, r) = layout
        .layer_tabs
        .iter()
        .find(|(p, _)| *p == Layer::FnMac)
        .unwrap();
    s.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: r.x,
            row: r.y,
            modifiers: KeyModifiers::NONE,
        },
        area,
    );
    assert_eq!(s.layer, Layer::FnMac);
}
#[test]
fn visual_keyboard_fits_compact_and_large_terminals_and_tiny_sizes_do_not_panic() {
    for (w, h) in [(80, 24), (120, 34), (160, 45), (20, 8), (1, 1)] {
        let mut s = state();
        s.snapshot.profiles[2].base[41] = air01::protocol::Action([0, 0, 44, 0]);
        let mut t = ratatui::Terminal::new(ratatui::backend::TestBackend::new(w, h)).unwrap();
        t.draw(|f| air01::tui::draw(f, &s)).unwrap();
        let text = t
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        if w >= 80 && h >= 24 {
            assert!(text.contains("Space"));
            assert!(text.contains("q quit"));
            assert!(!text.contains("Matrix slots /"));
        } else if w >= 20 {
            assert!(text.contains("Resize"));
        }
    }
}
#[test]
fn choosing_an_existing_ctrl_assignment_preserves_its_raw_encoding() {
    let mut s = state();
    s.selected_slot = 5;
    s.snapshot.profiles[2].base[5] = air01::protocol::Action([0, 224, 0, 0]);
    s.open_picker();
    assert_eq!(
        s.handle(key(KeyCode::Enter)),
        Intent::Remap("raw:00:e0:00:00".into())
    );
    s.open_picker();
    let mouse = crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
        column: 32,
        row: 16,
        modifiers: KeyModifiers::NONE,
    };
    assert_eq!(
        s.handle_mouse(mouse, ratatui::layout::Rect::new(0, 0, 120, 34)),
        Intent::Remap("raw:00:e0:00:00".into())
    );
}
#[test]
fn picker_and_confirmation_mouse_actions_use_their_visible_rows() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    use ratatui::layout::Rect;
    let area = Rect::new(0, 0, 120, 34);
    let click = |column, row| MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column,
        row,
        modifiers: KeyModifiers::NONE,
    };
    let mut s = state();
    s.open_picker();
    for c in "RightCtrl".chars() {
        s.handle(key(KeyCode::Char(c)));
    }
    // Picker is centered at (28,7), results start at (30,13).
    assert_eq!(
        s.handle_mouse(click(32, 13), area),
        Intent::Remap("raw:00:00:e4:00".into())
    );
    s.confirm("Change one key".into());
    assert_eq!(s.handle_mouse(click(70, 19), area), Intent::Confirm(true));
    s.confirm("Change one key".into());
    assert_eq!(s.handle_mouse(click(84, 19), area), Intent::Confirm(false));
}
#[test]
fn fn_layer_picker_offers_inheritance_instead_of_claiming_to_disable() {
    let mut s = state();
    s.layer = Layer::FnWin;
    s.open_picker();
    for c in "inherit".chars() {
        s.handle(key(KeyCode::Char(c)));
    }
    assert_eq!(
        s.handle(key(KeyCode::Enter)),
        Intent::Remap("raw:00:00:00:00".into())
    );
}
#[test]
fn compact_fn_keycap_shows_effective_remapped_base_assignment() {
    use ratatui::layout::Rect;
    let mut s = state();
    s.layer = Layer::FnWin;
    s.snapshot.profiles[2].base[5] = air01::protocol::Action::CTRL;
    let area = Rect::new(0, 0, 80, 24);
    let layout = air01::tui::UiLayout::new(area);
    let (_, r) = air01::keyboard::rectangles(layout.keyboard, false)
        .into_iter()
        .find(|(slot, _)| *slot == 5)
        .unwrap();
    let mut t = ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 24)).unwrap();
    t.draw(|f| air01::tui::draw(f, &s)).unwrap();
    let face = (r.x + 1..r.right() - 1)
        .map(|x| t.backend().buffer()[(x, r.y + 1)].symbol())
        .collect::<String>();
    assert_eq!(face.trim(), "Ct");
}
