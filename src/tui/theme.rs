//! One terminal-native shell shared by every workspace.
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Paragraph},
};
pub const BG: Color = Color::Reset;
pub const PANEL: Color = BG;
pub const TEXT: Color = Color::Reset;
pub const KEY_TEXT: Color = Color::Rgb(213, 214, 219);
pub const DIM: Color = Color::Rgb(145, 150, 175);
pub const ACCENT: Color = Color::Rgb(125, 207, 255);
pub const SELECTED: Color = Color::Rgb(48, 62, 85);
pub const GOLD: Color = Color::Rgb(224, 175, 104);
pub const BORDER: Color = Color::Rgb(68, 75, 106);
pub const KEY: Color = Color::Rgb(46, 51, 72);
pub fn workspace(a: Rect) -> Rect {
    let w = a.width.min(112);
    let h = a.height.min(36);
    Rect::new(a.x + (a.width - w) / 2, a.y + (a.height - h) / 2, w, h)
}
pub fn navigation(a: Rect) -> [Rect; 3] {
    let a = workspace(a);
    let x = a.x + (a.width.saturating_sub(29)) / 2;
    [
        Rect::new(x, a.y + 1, 14, 1),
        Rect::new(x + 15, a.y + 1, 14, 1),
        Rect::new(a.right() - 10, a.y + 1, 8, 1),
    ]
}
pub fn divider(f: &mut Frame, a: Rect, y: u16) {
    if y > a.y && y < a.bottom() - 1 {
        f.render_widget(
            Paragraph::new(format!(
                "├{}┤",
                "─".repeat(a.width.saturating_sub(2) as usize)
            ))
            .style(Style::default().fg(BORDER).bg(BG)),
            Rect::new(a.x, y, a.width, 1),
        );
    }
}
pub fn shell(f: &mut Frame, status: &str, context: &str) {
    let a = workspace(f.area());
    f.render_widget(
        Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(BORDER))
            .style(Style::default().bg(BG).fg(TEXT))
            .title(Line::from(" air01 ").style(Style::default().fg(ACCENT).bold()))
            .title(
                Line::from(format!(" {status} "))
                    .alignment(Alignment::Right)
                    .style(Style::default().fg(DIM)),
            )
            .title_bottom(
                Line::from(format!(" {context} · ? help · q quit "))
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(DIM)),
            ),
        a,
    );
    divider(f, a, a.y + 3);
    divider(f, a, a.bottom() - 4);
}
