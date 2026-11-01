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
            "Lighting",
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
                .any(|x| t.backend().buffer()[(x, *y)].bg != air01::tui::theme::PANEL)
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
#[test]
fn lighting_read_failure_still_has_mouse_reload_and_help() {
    let mut s = state();
    s.lighting = None;
    press(&mut s, KeyCode::Char('2'));
    let area = Rect::new(0, 0, 80, 24);
    let g = air01::tui::lighting::LightLayout::new(area);
    let (_, r) = g.controls.iter().find(|(i, _)| *i == 13).unwrap();
    let intent = s.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: r.x,
            row: r.y,
            modifiers: KeyModifiers::NONE,
        },
        area,
    );
    assert_eq!(intent, Intent::ReloadLighting);
    press(&mut s, KeyCode::F(1));
    let mut t = Terminal::new(TestBackend::new(80, 24)).unwrap();
    t.draw(|f| draw(f, &s)).unwrap();
    let text = t
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect::<String>();
    assert!(text.contains("QUICK GUIDE"));
}
#[test]
fn spectrum_rainbow_toggle_really_disables_rainbow() {
    let mut s = state();
    let l = s.lighting.as_mut().unwrap();
    l.draft = lighting::Lighting::new(3, [1, 2, 3], 4, 2, false).unwrap();
    l.focus = 7;
    press(&mut s, KeyCode::Char('2'));
    press(&mut s, KeyCode::Enter);
    assert!(!s.lighting.as_ref().unwrap().draft.rainbow());
    assert_eq!(s.lighting.as_ref().unwrap().draft.mode(), 1);
}
#[test]
fn failed_refresh_preserves_incomplete_recovery_and_backup_path() {
    let mut s = state();
    let before = s.lighting.as_ref().unwrap().original.clone();
    s.finish_lighting_apply(
        before,
        Err(anyhow::anyhow!(
            "recovery INCOMPLETE. Backup: /tmp/saved-lighting.json"
        )),
        Err(anyhow::anyhow!("device disconnected")),
    );
    assert!(s.status.contains("INCOMPLETE"));
    assert!(s.status.contains("/tmp/saved-lighting.json"));
    assert!(s.status.contains("disconnected"));
    assert!(matches!(s.mode, air01::tui::Mode::Notice(_)));
}
#[test]
fn edits_select_custom_and_custom_draft_survives_preset_switches_and_refresh() {
    let mut s = state();
    let l = s.lighting.as_mut().unwrap();
    assert_eq!(l.presets[0].name, "Custom");
    l.set_color([12, 34, 56]);
    assert_eq!(l.active_name(), "Custom");
    assert_eq!(l.selected, 0);
    let custom = l.draft;
    l.selected = 1;
    l.focus = 0;
    press(&mut s, KeyCode::Char('2'));
    press(&mut s, KeyCode::Enter);
    assert_eq!(s.lighting.as_ref().unwrap().active_name(), "Mint");
    {
        let l = s.lighting.as_mut().unwrap();
        l.replace_presets(lighting::presets());
        l.selected = 0;
    }
    press(&mut s, KeyCode::Enter);
    assert_eq!(s.lighting.as_ref().unwrap().draft, custom);
    // Brightness editing is also Custom, rather than keeping the old preset name.
    s.lighting.as_mut().unwrap().focus = 5;
    press(&mut s, KeyCode::Left);
    assert_eq!(s.lighting.as_ref().unwrap().active_name(), "Custom");
}
#[test]
fn large_layout_centers_a_bounded_keyboard_and_keeps_controls_inside() {
    for area in [
        Rect::new(0, 0, 160, 50),
        Rect::new(7, 3, 180, 56),
        Rect::new(0, 0, 80, 24),
    ] {
        let g = air01::tui::lighting::LightLayout::new(area);
        assert!(g.preview.width <= 90);
        assert_eq!(
            (g.preview.x - area.x) * 2 + g.preview.width,
            area.width,
            "preview not centered"
        );
        for r in g
            .sliders
            .iter()
            .chain(g.swatches.iter())
            .chain(g.controls.iter().map(|(_, r)| r))
        {
            assert!(
                r.x >= area.x
                    && r.y >= area.y
                    && r.right() <= area.right()
                    && r.bottom() <= area.bottom()
            );
        }
        assert!(
            g.presets.y >= g.preview.bottom(),
            "presets should sit below the keyboard"
        );
    }
}
#[test]
fn centered_navigation_hit_targets_match_visible_tabs() {
    let mut s = state();
    let area = Rect::new(0, 0, 160, 50);
    let [_, tab, _] = air01::tui::theme::navigation(area);
    s.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: tab.x,
            row: tab.y,
            modifiers: KeyModifiers::NONE,
        },
        area,
    );
    assert!(s.is_lighting());
    let g = air01::tui::UiLayout::new(area);
    assert!(g.keyboard.width < 100);
    assert_eq!(g.keyboard.x * 2 + g.keyboard.width, area.width);
}
#[test]
fn an_unchanged_lighting_draft_does_not_offer_a_write_or_undo() {
    let mut s = state();
    press(&mut s, KeyCode::Char('2'));
    assert_eq!(press(&mut s, KeyCode::Char('a')), Intent::None);
    assert_eq!(press(&mut s, KeyCode::Char('u')), Intent::None);
}
#[test]
fn multicolor_preview_does_not_invent_per_key_colors() {
    let rainbow = lighting::Lighting::new(4, [255, 0, 0], 4, 2, true).unwrap();
    assert_eq!(air01::tui::lighting::preview_color(rainbow), None);
    let pink = lighting::Lighting::new(1, [255, 105, 180], 4, 2, false).unwrap();
    assert_eq!(
        air01::tui::lighting::preview_color(pink),
        Some([255, 105, 180])
    );
}
#[test]
fn focus_follows_visual_order_and_skips_unavailable_actions() {
    let mut s = state();
    press(&mut s, KeyCode::Char('2'));
    press(&mut s, KeyCode::Tab);
    assert_eq!(s.lighting.as_ref().unwrap().focus, 1);
    press(&mut s, KeyCode::Tab);
    assert_eq!(s.lighting.as_ref().unwrap().focus, 7);
    for focus in [2, 3, 4, 5, 8, 14, 10, 13, 0] {
        press(&mut s, KeyCode::Tab);
        assert_eq!(s.lighting.as_ref().unwrap().focus, focus);
    }
}
#[test]
fn static_speed_is_disabled_for_both_mouse_and_keyboard() {
    let mut s = state();
    press(&mut s, KeyCode::Char('2'));
    let before = s.lighting.as_ref().unwrap().draft;
    s.lighting.as_mut().unwrap().focus = 6;
    press(&mut s, KeyCode::Right);
    assert_eq!(s.lighting.as_ref().unwrap().draft, before);
    let r = air01::tui::lighting::LightLayout::new(Rect::new(0, 0, 80, 24)).sliders[4];
    s.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: r.right() - 1,
            row: r.y,
            modifiers: KeyModifiers::NONE,
        },
        Rect::new(0, 0, 80, 24),
    );
    assert_eq!(s.lighting.as_ref().unwrap().draft, before);
}
#[test]
fn bright_preview_keycaps_keep_dark_readable_legends() {
    let mut s = state();
    press(&mut s, KeyCode::Char('2'));
    s.lighting.as_mut().unwrap().set_color([255, 105, 180]);
    let mut t = Terminal::new(TestBackend::new(120, 34)).unwrap();
    t.draw(|f| draw(f, &s)).unwrap();
    let r = air01::tui::lighting::LightLayout::new(Rect::new(0, 0, 120, 34)).preview;
    let cell = &t.backend().buffer()[(r.x + 1, r.y + 1)];
    assert!(matches!(cell.fg,Color::Rgb(red,green,blue) if red<60 && green<60 && blue<60));
}
#[test]
fn common_footer_separator_is_unbroken_in_both_workspaces() {
    for (w, h) in [(80, 24), (120, 34), (161, 51)] {
        for lighting in [false, true] {
            let mut s = state();
            s.lighting_workspace = lighting;
            let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
            t.draw(|f| draw(f, &s)).unwrap();
            let a = air01::tui::theme::workspace(Rect::new(0, 0, w, h));
            for x in a.x + 1..a.right() - 1 {
                assert_eq!(
                    t.backend().buffer()[(x, a.bottom() - 4)].symbol(),
                    "─",
                    "footer overwritten at{x}, size{w}x{h}, lighting={lighting}"
                );
            }
        }
    }
}
#[test]
fn clicking_disabled_action_keeps_the_existing_focus() {
    let mut s = state();
    press(&mut s, KeyCode::Char('2'));
    let g = air01::tui::lighting::LightLayout::new(Rect::new(0, 0, 80, 24));
    let (_, r) = g.controls.iter().find(|(f, _)| *f == 11).unwrap();
    s.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: r.x,
            row: r.y,
            modifiers: KeyModifiers::NONE,
        },
        Rect::new(0, 0, 80, 24),
    );
    assert_eq!(s.lighting.as_ref().unwrap().focus, 0);
}

fn render(s: &UiState, w: u16, h: u16) -> String {
    let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
    t.draw(|f| draw(f, s)).unwrap();
    let buf = t.backend().buffer().clone();
    (0..h)
        .map(|y| {
            (0..w)
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect::<String>()
                + "\n"
        })
        .collect()
}

#[test]
fn animation_dialog_configures_and_requests_upload() {
    use air01::animation::{Direction, Style as AnimStyle};
    let mut s = state();
    press(&mut s, KeyCode::Char('2'));
    for (w, h) in [(80, 24), (120, 34)] {
        let studio = render(&s, w, h);
        assert!(
            studio.contains("x Color…") && studio.contains("m Animate…"),
            "{studio}"
        );
        if std::env::var_os("SHOW_TUI").is_some() {
            println!("{studio}");
        }
    }
    assert_eq!(press(&mut s, KeyCode::Char('m')), Intent::None);
    for (w, h) in [(80, 24), (120, 34)] {
        let text = render(&s, w, h);
        assert!(text.contains("TEXT ANIMATION"), "{text}");
        assert!(text.contains("CLAUDE"), "{text}");
        assert!(text.contains("198 frames"), "{text}");
        if std::env::var_os("SHOW_TUI").is_some() {
            println!("{text}");
        }
    }
    // Typing edits the text, even letters that are shortcuts elsewhere.
    for _ in 0..6 {
        press(&mut s, KeyCode::Backspace);
    }
    for c in "hi am".chars() {
        press(&mut s, KeyCode::Char(c));
    }
    assert!(s.is_lighting());
    press(&mut s, KeyCode::Down);
    press(&mut s, KeyCode::Right); // style → scroll
    press(&mut s, KeyCode::Down);
    press(&mut s, KeyCode::Right); // direction → left
    press(&mut s, KeyCode::Down);
    press(&mut s, KeyCode::Right); // 70 ms
    press(&mut s, KeyCode::Down); // pause disabled in scroll: wraps to text
    assert_eq!(s.lighting.as_ref().unwrap().animation.field, 0);
    let color = s.lighting.as_ref().unwrap().draft.color();
    match press(&mut s, KeyCode::Enter) {
        Intent::UploadAnimation(text, m) => {
            assert_eq!(text, "HI AM");
            assert_eq!(m.style, AnimStyle::Scroll);
            assert_eq!(m.direction, Direction::Left);
            assert_eq!(m.delay_ms, 70);
            assert_eq!(m.color, color);
        }
        other => panic!("{other:?}"),
    }
    // Invalid text keeps the dialog open with an explanation.
    press(&mut s, KeyCode::Char('?'));
    assert_eq!(press(&mut s, KeyCode::Enter), Intent::None);
    assert!(render(&s, 80, 24).contains("unsupported marquee character"));
    press(&mut s, KeyCode::Esc);
    assert!(s.lighting.as_ref().unwrap().editor.is_none());
    // Settings are remembered for the next opening.
    press(&mut s, KeyCode::Char('m'));
    assert_eq!(s.lighting.as_ref().unwrap().animation.text, "HI AM?");
}

#[test]
fn animation_dialog_mouse_targets_work() {
    let mut s = state();
    press(&mut s, KeyCode::Char('2'));
    let area = Rect::new(0, 0, 80, 24);
    let g = air01::tui::lighting::LightLayout::new(area);
    let (_, r) = *g.controls.iter().find(|(i, _)| *i == 14).unwrap();
    let click = |s: &mut UiState, x: u16, y: u16| {
        s.handle_mouse(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: x,
                row: y,
                modifiers: KeyModifiers::NONE,
            },
            area,
        )
    };
    click(&mut s, r.x, r.y);
    assert!(render(&s, 80, 24).contains("TEXT ANIMATION"));
    let a = air01::tui::lighting::AnimationLayout::new(area);
    let style = a.fields[1];
    click(&mut s, style.right() - 1, style.y);
    assert_eq!(
        s.lighting.as_ref().unwrap().animation.style,
        air01::animation::Style::Scroll
    );
    assert!(matches!(
        click(&mut s, a.upload.x, a.upload.y),
        Intent::UploadAnimation(..)
    ));
    click(&mut s, a.cancel.x, a.cancel.y);
    assert!(s.lighting.as_ref().unwrap().editor.is_none());
}
