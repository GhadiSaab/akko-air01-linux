//! Shared visual hierarchy and bounded workspace for terminal and mouse layout.
use ratatui::prelude::*;
pub const BG: Color = Color::Rgb(242, 242, 247);
pub const PANEL: Color = Color::Rgb(255, 255, 255);
pub const TEXT: Color = Color::Rgb(29, 29, 31);
pub const DIM: Color = Color::Rgb(110, 110, 115);
pub const ACCENT: Color = Color::Rgb(0, 122, 255);
pub const SELECTED: Color = Color::Rgb(225, 238, 255);
pub const GOLD: Color = Color::Rgb(160, 100, 0);
pub const BORDER: Color = Color::Rgb(210, 210, 215);
pub const KEY: Color = Color::Rgb(248, 248, 250);
pub fn workspace(a: Rect) -> Rect {
    let w = a.width.min(112);
    let h = a.height.min(36);
    Rect::new(a.x + (a.width - w) / 2, a.y + (a.height - h) / 2, w, h)
}
pub fn navigation(a: Rect) -> [Rect; 3] {
    let a = workspace(a);
    let x = a.x + (a.width.saturating_sub(29)) / 2;
    [
        Rect::new(x, a.y + 2, 14, 1),
        Rect::new(x + 15, a.y + 2, 14, 1),
        Rect::new(a.right() - 9, a.y + 2, 9, 1),
    ]
}
