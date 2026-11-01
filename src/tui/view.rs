use super::theme::{self, ACCENT as MINT, BG, BORDER, DIM, GOLD, KEY, PANEL, SELECTED, TEXT};
use super::{Mode, UiState, picker_items};
use crate::{device::Layer, keyboard, operations, protocol::Action};
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};
#[derive(Clone, Copy, Debug)]
pub enum UiButton {
    Backup,
    Restore,
    Swap,
    Quit,
}
pub struct UiLayout {
    pub keyboard: Rect,
    pub large: bool,
    pub profile_tabs: Vec<(u8, Rect)>,
    pub layer_tabs: Vec<(Layer, Rect)>,
    pub activate: Rect,
    pub buttons: Vec<(UiButton, Rect)>,
    board: Rect,
    detail: Rect,
}
impl UiLayout {
    pub fn new(area: Rect) -> Self {
        let area = theme::workspace(area);
        let large = false;
        let w = area.width.saturating_sub(4).min(88);
        let top = area.y + 4 + ((area.height.saturating_sub(28)) / 4).min(2);
        let board = Rect::new(area.x + (area.width - w) / 2, top, w, 15);
        let keyboard = board.inner(Margin::new(1, 1));
        let footer = Rect::new(area.x, area.bottom().saturating_sub(3), area.width, 3);
        let detail = Rect::new(
            board.x,
            board.bottom(),
            board.width,
            footer.y.saturating_sub(1).saturating_sub(board.bottom()),
        );
        let profile_tabs = (0..4)
            .map(|p| {
                (
                    p,
                    Rect::new(area.x + 9 + u16::from(p) * 6, area.y + 2, 5, 1),
                )
            })
            .collect();
        let layer_tabs = vec![
            (Layer::Base, Rect::new(area.x + 42, area.y + 2, 7, 1)),
            (Layer::FnWin, Rect::new(area.x + 49, area.y + 2, 8, 1)),
            (Layer::FnMac, Rect::new(area.x + 57, area.y + 2, 8, 1)),
        ];
        let activate = Rect::new(area.right().saturating_sub(13), area.y + 2, 12, 1);
        let buttons = vec![
            (UiButton::Backup, Rect::new(area.x + 2, footer.y, 10, 1)),
            (UiButton::Restore, Rect::new(area.x + 12, footer.y, 11, 1)),
            (UiButton::Swap, Rect::new(area.x + 25, footer.y, 14, 1)),
            (UiButton::Quit, Rect::new(area.x + 41, footer.y, 10, 1)),
        ];
        Self {
            keyboard,
            large,
            profile_tabs,
            layer_tabs,
            activate,
            buttons,
            board,
            detail,
        }
    }
}
fn button(f: &mut Frame, area: Rect, text: &str, selected: bool) {
    f.render_widget(
        Paragraph::new(text)
            .alignment(Alignment::Center)
            .style(if selected {
                Style::default().bg(SELECTED).fg(MINT).bold()
            } else {
                Style::default().bg(PANEL).fg(DIM)
            }),
        area,
    );
}
fn action_label(a: Action) -> String {
    if a == Action([10, 9, 0, 0]) {
        "Toggle lighting".into()
    } else {
        a.label()
    }
}
fn short(text: &str, width: u16) -> String {
    let text = match text {
        "LeftCtrl" | "RightCtrl" => "Ctrl",
        "LeftShift" | "RightShift" => "Shift",
        "LeftAlt" | "RightAlt" => "Alt",
        "LeftMeta" | "RightMeta" => "Win",
        "Disabled" => "—",
        "Toggle lighting" => "☼",
        "Light" => "☼",
        "Escape" => "Esc",
        "Backspace" => "Bksp",
        "CapsLock" => "Caps",
        "PageUp" => "PgUp",
        "PageDown" => "PgDn",
        "PrintScreen" => "Prt",
        "Insert" => "Ins",
        "Delete" => "Del",
        "LeftBracket" => "[",
        "RightBracket" => "]",
        "Backslash" => "\\",
        "Semicolon" => ";",
        "Apostrophe" => "'",
        "Grave" => "`",
        "Comma" => ",",
        "Period" => ".",
        "Slash" => "/",
        "Minus" => "-",
        "Equal" => "=",
        "Up" => "↑",
        "Down" => "↓",
        "Left" => "←",
        "Right" => "→",
        _ => text,
    };
    if width < 3 {
        match text {
            "Ctrl" => "Ct",
            "Shift" => "Sh",
            "Opt" => "Op",
            "Cmd" => "⌘",
            "Caps" => "Cp",
            "Light" => "☼",
            "Tab" => "Tb",
            "Esc" => "Es",
            "F10" => "10",
            "F11" => "11",
            "F12" => "12",
            "PgUp" => "PU",
            "PgDn" => "PD",
            "Home" => "Hm",
            "End" => "En",
            "Ins" => "In",
            "Prt" => "Pr",
            _ => text,
        }
        .chars()
        .take(width as usize)
        .collect()
    } else if text.starts_with("raw:") || text.starts_with("HID:") {
        "?".into()
    } else {
        text.chars().take(width as usize).collect()
    }
}
fn effective(s: &UiState, slot: u8) -> (Action, bool) {
    let Ok(p) = s.snapshot.profile(s.selected_profile) else {
        return (Action::DISABLED, false);
    };
    let raw = operations::entries(p, s.layer)[slot as usize];
    let inherited = s.layer != Layer::Base && raw == Action::DISABLED;
    (
        if inherited {
            p.base[slot as usize]
        } else {
            raw
        },
        inherited,
    )
}
pub fn draw(f: &mut Frame, s: &UiState) {
    let area = f.area();
    f.render_widget(Block::new().style(Style::default().bg(BG).fg(TEXT)), area);
    if area.width < 80 || area.height < 24 {
        f.render_widget(
            Paragraph::new("Resize terminal to at least 80×24.\nq quit")
                .style(Style::default().fg(GOLD))
                .wrap(Wrap { trim: false }),
            area,
        );
        return;
    }
    if s.is_lighting() {
        super::lighting::draw(f, s);
        return;
    }
    let area = theme::workspace(area);
    theme::shell(
        f,
        &format!(
            "active: {} · viewing: {} · USB",
            s.snapshot.active_profile, s.selected_profile
        ),
        "Arrows move · Enter edit · Tab layer",
    );
    workspace_nav(f, s);
    let g = UiLayout::new(area);
    f.render_widget(
        Paragraph::new(" PROFILE").style(Style::default().fg(DIM)),
        Rect::new(area.x + 1, area.y + 2, 8, 1),
    );
    for (p, r) in &g.profile_tabs {
        button(
            f,
            *r,
            &format!(
                " {p}{} ",
                if *p == s.snapshot.active_profile {
                    "*"
                } else {
                    " "
                }
            ),
            *p == s.selected_profile,
        );
    }
    f.render_widget(
        Paragraph::new("LAYER").style(Style::default().fg(DIM)),
        Rect::new(area.x + 35, area.y + 2, 6, 1),
    );
    for (layer, r) in &g.layer_tabs {
        button(
            f,
            *r,
            match layer {
                Layer::Base => "Base",
                Layer::FnWin => "Win Fn",
                Layer::FnMac => "Mac Fn",
            },
            *layer == s.layer,
        );
    }
    button(f, g.activate, "Activate [a]", false);
    f.render_widget(
        Paragraph::new("Keyboard · click a key").style(Style::default().fg(DIM)),
        Rect::new(g.board.x, g.board.y, g.board.width, 1),
    );
    for (slot, r) in keyboard::rectangles(g.keyboard, g.large) {
        let Some(k) = keyboard::key(slot) else {
            continue;
        };
        let (a, inherited) = effective(s, slot);
        let changed = if s.layer == Layer::Base {
            a.label() != k.default.label()
        } else {
            !inherited
        };
        let selected = slot == s.selected_slot;
        let bg = if selected { SELECTED } else { KEY };
        let inner = Rect::new(r.x, r.y + 1, r.width, 1);
        f.render_widget(Block::new().style(Style::default().bg(bg)), inner);
        let legend = short(k.legend, inner.width);
        if g.large {
            let assigned = if changed {
                format!(
                    "→{}",
                    short(&action_label(a), inner.width.saturating_sub(1))
                )
            } else if inherited {
                "·".into()
            } else {
                String::new()
            };
            let text = Text::from(vec![
                Line::from(Span::styled(
                    legend,
                    Style::default()
                        .fg(if selected {
                            MINT
                        } else {
                            super::theme::KEY_TEXT
                        })
                        .bold(),
                )),
                Line::from(Span::styled(
                    assigned,
                    Style::default().fg(if changed { GOLD } else { DIM }),
                )),
            ]);
            f.render_widget(Paragraph::new(text).alignment(Alignment::Center), inner);
        } else {
            let face = if changed || inherited {
                short(&action_label(a), inner.width)
            } else {
                legend
            };
            f.render_widget(
                Paragraph::new(face).alignment(Alignment::Center).style(
                    Style::default()
                        .fg(if selected {
                            MINT
                        } else if changed {
                            GOLD
                        } else {
                            super::theme::KEY_TEXT
                        })
                        .bold(),
                ),
                inner,
            );
        }
    }
    let (a, inherited) = effective(s, s.selected_slot);
    let name = keyboard::key(s.selected_slot).map_or("Key", |k| k.legend);
    let detail = if g.detail.height >= 5 {
        let block = Block::new()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(BORDER));
        let inner = block.inner(g.detail);
        f.render_widget(block, g.detail);
        inner
    } else {
        g.detail
    };
    let current = if inherited {
        format!("inherits {}", action_label(a))
    } else {
        action_label(a)
    };
    let text = vec![Line::from(vec![
        Span::styled(format!(" {name} "), Style::default().fg(MINT).bold()),
        Span::raw(format!("→ {current}    ")),
        Span::styled(
            format!(
                "profile {} · {} · slot {}",
                s.selected_profile, s.layer, s.selected_slot
            ),
            Style::default().fg(DIM),
        ),
    ])];
    f.render_widget(Paragraph::new(text).wrap(Wrap { trim: false }), detail);
    f.render_widget(
        Paragraph::new(s.status.as_str()).style(Style::default().fg(DIM)),
        Rect::new(area.x + 2, area.bottom() - 2, area.width - 4, 1),
    );
    for (b, r) in &g.buttons {
        button(
            f,
            *r,
            match b {
                UiButton::Backup => "b Backup",
                UiButton::Restore => "r Restore",
                UiButton::Swap => "s Ctrl/Fn swap",
                UiButton::Quit => "q quit",
            },
            false,
        );
    }
    match &s.mode {
        Mode::Browse => {}
        Mode::Help => draw_help(f),
        Mode::Notice(text) => draw_notice(f, text),
        Mode::Picker { query, selected } => draw_picker(f, s, query, *selected),
        Mode::Confirm(text) => {
            let p = confirm_popup(area);
            popup_frame(f, p, " Confirm keyboard changes ");
            let body = Rect::new(p.x + 2, p.y + 2, p.width - 4, p.height - 5);
            f.render_widget(
                Paragraph::new(text.as_str())
                    .wrap(Wrap { trim: true })
                    .style(Style::default().fg(TEXT)),
                body,
            );
            let (apply, cancel) = confirm_buttons(area);
            button(f, apply, "Apply [y]", true);
            button(f, cancel, "Cancel [n]", false);
        }
        Mode::Edit(value) | Mode::Restore(value) => {
            let p = confirm_popup(area);
            popup_frame(
                f,
                p,
                if matches!(s.mode, Mode::Edit(_)) {
                    " Advanced assignment "
                } else {
                    " Restore keymap backup "
                },
            );
            let message = if matches!(s.mode, Mode::Edit(_)) {
                "Key name, Fn, Disabled, or raw:00:00:e0:00"
            } else {
                "Absolute path to the snapshot JSON"
            };
            f.render_widget(
                Paragraph::new(format!(
                    "{message}\n\n{value}_\n\nEnter: preview changes   Ctrl+U: clear   Esc: cancel"
                ))
                .wrap(Wrap { trim: false }),
                Rect::new(p.x + 2, p.y + 2, p.width - 4, p.height - 3),
            );
        }
    }
}
fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let area = theme::workspace(area);
    let w = width.min(area.width.saturating_sub(4));
    let h = height.min(area.height.saturating_sub(2));
    Rect::new(
        area.x + (area.width - w) / 2,
        area.y + (area.height - h) / 2,
        w,
        h,
    )
}
pub(crate) fn picker_popup(area: Rect) -> Rect {
    centered(area, 64, 20)
}
pub(crate) fn picker_list(area: Rect) -> Rect {
    let p = picker_popup(area);
    Rect::new(
        p.x + 2,
        p.y + 6,
        p.width.saturating_sub(4),
        p.height.saturating_sub(9),
    )
}
pub(crate) fn picker_start(selected: usize, height: u16) -> usize {
    selected.saturating_sub(usize::from(height.saturating_sub(1)))
}
pub(crate) fn confirm_popup(area: Rect) -> Rect {
    centered(area, 72, 11)
}
pub(crate) fn confirm_buttons(area: Rect) -> (Rect, Rect) {
    let p = confirm_popup(area);
    (
        Rect::new(p.x + p.width - 28, p.bottom() - 3, 12, 1),
        Rect::new(p.x + p.width - 14, p.bottom() - 3, 12, 1),
    )
}
fn popup_frame(f: &mut Frame, area: Rect, title: &str) {
    f.render_widget(Clear, area);
    f.render_widget(
        Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(MINT))
            .style(Style::default().bg(PANEL).fg(TEXT))
            .title(title),
        area,
    );
}
fn draw_picker(f: &mut Frame, s: &UiState, query: &str, selected: usize) {
    let area = f.area();
    let p = picker_popup(area);
    let name = keyboard::key(s.selected_slot).map_or("key", |k| k.legend);
    popup_frame(f, p, &format!(" Assign {name} "));
    let (effective_action, inherited) = effective(s, s.selected_slot);
    let current_label = if inherited {
        format!("inherits {}", action_label(effective_action))
    } else {
        action_label(s.current_action())
    };
    f.render_widget(
        Paragraph::new(format!(
            "Profile {} · {} · currently {}",
            s.selected_profile, s.layer, current_label
        ))
        .style(Style::default().fg(DIM)),
        Rect::new(p.x + 2, p.y + 1, p.width - 4, 1),
    );
    f.render_widget(
        Paragraph::new(format!("Search: {query}_"))
            .style(Style::default().fg(MINT))
            .block(Block::bordered().border_style(Style::default().fg(BORDER))),
        Rect::new(p.x + 2, p.y + 3, p.width - 4, 3),
    );
    let list = picker_list(area);
    let items = picker_items(query, s.layer);
    let start = picker_start(selected, list.height);
    if items.is_empty() {
        f.render_widget(
            Paragraph::new("No assignments match. Ctrl+U clears the search.")
                .style(Style::default().fg(GOLD)),
            list,
        );
    }
    for (i, item) in items
        .iter()
        .enumerate()
        .skip(start)
        .take(list.height as usize)
    {
        let r = Rect::new(list.x, list.y + (i - start) as u16, list.width, 1);
        let label = format!(
            " {} {:32} {}",
            if i == selected { "›" } else { " " },
            item.label,
            item.group
        );
        f.render_widget(
            Paragraph::new(label).style(if i == selected {
                Style::default().bg(SELECTED).fg(MINT).bold()
            } else {
                Style::default().fg(TEXT)
            }),
            r,
        );
    }
    f.render_widget(
        Paragraph::new("Click / Enter choose   ↑↓ navigate   F2 raw   Esc cancel")
            .style(Style::default().fg(DIM)),
        Rect::new(p.x + 2, p.bottom() - 2, p.width - 4, 1),
    );
}

pub(super) fn workspace_nav(f: &mut Frame, s: &UiState) {
    let [keys, lighting, help] = theme::navigation(f.area());
    button(f, keys, "1  Keymap", !s.is_lighting());
    button(f, lighting, "2  Lighting", s.is_lighting());
    button(f, help, "? Help", false);
}
pub(super) fn draw_help(f: &mut Frame) {
    let a = theme::workspace(f.area());
    let p = Rect::new(a.x + (a.width - 74) / 2, a.y + (a.height - 21) / 2, 74, 21);
    popup_frame(f, p, " AIR01 · QUICK GUIDE ");
    let help = "WORKSPACES
  1 Keymap   2 Lighting   ? / F1 Help   Ctrl+C Quit

KEYMAP
  Click a key / arrows to select · Enter opens assignment search
  Type to filter · Enter selects · y Apply / Esc Cancel
  Profile tabs preview · a activates · Tab cycles Fn layers
  b Backup · r Restore · s Ctrl/Fn swap

LIGHTING
  Click a preset to preview · ↑↓ browse and Enter select
  Tab / Shift+Tab focus controls · ←→ adjust · Enter activate
  Click or drag RGB, brightness and speed bars
  x Hex color · Ctrl+U clear · s Save named preset
  m Text animation stored on the keyboard (e.g. CLAUDE)
  a Apply to active profile · u Undo last change
  r Revert draft · Reload reads the keyboard

Drafts stay on screen until Apply. Press any key or click to close.";
    f.render_widget(
        Paragraph::new(help)
            .style(Style::default().fg(TEXT))
            .wrap(Wrap { trim: false }),
        Rect::new(p.x + 2, p.y + 1, p.width - 4, p.height - 2),
    );
}

pub(super) fn draw_notice(f: &mut Frame, text: &str) {
    let a = theme::workspace(f.area());
    let p = Rect::new(a.x + 4, a.y + (a.height - 14) / 2, a.width - 8, 14);
    popup_frame(f, p, " AIR01 · NOTICE ");
    f.render_widget(
        Paragraph::new(text)
            .style(Style::default().fg(GOLD))
            .wrap(Wrap { trim: false }),
        Rect::new(p.x + 2, p.y + 2, p.width - 4, p.height - 5),
    );
    button(
        f,
        Rect::new(p.x + 2, p.bottom() - 2, p.width - 4, 1),
        "Enter / Esc / click to close",
        true,
    );
}

pub(super) fn draw_loading(f: &mut Frame, step: u16, message: &str) {
    let a = f.area();
    f.render_widget(Block::new().style(Style::default().bg(BG).fg(TEXT)), a);
    let w = a.width.min(62);
    let h = a.height.min(9);
    let r = Rect::new(a.x + (a.width - w) / 2, a.y + (a.height - h) / 2, w, h);
    f.render_widget(
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(" air01 / CONNECTING ")
            .border_style(Style::default().fg(MINT)),
        r,
    );
    let inner = r.inner(Margin::new(2, 1));
    if inner.height > 0 {
        f.render_widget(
            Paragraph::new(message).style(Style::default().fg(TEXT)),
            Rect::new(inner.x, inner.y, inner.width, 1),
        );
    }
    if inner.height > 2 {
        f.render_widget(
            ratatui::widgets::Gauge::default()
                .ratio(f64::from(step.min(13)) / 13.0)
                .gauge_style(Style::default().fg(MINT).bg(PANEL))
                .label(format!("{}/13", step.min(13))),
            Rect::new(inner.x, inner.y + 2, inner.width, 1),
        );
    }
    if inner.height > 4 {
        f.render_widget(
            Paragraph::new("Reading onboard settings · no changes sent")
                .style(Style::default().fg(DIM)),
            Rect::new(inner.x, inner.y + 4, inner.width, 1),
        );
    }
}
