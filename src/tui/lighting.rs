//! Visual lighting workspace. Layout rectangles are also its input hit targets.
use super::theme::{self, ACCENT as MINT, BG, BORDER, DIM, GOLD, KEY, PANEL, SELECTED, TEXT};
use super::{Intent, UiState};
use crate::{
    keyboard,
    lighting::{self, EFFECTS, Lighting, LightingSnapshot, Preset},
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Clear, Paragraph},
};
pub const SWATCHES: [[u8; 3]; 8] = [
    [94, 234, 212],
    [56, 130, 255],
    [166, 100, 255],
    [255, 85, 150],
    [255, 102, 40],
    [255, 190, 110],
    [255, 255, 255],
    [0, 0, 0],
];
#[derive(Clone, Debug)]
pub enum Editor {
    Hex(String),
    Name(String),
}
pub struct LightingUi {
    pub original: LightingSnapshot,
    pub draft: Lighting,
    pub presets: Vec<Preset>,
    pub selected: usize,
    active_preset: Option<String>,
    pub focus: usize,
    pub editor: Option<Editor>,
    pub undo: Option<LightingSnapshot>,
}
impl LightingUi {
    pub fn new(original: LightingSnapshot, mut presets: Vec<Preset>) -> Self {
        let active_preset = presets
            .iter()
            .find(|p| p.lighting == original.lighting)
            .map(|p| p.name.clone());
        presets.insert(
            0,
            Preset {
                name: "Custom".into(),
                lighting: original.lighting,
                custom: false,
            },
        );
        let selected = active_preset
            .as_ref()
            .and_then(|name| presets.iter().position(|p| &p.name == name))
            .unwrap_or(0);
        Self {
            draft: original.lighting,
            original,
            presets,
            selected,
            active_preset,
            focus: 0,
            editor: None,
            undo: None,
        }
    }
    pub fn active_name(&self) -> &str {
        self.active_preset.as_deref().unwrap_or("Custom")
    }
    pub fn replace_presets(&mut self, mut presets: Vec<Preset>) {
        let custom = self.presets.first().map_or(self.draft, |p| p.lighting);
        presets.insert(
            0,
            Preset {
                name: "Custom".into(),
                lighting: custom,
                custom: false,
            },
        );
        self.presets = presets;
        self.selected = self.selected.min(self.presets.len() - 1);
    }
    fn mark_custom(&mut self) {
        self.active_preset = None;
        self.selected = 0;
        self.presets[0].lighting = self.draft;
    }
    pub fn restore_draft(&mut self, lighting: Lighting) {
        self.draft = lighting;
        let matched = self
            .presets
            .iter()
            .enumerate()
            .skip(1)
            .find(|(_, p)| p.lighting == lighting);
        self.selected = matched.map_or(0, |(i, _)| i);
        self.active_preset = matched.map(|(_, p)| p.name.clone());
        if self.selected == 0 {
            self.presets[0].lighting = lighting;
        }
    }
    pub fn choose_preset(&mut self, index: usize) {
        if let Some(p) = self.presets.get(index) {
            self.draft = p.lighting;
            self.active_preset = if index == 0 {
                None
            } else {
                Some(p.name.clone())
            };
            self.selected = index;
        }
    }
    pub fn dirty(&self) -> bool {
        self.draft != self.original.lighting
    }
    pub fn set_color(&mut self, c: [u8; 3]) {
        self.draft = self.draft.with_color(c);
        self.mark_custom();
    }
    fn adjust(&mut self, delta: i16) {
        match self.focus {
            0 => {
                if !self.presets.is_empty() {
                    self.selected = (self.selected as i16 + delta)
                        .rem_euclid(self.presets.len() as i16)
                        as usize;
                }
            }
            1 => {
                let i = EFFECTS
                    .iter()
                    .position(|e| e.0 == self.draft.mode())
                    .unwrap_or(0);
                let mode = EFFECTS[(i as i16 + delta).rem_euclid(EFFECTS.len() as i16) as usize].0;
                self.draft.params[0] = mode;
                self.draft.params[3] =
                    (if mode == 4 { 0x20 } else { 0 }) | (self.draft.params[3] & 15);
            }
            2..=4 => {
                let mut c = self.draft.color();
                let i = self.focus - 2;
                c[i] = (i16::from(c[i]) + delta * 5).clamp(0, 255) as u8;
                self.set_color(c);
            }
            5 => {
                self.draft.params[2] =
                    (i16::from(self.draft.brightness()) + delta).clamp(0, 4) as u8
            }
            6 => {
                self.draft.params[1] = 4 - (i16::from(self.draft.speed()) + delta).clamp(0, 4) as u8
            }
            7 => {
                self.draft = self.draft.with_rainbow(!self.draft.rainbow());
            }
            _ => {}
        }
        if (1..=7).contains(&self.focus) {
            self.mark_custom();
        }
    }
    fn activate(&mut self) -> Intent {
        match self.focus {
            0 => {
                self.choose_preset(self.selected);
            }
            1..=7 => self.adjust(1),
            8 => self.editor = Some(Editor::Hex(self.draft.hex())),
            9 if self.dirty() => return Intent::ApplyLighting,
            10 => self.editor = Some(Editor::Name(String::new())),
            11 if self.undo.is_some() => return Intent::UndoLighting,
            12 if self.dirty() => self.restore_draft(self.original.lighting),
            13 if !self.dirty() => return Intent::ReloadLighting,
            _ => {}
        }
        Intent::None
    }
    pub fn handle(&mut self, key: KeyEvent, status: &mut String) -> Intent {
        if let Some(mut editor) = self.editor.take() {
            let text = match &mut editor {
                Editor::Hex(s) | Editor::Name(s) => s,
            };
            match key.code {
                KeyCode::Esc => return Intent::None,
                KeyCode::Enter => match editor {
                    Editor::Hex(ref s) => match lighting::parse_color(s) {
                        Ok(c) => {
                            self.set_color(c);
                            return Intent::None;
                        }
                        Err(e) => *status = e.to_string(),
                    },
                    Editor::Name(ref s) => {
                        let name = s.clone();
                        self.editor = Some(editor);
                        return Intent::SaveLighting(name);
                    }
                },
                KeyCode::Backspace => {
                    text.pop();
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => text.clear(),
                KeyCode::Char(c)
                    if !key.modifiers.contains(KeyModifiers::CONTROL) && text.len() < 32 =>
                {
                    text.push(c)
                }
                _ => {}
            }
            self.editor = Some(editor);
            return Intent::None;
        }
        match key.code {
            KeyCode::Tab => self.focus = (self.focus + 1) % 14,
            KeyCode::BackTab => self.focus = (self.focus + 13) % 14,
            KeyCode::Up => {
                if self.focus == 0 {
                    self.adjust(-1)
                } else {
                    self.focus = (self.focus + 13) % 14
                }
            }
            KeyCode::Down => {
                if self.focus == 0 {
                    self.adjust(1)
                } else {
                    self.focus = (self.focus + 1) % 14
                }
            }
            KeyCode::Left => self.adjust(-1),
            KeyCode::Right => self.adjust(1),
            KeyCode::Enter | KeyCode::Char(' ') => return self.activate(),
            KeyCode::Char('a') if self.dirty() => return Intent::ApplyLighting,
            KeyCode::Char('u') if self.undo.is_some() => return Intent::UndoLighting,
            KeyCode::Char('r') => self.restore_draft(self.original.lighting),
            KeyCode::Char('s') => self.editor = Some(Editor::Name(String::new())),
            KeyCode::Char('x') => self.editor = Some(Editor::Hex(self.draft.hex())),
            KeyCode::Char('q') | KeyCode::Esc => return Intent::Quit,
            _ => {}
        }
        Intent::None
    }
    pub fn mouse(&mut self, event: MouseEvent, area: Rect, status: &mut String) -> Intent {
        if self.editor.is_some() {
            if event.kind == MouseEventKind::Down(MouseButton::Left) {
                let (apply, cancel) = editor_buttons(area);
                if cancel.contains(Position::new(event.column, event.row)) {
                    self.editor = None;
                } else if apply.contains(Position::new(event.column, event.row)) {
                    return self.handle(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), status);
                }
            }
            return Intent::None;
        }
        let g = LightLayout::new(area);
        let pt = Position::new(event.column, event.row);
        if matches!(
            event.kind,
            MouseEventKind::ScrollDown | MouseEventKind::ScrollUp
        ) {
            if g.presets.contains(pt) {
                self.focus = 0;
                self.adjust(if event.kind == MouseEventKind::ScrollUp {
                    -1
                } else {
                    1
                });
            }
            return Intent::None;
        }
        if !matches!(
            event.kind,
            MouseEventKind::Down(MouseButton::Left) | MouseEventKind::Drag(MouseButton::Left)
        ) {
            return Intent::None;
        }
        for (i, r) in g.sliders.iter().enumerate() {
            if r.contains(pt) {
                self.focus = i + 2;
                let max = if i < 3 { 255 } else { 4 };
                let v = ((event.column - r.x) as u32 * max
                    / u32::from(r.width.saturating_sub(1).max(1))) as u8;
                if i < 3 {
                    let mut c = self.draft.color();
                    c[i] = v;
                    self.set_color(c);
                } else if i == 3 {
                    self.draft.params[2] = v
                } else {
                    self.draft.params[1] = 4 - v
                };
                self.mark_custom();
                return Intent::None;
            }
        }
        if event.kind != MouseEventKind::Down(MouseButton::Left) {
            return Intent::None;
        }
        if g.presets.contains(pt) {
            let start = self.preset_start(g.presets.height);
            let i = start + (event.row - g.presets.y) as usize;
            if i < self.presets.len() {
                self.selected = i;
                self.focus = 0;
                return self.activate();
            }
        }
        for (i, r) in g.swatches.iter().enumerate() {
            if r.contains(pt) {
                self.set_color(SWATCHES[i]);
                return Intent::None;
            }
        }
        for (focus, r) in &g.controls {
            if r.contains(pt) {
                self.focus = *focus;
                if *focus == 1 {
                    self.adjust(if event.column < r.x + r.width / 2 {
                        -1
                    } else {
                        1
                    });
                    return Intent::None;
                }
                return self.activate();
            }
        }
        Intent::None
    }
    fn preset_start(&self, h: u16) -> usize {
        self.selected.saturating_sub(h.saturating_sub(1) as usize)
    }
}
pub struct LightLayout {
    pub presets: Rect,
    pub preview: Rect,
    pub sliders: [Rect; 5],
    pub swatches: Vec<Rect>,
    pub controls: Vec<(usize, Rect)>,
    library: Rect,
    editor: Rect,
    pub workspace: Rect,
}
impl LightLayout {
    pub fn new(area: Rect) -> Self {
        let a = theme::workspace(area);
        let w = (a.width - 8).min(90);
        let h = if a.height >= 32 { 14 } else { 8 };
        let preview = Rect::new(a.x + (a.width - w) / 2, a.y + 4, w, h);
        let y = preview.bottom() + 1;
        let bottom = a.bottom() - 3;
        let library = Rect::new(a.x + 2, y, 20, bottom - y);
        let presets = library.inner(Margin::new(1, 1));
        let x = a.x + 24;
        let w = a.width - 26;
        let editor = Rect::new(x, y, w, bottom - y);
        let col = (w - 4) / 3;
        let sliders = [
            Rect::new(x, y + 3, col, 1),
            Rect::new(x + col + 2, y + 3, col, 1),
            Rect::new(x + (col + 2) * 2, y + 3, col, 1),
            Rect::new(x, y + 5, col, 1),
            Rect::new(x + col + 2, y + 5, col, 1),
        ];
        let swatches = (0..8).map(|i| Rect::new(x + i * 3, y + 6, 2, 1)).collect();
        let mut controls = vec![
            (1, Rect::new(x, y, w / 2, 1)),
            (7, Rect::new(x + w / 2 + 1, y, w - w / 2 - 1, 1)),
            (8, Rect::new(x + 27, y + 6, w - 27, 1)),
        ];
        let mut left = a.x + (a.width - 73) / 2;
        for (focus, width) in [(9, 21), (10, 14), (11, 10), (12, 10), (13, 10)] {
            controls.push((focus, Rect::new(left, a.bottom() - 3, width, 1)));
            left += width + 2;
        }
        Self {
            presets,
            preview,
            sliders,
            swatches,
            controls,
            library,
            editor,
            workspace: a,
        }
    }
}
fn panel(f: &mut Frame, r: Rect, title: &str) {
    f.render_widget(
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(Line::from(title).style(Style::default().fg(DIM)))
            .border_style(Style::default().fg(BORDER))
            .style(Style::default().bg(PANEL).fg(DIM)),
        r,
    )
}
fn text(f: &mut Frame, r: Rect, s: impl Into<String>, fg: Color, bg: Color) {
    f.render_widget(
        Paragraph::new(s.into()).style(Style::default().fg(fg).bg(bg)),
        r,
    )
}
fn control(f: &mut Frame, r: Rect, s: &str, focus: bool) {
    f.render_widget(
        Paragraph::new(s).alignment(Alignment::Center).style(
            Style::default()
                .fg(if focus { MINT } else { TEXT })
                .bg(if focus { SELECTED } else { PANEL }),
        ),
        r,
    )
}
/// A multicolor animation has no single accurate per-key terminal palette.
pub fn preview_color(l: Lighting) -> Option<[u8; 3]> {
    if l.rainbow() {
        return None;
    }
    let mut c = l.color();
    for v in &mut c {
        *v = (u16::from(*v) * u16::from(l.brightness()) / 4) as u8;
    }
    Some(c)
}
pub fn draw(f: &mut Frame, s: &UiState) {
    let g = LightLayout::new(f.area());
    let a = g.workspace;
    f.render_widget(
        Paragraph::new("Air 01")
            .alignment(Alignment::Center)
            .style(Style::default().fg(TEXT).bold()),
        Rect::new(a.x, a.y, a.width, 1),
    );
    super::view::workspace_nav(f, s);
    let Some(l) = &s.lighting else {
        text(
            f,
            Rect::new(a.x + 4, a.y + 6, a.width - 8, 3),
            format!("Lighting unavailable. {}", s.status),
            GOLD,
            BG,
        );
        if let Some((_, r)) = g.controls.iter().find(|(i, _)| *i == 13) {
            control(f, *r, "Reload", true);
        }
        if matches!(s.mode, super::Mode::Help) {
            super::view::draw_help(f);
        }
        if let super::Mode::Notice(t) = &s.mode {
            super::view::draw_notice(f, t);
        }
        return;
    };
    let dirty = l.dirty();
    f.render_widget(
        Paragraph::new(format!(
            "Connected via USB · Profile {} · {}",
            l.original.profile,
            if dirty {
                "Draft — not applied"
            } else {
                "Up to date"
            }
        ))
        .alignment(Alignment::Center)
        .style(Style::default().fg(if dirty { GOLD } else { DIM })),
        Rect::new(a.x, a.y + 1, a.width, 1),
    );
    let color = if l.draft.rainbow() {
        "Multicolor effect".to_string()
    } else {
        l.draft.hex()
    };
    panel(f, g.preview, &format!(" {} · {} ", l.active_name(), color));
    let inner = g.preview.inner(Margin::new(1, 1));
    let pitch = if g.preview.height >= 14 { 2 } else { 1 };
    let rgb = preview_color(l.draft);
    for k in keyboard::keys() {
        let x = inner.x + (u32::from(k.x) * u32::from(inner.width) / 732) as u16;
        let right = inner.x
            + ((u32::from(k.x + k.width) * u32::from(inner.width) / 732) as u16).min(inner.width);
        let y = inner.y + k.y * pitch;
        if right <= x || y >= inner.bottom() {
            continue;
        }
        let r = Rect::new(x, y, (right - x).saturating_sub(1).max(1), 1);
        let (bg, fg) = match rgb {
            Some([red, green, blue]) => {
                let luminance =
                    (u32::from(red) * 299 + u32::from(green) * 587 + u32::from(blue) * 114) / 1000;
                (
                    Color::Rgb(red, green, blue),
                    if luminance > 150 { TEXT } else { Color::White },
                )
            }
            None => (KEY, DIM),
        };
        f.render_widget(
            Paragraph::new(k.legend.chars().take(r.width as usize).collect::<String>())
                .alignment(Alignment::Center)
                .style(Style::default().fg(fg).bg(bg)),
            r,
        );
    }
    let library = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(Line::from(" Presets ").style(Style::default().fg(DIM)))
        .title_bottom(
            Line::from(format!(" ↑↓ {}/{} ", l.selected + 1, l.presets.len()))
                .style(Style::default().fg(DIM)),
        )
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(PANEL).fg(DIM));
    f.render_widget(library, g.library);
    let start = l.preset_start(g.presets.height);
    for (row, p) in l
        .presets
        .iter()
        .skip(start)
        .take(g.presets.height as usize)
        .enumerate()
    {
        let i = start + row;
        let selected = i == l.selected;
        let active = p.name == l.active_name();
        let [red, green, blue] = p.lighting.color();
        let r = Rect::new(g.presets.x, g.presets.y + row as u16, g.presets.width, 1);
        let line = Line::from(vec![
            Span::styled(if active { "✓ " } else { "  " }, Style::default().fg(MINT)),
            Span::styled(
                if i == 0 { "◉ " } else { "● " },
                Style::default().fg(if i == 0 {
                    MINT
                } else {
                    Color::Rgb(red, green, blue)
                }),
            ),
            Span::raw(p.name.chars().take(13).collect::<String>()),
        ]);
        f.render_widget(
            Paragraph::new(line).style(Style::default().fg(TEXT).bg(if selected {
                SELECTED
            } else {
                PANEL
            })),
            r,
        );
    }
    f.render_widget(Block::new().style(Style::default().bg(PANEL)), g.editor);
    let effect = format!("‹ {} ›", l.draft.effect());
    let rainbow = format!("{} Multicolor", if l.draft.rainbow() { "☑" } else { "☐" });
    for (focus, r) in &g.controls {
        let label = match focus {
            1 => effect.as_str(),
            7 => rainbow.as_str(),
            8 => "x Color…",
            9 => "a Apply to keyboard",
            10 => "s Save preset…",
            11 => "u Undo",
            12 => "r Revert",
            13 => "Reload",
            _ => "",
        };
        control(f, *r, label, l.focus == *focus);
        if *focus == 9 && dirty {
            f.render_widget(
                Paragraph::new(label)
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(Color::White).bg(MINT).bold()),
                *r,
            );
        }
        if (*focus == 9 && !dirty)
            || (*focus == 11 && l.undo.is_none())
            || (*focus == 12 && !dirty)
            || (*focus == 13 && dirty)
        {
            text(f, *r, format!(" {label}"), DIM, KEY);
        }
    }
    let caption = if l.draft.rainbow() {
        "Multicolor animation runs on keyboard".into()
    } else {
        format!("Color {}", l.draft.hex())
    };
    text(
        f,
        Rect::new(g.editor.x, g.editor.y + 1, g.editor.width, 1),
        caption,
        DIM,
        PANEL,
    );
    let c = l.draft.color();
    for (i, r) in g.sliders.iter().enumerate() {
        let (name, v, max, color) = match i {
            0 => ("Red", c[0], 255, Color::Rgb(220, 45, 65)),
            1 => ("Green", c[1], 255, Color::Rgb(25, 135, 65)),
            2 => ("Blue", c[2], 255, MINT),
            3 => ("Brightness", l.draft.brightness(), 4, MINT),
            _ => ("Speed", l.draft.speed(), 4, MINT),
        };
        text(
            f,
            Rect::new(r.x, r.y - 1, r.width, 1),
            format!("{name} {v}"),
            if l.focus == i + 2 { MINT } else { DIM },
            PANEL,
        );
        let filled = (u32::from(v) * u32::from(r.width) / max) as usize;
        let width = r.width as usize;
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("━".repeat(filled), Style::default().fg(color)),
                Span::styled("─".repeat(width - filled), Style::default().fg(BORDER)),
            ]))
            .style(Style::default().bg(PANEL)),
            *r,
        );
    }
    for (i, r) in g.swatches.iter().enumerate() {
        let [red, green, blue] = SWATCHES[i];
        text(f, *r, "  ", TEXT, Color::Rgb(red, green, blue));
    }
    if g.editor.height >= 8 {
        text(
            f,
            Rect::new(g.editor.x, g.editor.y + 7, g.editor.width, 1),
            format!(
                "{}{}",
                l.active_name(),
                if l.draft.mode() == 1 {
                    " · Static ignores speed"
                } else {
                    ""
                }
            ),
            DIM,
            PANEL,
        );
    }
    let status = if s.status.starts_with("Click a key") {
        if dirty {
            "Previewing changes. Apply when ready."
        } else {
            "Choose a preset, or adjust a color to create Custom."
        }
    } else {
        &s.status
    };
    text(
        f,
        Rect::new(a.x + 2, a.bottom() - 2, a.width - 4, 1),
        status,
        DIM,
        BG,
    );
    f.render_widget(
        Paragraph::new("Tab focus · ←→ adjust · Enter choose · ? Help · q Quit")
            .alignment(Alignment::Center)
            .style(Style::default().fg(DIM)),
        Rect::new(a.x, a.bottom() - 1, a.width, 1),
    );
    if let Some(e) = &l.editor {
        draw_editor(f, e)
    }
    if matches!(s.mode, super::Mode::Help) {
        super::view::draw_help(f);
    }
    if let super::Mode::Notice(t) = &s.mode {
        super::view::draw_notice(f, t);
    }
}
fn editor_rect(a: Rect) -> Rect {
    let a = theme::workspace(a);
    Rect::new(a.x + (a.width - 60) / 2, a.y + (a.height - 10) / 2, 60, 10)
}
fn editor_buttons(a: Rect) -> (Rect, Rect) {
    let p = editor_rect(a);
    (
        Rect::new(p.x + 3, p.bottom() - 2, 18, 1),
        Rect::new(p.x + 25, p.bottom() - 2, 18, 1),
    )
}
fn draw_editor(f: &mut Frame, e: &Editor) {
    let p = editor_rect(f.area());
    f.render_widget(Clear, p);
    panel(
        f,
        p,
        match e {
            Editor::Hex(_) => " CUSTOM COLOR ",
            Editor::Name(_) => " SAVE PRESET ",
        },
    );
    let (value, hint) = match e {
        Editor::Hex(s) => (s, "Six hex digits · Ctrl+U clears · Enter previews"),
        Editor::Name(s) => (s, "Name: letters, digits, - or _ · existing names kept"),
    };
    text(
        f,
        Rect::new(p.x + 3, p.y + 2, p.width - 6, 1),
        hint,
        DIM,
        PANEL,
    );
    text(
        f,
        Rect::new(p.x + 3, p.y + 4, p.width - 6, 1),
        format!("{value}▏"),
        MINT,
        BG,
    );
    let (apply, cancel) = editor_buttons(f.area());
    control(f, apply, "Enter Continue", true);
    control(f, cancel, "Esc Cancel", false);
}
