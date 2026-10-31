//! Visual lighting workspace. Layout rectangles are also its input hit targets.
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
const BG: Color = Color::Rgb(10, 15, 24);
const PANEL: Color = Color::Rgb(16, 23, 35);
const TEXT: Color = Color::Rgb(220, 229, 243);
const DIM: Color = Color::Rgb(125, 144, 169);
const MINT: Color = Color::Rgb(94, 234, 212);
const GOLD: Color = Color::Rgb(251, 191, 84);
const BORDER: Color = Color::Rgb(57, 73, 96);
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
    pub focus: usize,
    pub editor: Option<Editor>,
    pub undo: Option<LightingSnapshot>,
}
impl LightingUi {
    pub fn new(original: LightingSnapshot, presets: Vec<Preset>) -> Self {
        Self {
            draft: original.lighting,
            original,
            presets,
            selected: 0,
            focus: 0,
            editor: None,
            undo: None,
        }
    }
    pub fn dirty(&self) -> bool {
        self.draft != self.original.lighting
    }
    pub fn set_color(&mut self, c: [u8; 3]) {
        let c = if c == [255; 3] { [250; 3] } else { c };
        self.draft.params[3] = (self.draft.params[3] & 0xf0) | 7;
        self.draft.params[4..7].copy_from_slice(&c);
        if self.draft.mode() == 3 {
            self.draft.params[0] = 1;
        }
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
                if self.draft.mode() == 3 {
                    self.draft.params[0] = 1;
                }
                self.draft.params[3] =
                    (self.draft.params[3] & 0xf0) | if self.draft.rainbow() { 7 } else { 8 };
            }
            _ => {}
        }
    }
    fn activate(&mut self) -> Intent {
        match self.focus {
            0 => {
                if let Some(p) = self.presets.get(self.selected) {
                    self.draft = p.lighting;
                }
            }
            1..=7 => self.adjust(1),
            8 => self.editor = Some(Editor::Hex(self.draft.hex())),
            9 => return Intent::ApplyLighting,
            10 => self.editor = Some(Editor::Name(String::new())),
            11 => return Intent::UndoLighting,
            12 => self.draft = self.original.lighting,
            13 => return Intent::ReloadLighting,
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
            KeyCode::Char('a') => return Intent::ApplyLighting,
            KeyCode::Char('u') => return Intent::UndoLighting,
            KeyCode::Char('r') => self.draft = self.original.lighting,
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
}
impl LightLayout {
    pub fn new(a: Rect) -> Self {
        let left = 22;
        let library = Rect::new(a.x, a.y + 4, left, a.height - 8);
        let presets = Rect::new(library.x + 1, library.y + 2, left - 2, library.height - 4);
        let x = a.x + left + 2;
        let w = a.width - left - 3;
        let preview_h = if a.height >= 32 { 14 } else { 8 };
        let preview = Rect::new(x, a.y + 4, w, preview_h);
        let editor = Rect::new(x, preview.bottom(), w, a.bottom() - 4 - preview.bottom());
        let col = (w - 4) / 3;
        let y = editor.y;
        let sliders = [
            Rect::new(x, y + 3, col, 1),
            Rect::new(x + col + 2, y + 3, col, 1),
            Rect::new(x + (col + 2) * 2, y + 3, col, 1),
            Rect::new(x, y + 5, col, 1),
            Rect::new(x + col + 2, y + 5, col, 1),
        ];
        let swatches = (0..8).map(|i| Rect::new(x + i * 3, y + 6, 2, 1)).collect();
        let controls = vec![
            (1, Rect::new(x, y, w / 2, 1)),
            (7, Rect::new(x + w / 2 + 1, y, w - w / 2 - 1, 1)),
            (8, Rect::new(x + 27, y + 6, w.saturating_sub(27), 1)),
            (9, Rect::new(a.x + 1, a.bottom() - 3, 12, 1)),
            (10, Rect::new(a.x + 15, a.bottom() - 3, 12, 1)),
            (11, Rect::new(a.x + 29, a.bottom() - 3, 12, 1)),
            (12, Rect::new(a.x + 43, a.bottom() - 3, 12, 1)),
            (13, Rect::new(a.x + 57, a.bottom() - 3, 13, 1)),
        ];
        Self {
            presets,
            preview,
            sliders,
            swatches,
            controls,
            library,
            editor,
        }
    }
}
fn panel(f: &mut Frame, r: Rect, title: &str) {
    f.render_widget(
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(title)
            .border_style(Style::default().fg(BORDER))
            .style(Style::default().bg(PANEL)),
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
    text(
        f,
        r,
        format!(" {s}"),
        if focus { BG } else { MINT },
        if focus { MINT } else { PANEL },
    )
}
pub fn draw(f: &mut Frame, s: &UiState) {
    let a = f.area();
    let g = LightLayout::new(a);
    text(
        f,
        Rect::new(a.x, a.y, a.width, 1),
        " air01  /  LIGHTING STUDIO",
        MINT,
        BG,
    );
    let Some(l) = &s.lighting else {
        text(
            f,
            Rect::new(a.x + 2, a.y + 5, a.width - 4, 3),
            format!("Lighting unavailable. {}  [r] reload", s.status),
            GOLD,
            BG,
        );
        super::view::workspace_nav(f, s);
        return;
    };
    let dirty = l.dirty();
    text(
        f,
        Rect::new(a.x + 1, a.y + 1, a.width - 2, 1),
        format!(
            "USB · active profile {}    {}",
            l.original.profile,
            if dirty {
                "● Unsaved draft · Apply to keyboard"
            } else {
                "✓ Synced with keyboard"
            }
        ),
        if dirty { GOLD } else { DIM },
        BG,
    );
    super::view::workspace_nav(f, s);
    panel(f, g.library, " PRESET LIBRARY ");
    text(
        f,
        Rect::new(g.library.x + 1, g.library.y + 1, g.library.width - 2, 1),
        " Select to preview",
        DIM,
        PANEL,
    );
    let start = l.preset_start(g.presets.height);
    for (row, p) in l
        .presets
        .iter()
        .skip(start)
        .take(g.presets.height as usize)
        .enumerate()
    {
        let r = Rect::new(g.presets.x, g.presets.y + row as u16, g.presets.width, 1);
        let [red, green, blue] = p.lighting.color();
        let sel = start + row == l.selected;
        let name = p.name.chars().take(14).collect::<String>();
        let line = Line::from(vec![
            Span::styled(if sel { " ▸ " } else { "   " }, Style::default().fg(MINT)),
            Span::styled("● ", Style::default().fg(Color::Rgb(red, green, blue))),
            Span::raw(name),
            Span::styled(if p.custom { " +" } else { "" }, Style::default().fg(DIM)),
        ]);
        f.render_widget(
            Paragraph::new(line).style(
                Style::default()
                    .fg(if sel { TEXT } else { DIM })
                    .bg(if sel { Color::Rgb(29, 45, 59) } else { PANEL }),
            ),
            r,
        );
    }
    text(
        f,
        Rect::new(
            g.library.x + 1,
            g.library.bottom() - 2,
            g.library.width - 2,
            1,
        ),
        format!(" {}/{} · ↑↓ Enter", l.selected + 1, l.presets.len()),
        DIM,
        PANEL,
    );
    panel(f, g.preview, " COLOR PREVIEW ");
    let inner = g.preview.inner(Margin::new(1, 1));
    let pitch = if g.preview.height >= 14 { 2 } else { 1 };
    for k in keyboard::keys() {
        let x = inner.x + (u32::from(k.x) * u32::from(inner.width) / 732) as u16;
        let right = inner.x
            + ((u32::from(k.x + k.width) * u32::from(inner.width) / 732) as u16).min(inner.width);
        let row = k.y; // normalized physical row, 0..5
        let y = inner.y + row * pitch;
        if right <= x || y >= inner.bottom() {
            continue;
        }
        let mut c = if l.draft.rainbow() {
            SWATCHES[(usize::from(k.x / 80) + usize::from(row)) % 7]
        } else {
            l.draft.color()
        };
        let b = u16::from(l.draft.brightness());
        for ch in &mut c {
            *ch = (u16::from(*ch) * b / 4) as u8;
        }
        let bg = Color::Rgb(c[0] / 4, c[1] / 4, c[2] / 4);
        let fg = Color::Rgb(c[0].max(40), c[1].max(40), c[2].max(40));
        let r = Rect::new(x, y, (right - x).saturating_sub(1).max(1), 1);
        f.render_widget(
            Paragraph::new(k.legend.chars().take(r.width as usize).collect::<String>())
                .alignment(Alignment::Center)
                .style(Style::default().fg(fg).bg(bg).bold()),
            r,
        );
    }
    let effect = format!("◀ {} ▶", l.draft.effect());
    let spectrum = format!("{} Rainbow", if l.draft.rainbow() { "☑" } else { "☐" });
    for (focus, r) in &g.controls {
        let label = match focus {
            1 => effect.as_str(),
            7 => spectrum.as_str(),
            8 => "x Hex color",
            9 => "a Apply",
            10 => "s Save",
            11 => "u Undo",
            12 => "r Revert",
            13 => "Reload",
            _ => "",
        };
        control(f, *r, label, l.focus == *focus);
    }
    text(
        f,
        Rect::new(g.editor.x, g.editor.y + 1, g.editor.width, 1),
        format!(
            "{}    {}",
            l.draft.hex(),
            if l.draft.rainbow() {
                "color varies by effect"
            } else {
                "custom single color"
            }
        ),
        DIM,
        BG,
    );
    let c = l.draft.color();
    for (i, r) in g.sliders.iter().enumerate() {
        let (name, v, max, color) = match i {
            0 => ("Red", c[0], 255, Color::Rgb(255, 110, 130)),
            1 => ("Green", c[1], 255, MINT),
            2 => ("Blue", c[2], 255, Color::Rgb(110, 160, 255)),
            3 => ("Brightness", l.draft.brightness(), 4, GOLD),
            _ => ("Speed", l.draft.speed(), 4, Color::Rgb(180, 150, 255)),
        };
        text(
            f,
            Rect::new(r.x, r.y - 1, r.width, 1),
            format!("{name} {v}"),
            if l.focus == i + 2 { TEXT } else { DIM },
            BG,
        );
        let filled = (u32::from(v) * u32::from(r.width) / max) as usize;
        let width = r.width as usize;
        let line = Line::from(vec![
            Span::styled("━".repeat(filled), Style::default().fg(color)),
            Span::styled("─".repeat(width - filled), Style::default().fg(BORDER)),
        ]);
        f.render_widget(Paragraph::new(line), *r);
    }
    for (i, r) in g.swatches.iter().enumerate() {
        let [red, green, blue] = SWATCHES[i];
        text(f, *r, "  ", TEXT, Color::Rgb(red, green, blue));
    }
    let note = if l.draft.mode() == 1 {
        "Static effect · speed applies to animated effects".to_string()
    } else {
        format!(
            "Preset: {}",
            l.presets
                .get(l.selected)
                .map_or("Custom", |p| p.name.as_str())
        )
    };
    text(
        f,
        Rect::new(g.editor.x, g.editor.bottom() - 1, g.editor.width, 1),
        note,
        DIM,
        BG,
    );
    text(
        f,
        Rect::new(a.x, a.bottom() - 4, a.width, 1),
        format!(
            " {}",
            if s.status.starts_with("Click a key") {
                "Choose a preset or color. Apply sends this draft to the keyboard."
            } else {
                &s.status
            }
        ),
        if s.status.starts_with("Error") {
            GOLD
        } else {
            DIM
        },
        BG,
    );
    text(
        f,
        Rect::new(a.x, a.bottom() - 2, a.width, 1),
        " Tab focus · ←→ adjust · Enter select · click/drag sliders · x hex · ? help",
        DIM,
        BG,
    );
    text(
        f,
        Rect::new(a.x, a.bottom() - 1, a.width, 1),
        " Preview only until Apply · Undo restores last applied lighting · q quit",
        DIM,
        BG,
    );
    if let Some(editor) = &l.editor {
        draw_editor(f, editor);
    }
    if let super::Mode::Notice(text) = &s.mode {
        super::view::draw_notice(f, text);
    }
    if matches!(s.mode, super::Mode::Help) {
        super::view::draw_help(f);
    }
}
fn editor_rect(a: Rect) -> Rect {
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
