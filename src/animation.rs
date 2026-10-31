//! Onboard per-key animations (light mode 25, vendor `LightUserColor`).
//!
//! The vendor Gen2 serializer (`setUserGifStart` / `setUserGif` in
//! https://web.akkogear.com/js/4abb36b7.js) starts an upload with opcode
//! `0x12` and byte 3 = 0, then sends every frame as seven Bit7 pages:
//! `12 frame page 01 count delay_lo delay_hi checksum` + 56 RGB bytes, with
//! RGB triples in matrix-slot order. The firmware stores the frames and plays
//! them by itself, like the built-in effects; nothing has to keep running on
//! the host. There is no read-back command for uploaded frames.
use crate::{
    device::Board,
    keyboard,
    lighting::{self, FIXED_COLOR, Lighting, LightingSnapshot},
    protocol::packet,
    transport::{Wire, state_dir},
};
use anyhow::{Context, Result, bail, ensure};
use std::{
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub const OPCODE: u8 = 0x12;
pub const MODE: u8 = 25;
pub const SLOTS: usize = 128;
pub const PAGES: u8 = 7;
pub const PAGE_BYTES: usize = 56;
/// The frame index and count are single bytes on the wire.
pub const MAX_FRAMES: usize = 255;
/// Columns of the 46-unit key pitch used by `keyboard::keys()`.
pub const COLUMNS: i32 = 16;
const PITCH: i32 = 46;
/// Font rows map onto the function row through the bottom letter row.
const FONT_ROWS: usize = 5;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame(pub [[u8; 3]; SLOTS]);
impl Default for Frame {
    fn default() -> Self {
        Self([[0; 3]; SLOTS])
    }
}
impl Frame {
    pub fn lit(&self) -> Vec<u8> {
        (0..SLOTS as u8)
            .filter(|&s| self.0[s as usize] != [0; 3])
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Animation {
    pub frames: Vec<Frame>,
    /// Hold time of each frame in milliseconds.
    pub delay_ms: u16,
}
impl Animation {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            !self.frames.is_empty() && self.frames.len() <= MAX_FRAMES,
            "an animation needs 1..={MAX_FRAMES} frames"
        );
        ensure!(self.delay_ms > 0, "frame delay must be at least 1 ms");
        Ok(())
    }
}

/// `12 00 00 00 …`: resets the firmware's frame store before an upload.
pub fn start_packet() -> [u8; 65] {
    packet(OPCODE, &[0, 0, 0], None).expect("static header")
}

/// The seven pages of one frame. `count` is the total number of frames.
pub fn frame_packets(index: u8, count: u8, delay_ms: u16, frame: &Frame) -> Vec<[u8; 65]> {
    let mut rgb = [0u8; PAGES as usize * PAGE_BYTES];
    for (slot, c) in frame.0.iter().enumerate() {
        rgb[slot * 3..slot * 3 + 3].copy_from_slice(c);
    }
    let [lo, hi] = delay_ms.to_le_bytes();
    (0..PAGES)
        .map(|page| {
            let mut p =
                packet(OPCODE, &[index, page, 1, count, lo, hi], None).expect("six header bytes");
            let start = page as usize * PAGE_BYTES;
            p[9..65].copy_from_slice(&rgb[start..start + PAGE_BYTES]);
            p
        })
        .collect()
}

/// Bold 5×5 glyphs, rows top to bottom. Five columns survive the Air 01's
/// row stagger far better than a 3-column font.
fn glyph(c: char) -> Option<[&'static str; 5]> {
    Some(match c.to_ascii_uppercase() {
        'A' => [".###.", "#...#", "#####", "#...#", "#...#"],
        'B' => ["####.", "#...#", "####.", "#...#", "####."],
        'C' => [".####", "#....", "#....", "#....", ".####"],
        'D' => ["####.", "#...#", "#...#", "#...#", "####."],
        'E' => ["#####", "#....", "####.", "#....", "#####"],
        'F' => ["#####", "#....", "####.", "#....", "#...."],
        'G' => [".####", "#....", "#.###", "#...#", ".####"],
        'H' => ["#...#", "#...#", "#####", "#...#", "#...#"],
        'I' => ["#####", "..#..", "..#..", "..#..", "#####"],
        'J' => ["..###", "...#.", "...#.", "#..#.", ".##.."],
        'K' => ["#..#.", "#.#..", "##...", "#.#..", "#..#."],
        'L' => ["#....", "#....", "#....", "#....", "#####"],
        'M' => ["#...#", "##.##", "#.#.#", "#...#", "#...#"],
        'N' => ["#...#", "##..#", "#.#.#", "#..##", "#...#"],
        'O' => [".###.", "#...#", "#...#", "#...#", ".###."],
        'P' => ["####.", "#...#", "####.", "#....", "#...."],
        'Q' => [".###.", "#...#", "#.#.#", "#..#.", ".##.#"],
        'R' => ["####.", "#...#", "####.", "#..#.", "#...#"],
        'S' => [".####", "#....", ".###.", "....#", "####."],
        'T' => ["#####", "..#..", "..#..", "..#..", "..#.."],
        'U' => ["#...#", "#...#", "#...#", "#...#", ".###."],
        'V' => ["#...#", "#...#", "#...#", ".#.#.", "..#.."],
        'W' => ["#...#", "#...#", "#.#.#", "##.##", "#...#"],
        'X' => ["#...#", ".#.#.", "..#..", ".#.#.", "#...#"],
        'Y' => ["#...#", ".#.#.", "..#..", "..#..", "..#.."],
        'Z' => ["#####", "...#.", "..#..", ".#...", "#####"],
        '0' => [".###.", "#..##", "#.#.#", "##..#", ".###."],
        '1' => ["..#..", ".##..", "..#..", "..#..", ".###."],
        '2' => [".###.", "#...#", "..##.", ".#...", "#####"],
        '3' => ["####.", "....#", ".###.", "....#", "####."],
        '4' => ["#...#", "#...#", "#####", "....#", "....#"],
        '5' => ["#####", "#....", "####.", "....#", "####."],
        '6' => [".###.", "#....", "####.", "#...#", ".###."],
        '7' => ["#####", "....#", "...#.", "..#..", "..#.."],
        '8' => [".###.", "#...#", ".###.", "#...#", ".###."],
        '9' => [".###.", "#...#", ".####", "....#", ".###."],
        ' ' => [".....", ".....", ".....", ".....", "....."],
        '-' => [".....", ".....", ".###.", ".....", "....."],
        '!' => ["..#..", "..#..", "..#..", ".....", "..#.."],
        _ => return None,
    })
}
const GLYPH_WIDTH: i32 = 5;

type Columns = Vec<[bool; FONT_ROWS]>;

fn columns(c: char) -> Result<Columns> {
    let g = glyph(c).with_context(|| {
        format!("unsupported marquee character {c:?}; use A-Z, 0-9, space, - or !")
    })?;
    Ok((0..GLYPH_WIDTH as usize)
        .map(|x| std::array::from_fn(|row| g[row].as_bytes()[x] == b'#'))
        .collect())
}

fn check_text(text: &str) -> Result<()> {
    ensure!(
        !text.trim().is_empty() && text.chars().count() <= 32,
        "marquee text must be 1..=32 characters"
    );
    Ok(())
}

/// Text as columns of five pixels, one blank column between glyphs.
pub fn bitmap(text: &str) -> Result<Columns> {
    check_text(text)?;
    let mut cols = vec![];
    for (i, c) in text.chars().enumerate() {
        if i > 0 {
            cols.push([false; FONT_ROWS]);
        }
        cols.extend(columns(c)?);
    }
    Ok(cols)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Direction {
    /// Text moves from the left edge towards the right edge.
    Right,
    /// Classic ticker: text enters on the right and leaves on the left.
    Left,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Style {
    /// One letter at a time: slide in, pause in the middle, slide out.
    Letters,
    /// The whole text as one continuous banner.
    Scroll,
}

/// Physical keys as (slot, grid column, grid row) for the font area.
fn grid() -> Vec<(u8, i32, usize)> {
    keyboard::keys()
        .iter()
        .filter(|k| (k.y as usize) < FONT_ROWS)
        .map(|k| {
            let center = k.x as i32 + k.width as i32 / 2;
            (k.slot, center / PITCH, k.y as usize)
        })
        .collect()
}

/// Draw `cols` with its column 0 at keyboard column `offset`.
fn draw(keys: &[(u8, i32, usize)], cols: &Columns, offset: i32, color: [u8; 3]) -> Frame {
    let mut f = Frame::default();
    for &(slot, col, row) in keys {
        let t = col - offset;
        if (0..cols.len() as i32).contains(&t) && cols[t as usize][row] {
            f.0[slot as usize] = color;
        }
    }
    f
}

/// Offsets that carry `width` columns from fully off one edge to fully off
/// the other, so every pass begins and ends on a dark board.
fn travel(width: i32, direction: Direction) -> Vec<i32> {
    match direction {
        Direction::Right => (-width..=COLUMNS).collect(),
        Direction::Left => (-width..=COLUMNS).rev().collect(),
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Marquee {
    pub color: [u8; 3],
    pub direction: Direction,
    pub style: Style,
    /// Milliseconds per one-column step.
    pub delay_ms: u16,
    /// Pause on each centered letter (`Letters` style only).
    pub hold_ms: u16,
}

/// Animate `text` across the keyboard; every frame is one column step.
pub fn marquee(text: &str, m: Marquee) -> Result<Animation> {
    ensure!(m.color != [0; 3], "marquee color must not be black");
    ensure!(m.delay_ms > 0, "frame delay must be at least 1 ms");
    check_text(text)?;
    let keys = grid();
    let mut frames = vec![];
    match m.style {
        Style::Scroll => {
            let cols = bitmap(text)?;
            for offset in travel(cols.len() as i32, m.direction) {
                frames.push(draw(&keys, &cols, offset, m.color));
            }
        }
        Style::Letters => {
            let hold = (m.hold_ms / m.delay_ms) as usize;
            let center = (COLUMNS - GLYPH_WIDTH) / 2;
            for c in text.chars() {
                let cols = columns(c)?;
                for offset in travel(GLYPH_WIDTH, m.direction) {
                    let f = draw(&keys, &cols, offset, m.color);
                    if offset == center {
                        frames.extend(std::iter::repeat_n(f.clone(), hold));
                    }
                    frames.push(f);
                }
            }
        }
    }
    ensure!(
        frames.len() <= MAX_FRAMES,
        "{} frames exceed the keyboard's {MAX_FRAMES}; shorten the text or the pause",
        frames.len()
    );
    let a = Animation {
        frames,
        delay_ms: m.delay_ms,
    };
    a.validate()?;
    Ok(a)
}

/// Lighting parameters that play the uploaded frames.
pub fn lighting_for(color: [u8; 3], brightness: u8) -> Result<Lighting> {
    ensure!(brightness <= 4, "brightness must be 0..4");
    let mut params = [
        MODE,
        0,
        brightness,
        FIXED_COLOR,
        color[0],
        color[1],
        color[2],
    ];
    if color == [255; 3] {
        params[4..7].copy_from_slice(&[250; 3]);
    }
    Ok(Lighting { params })
}

#[derive(Debug)]
pub struct Upload {
    pub backup: PathBuf,
    pub packets: usize,
}

/// Upload frames, then select mode 25 through the verified lighting transaction.
pub fn upload<W: Wire>(
    b: &mut Board<W>,
    expected: &LightingSnapshot,
    animation: &Animation,
    target: Lighting,
) -> Result<Upload> {
    upload_in(
        b,
        expected,
        animation,
        target,
        &state_dir()?.join("lighting-backups"),
    )
}

pub fn upload_in<W: Wire>(
    b: &mut Board<W>,
    expected: &LightingSnapshot,
    animation: &Animation,
    target: Lighting,
    dir: &Path,
) -> Result<Upload> {
    animation.validate()?;
    target.validate()?;
    ensure!(
        target.mode() == MODE,
        "animation lighting must use mode {MODE}"
    );
    ensure!(b.wired, "animation uploads require wired USB mode");
    ensure!(
        lighting::read(b)? == *expected,
        "lighting or active profile changed externally; reload before uploading"
    );
    std::fs::create_dir_all(dir)?;
    let backup = dir.join(format!(
        "lighting-{}-before-animation.json",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    ));
    lighting::save(expected, &backup)?;
    let count = animation.frames.len() as u8;
    let mut packets = 0;
    let sent = (|| -> Result<()> {
        b.check_identity()?;
        b.wire.send(start_packet())?;
        packets += 1;
        b.wire.wait(Duration::from_millis(500));
        for (i, frame) in animation.frames.iter().enumerate() {
            for p in frame_packets(i as u8, count, animation.delay_ms, frame) {
                b.wire.send(p)?;
                packets += 1;
                b.wire.wait(Duration::from_millis(10));
            }
        }
        b.wire.wait(Duration::from_secs(1));
        b.check_identity()
    })();
    if let Err(e) = sent {
        bail!(
            "{e:#}; animation upload stopped after {packets} reports. The lighting \
             mode was not changed. Backup: {}",
            backup.display()
        );
    }
    let current = lighting::read(b)?;
    ensure!(
        current.profile == expected.profile,
        "active profile changed during upload; lighting mode not changed. Backup: {}",
        backup.display()
    );
    lighting::apply_in(b, &current, target, dir).with_context(|| {
        format!(
            "frames uploaded but selecting mode {MODE} failed. Backup: {}",
            backup.display()
        )
    })?;
    Ok(Upload { backup, packets })
}

/// Text rendering of one frame on the physical layout, for previews.
pub fn render_ascii(frame: &Frame) -> String {
    let mut rows = vec![vec!['.'; COLUMNS as usize]; 6];
    for k in keyboard::keys() {
        let col = ((k.x as i32 + k.width as i32 / 2) / PITCH).clamp(0, COLUMNS - 1) as usize;
        if frame.0[k.slot as usize] != [0; 3] {
            rows[k.y as usize][col] = '#';
        }
    }
    rows.into_iter()
        .map(|r| r.into_iter().collect::<String>() + "\n")
        .collect()
}
