//! Render a saved snapshot without opening hardware: snapshot.json cols rows output.svg [picker]
use air01::{
    operations::load_snapshot,
    tui::{UiState, draw},
};
use anyhow::{Context, Result};
use ratatui::{backend::TestBackend, prelude::*};
fn rgb(c: Color) -> String {
    match c {
        Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        Color::White => "#ffffff".into(),
        Color::Black => "#000000".into(),
        _ => "#dce5f3".into(),
    }
}
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    anyhow::ensure!(
        args.len() >= 4,
        "snapshot.json cols rows output.svg [picker]"
    );
    let cols: u16 = args[1].parse()?;
    let rows: u16 = args[2].parse()?;
    anyhow::ensure!(
        cols > 0 && cols <= 240 && rows > 0 && rows <= 100,
        "preview dimensions out of bounds"
    );
    let mut ui = UiState::new(load_snapshot(std::path::Path::new(&args[0]))?);
    ui.selected_slot = 5;
    if args.get(4).is_some_and(|v| v == "picker") {
        ui.open_picker();
    }
    let mut terminal = Terminal::new(TestBackend::new(cols, rows))?;
    terminal.draw(|f| draw(f, &ui))?;
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\"><rect width=\"100%\" height=\"100%\" fill=\"#0a0f18\"/>",
        cols * 9,
        rows * 18
    );
    for y in 0..rows {
        for x in 0..cols {
            let cell = &terminal.backend().buffer()[(x, y)];
            let bg = if cell.bg == Color::Reset {
                "#0a0f18".into()
            } else {
                rgb(cell.bg)
            };
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"9\" height=\"18\" fill=\"{bg}\"/>",
                x * 9,
                y * 18
            ));
            if cell.symbol() != " " {
                out.push_str(&format!("<text x=\"{}\" y=\"{}\" font-family=\"DejaVu Sans Mono\" font-size=\"14\" font-weight=\"{}\" fill=\"{}\" textLength=\"9\" lengthAdjust=\"spacingAndGlyphs\">{}</text>",x*9,y*18+14,if cell.modifier.contains(Modifier::BOLD){700}else{400},rgb(cell.fg),escape(cell.symbol())));
            }
        }
    }
    out.push_str("</svg>");
    std::fs::write(&args[3], out).context("write SVG preview")?;
    Ok(())
}
