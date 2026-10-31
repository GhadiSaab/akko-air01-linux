mod common;
use air01::{
    animation::{self, Direction, Frame, Marquee, Style},
    device::Board,
    keyboard, lighting,
};
use common::MemoryWire;

fn m(color: [u8; 3], direction: Direction, style: Style, delay_ms: u16) -> Marquee {
    Marquee {
        color,
        direction,
        style,
        delay_ms,
        hold_ms: 600,
    }
}
fn bit7(p: &[u8; 65]) -> bool {
    p[1..9].iter().map(|&v| v as u16).sum::<u16>() & 255 == 255
}
fn slot(legend: &str) -> usize {
    keyboard::keys()
        .iter()
        .find(|k| k.legend == legend)
        .unwrap()
        .slot as usize
}

#[test]
fn start_and_frame_packets_match_vendor_layout() {
    let s = animation::start_packet();
    assert_eq!(&s[..8], &[0, 0x12, 0, 0, 0, 0, 0, 0]);
    assert!(bit7(&s));

    let mut f = Frame::default();
    f.0[0] = [1, 2, 3];
    f.0[18] = [4, 5, 6]; // bytes 54..57 straddle pages 0 and 1
    f.0[127] = [7, 8, 9];
    let ps = animation::frame_packets(3, 40, 0x1234, &f);
    assert_eq!(ps.len(), 7);
    for (page, p) in ps.iter().enumerate() {
        assert_eq!(&p[1..8], &[0x12, 3, page as u8, 1, 40, 0x34, 0x12]);
        assert!(bit7(p));
    }
    assert_eq!(&ps[0][9..12], &[1, 2, 3]);
    assert_eq!(&ps[0][63..65], &[4, 5]);
    assert_eq!(ps[1][9], 6);
    // slot 127 → bytes 381..384 → page 6 offset 45
    assert_eq!(&ps[6][9 + 45..9 + 48], &[7, 8, 9]);
}

#[test]
fn scroll_style_moves_the_banner_right_on_a_dark_loop() {
    let orange = [255, 85, 0];
    let a = animation::marquee("CLAUDE", m(orange, Direction::Right, Style::Scroll, 120)).unwrap();
    // 6 glyphs × 5 + 5 gaps = 35 columns; offsets -35..=16.
    assert_eq!(a.frames.len(), 52);
    assert!(a.frames.first().unwrap().lit().is_empty());
    assert!(a.frames.last().unwrap().lit().is_empty());
    assert!(
        a.frames
            .iter()
            .flat_map(|f| f.0.iter())
            .all(|c| *c == [0; 3] || *c == orange)
    );
    // Moving right, E's right column (#,.,.,.,#) enters at column 0 first.
    let first = &a.frames[1];
    assert_eq!(first.0[slot("Esc")], orange);
    assert_eq!(first.0[slot("Tab")], [0; 3]);
    assert_eq!(first.0[slot("`")], [0; 3]);
    assert!(a.frames.iter().all(|f| f.0[slot("Space")] == [0; 3]));
    let b = animation::marquee("CLAUDE", m(orange, Direction::Left, Style::Scroll, 120)).unwrap();
    let mut rev = b.frames.clone();
    rev.reverse();
    assert_eq!(rev, a.frames);
}

#[test]
fn letters_style_pauses_each_centered_letter() {
    let orange = [255, 85, 0];
    let a = animation::marquee("CLAUDE", m(orange, Direction::Right, Style::Letters, 60)).unwrap();
    // Per letter: 22 travel frames + 600/60 = 10 held copies.
    assert_eq!(a.frames.len(), 6 * 32);
    // L, centered on columns 5..=9: its stem is on column 5 in every font row.
    let l = &a.frames[32 + 5 + 5];
    for legend in ["F5", "5", "R", "F", "V"] {
        assert_eq!(l.0[slot(legend)], orange, "{legend}");
    }
    assert_eq!(l.0[slot("F6")], [0; 3]);
    assert_eq!(l.0[slot("B")], orange);
    assert!(
        animation::marquee(
            "ABCDEFGHIJ",
            m(orange, Direction::Right, Style::Letters, 60)
        )
        .is_err()
    );
}

#[test]
fn marquee_rejects_bad_input() {
    let s = Style::Scroll;
    assert!(animation::marquee("", m([255, 0, 0], Direction::Right, s, 100)).is_err());
    assert!(animation::marquee("CLAUDE?", m([255, 0, 0], Direction::Right, s, 100)).is_err());
    assert!(animation::marquee("CLAUDE", m([0, 0, 0], Direction::Right, s, 100)).is_err());
    assert!(animation::marquee("CLAUDE", m([255, 0, 0], Direction::Right, s, 0)).is_err());
}

#[test]
fn upload_sends_frames_then_selects_mode_25_with_backup() {
    let w = MemoryWire::new();
    let mut b = Board::connect(w.clone(), true).unwrap();
    let before = lighting::read(&mut b).unwrap();
    let a = animation::marquee(
        "CLAUDE",
        m([255, 85, 0], Direction::Right, Style::Scroll, 120),
    )
    .unwrap();
    let target = animation::lighting_for([255, 85, 0], 3).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let up = animation::upload_in(&mut b, &before, &a, target, dir.path()).unwrap();
    assert_eq!(up.packets, 1 + 52 * 7);
    assert_eq!(lighting::load(&up.backup).unwrap(), before);
    let writes = w.0.borrow().writes.clone();
    assert_eq!(writes[0], animation::start_packet());
    assert!(writes[1..=364].iter().all(|p| p[1] == 0x12));
    assert_eq!(writes.last().unwrap()[1], 7);
    let after = lighting::read(&mut b).unwrap().lighting;
    assert_eq!(after, target);
    assert_eq!(after.effect(), "Custom animation");
    assert_eq!(after.hex(), "#FF5500");
}

#[test]
fn upload_refuses_wireless_and_stale_state() {
    let w = MemoryWire::new();
    let mut b = Board::connect(w.clone(), false).unwrap();
    let before = lighting::read(&mut b).unwrap();
    let a = animation::marquee("HI", m([255, 0, 0], Direction::Right, Style::Scroll, 50)).unwrap();
    let target = animation::lighting_for([255, 0, 0], 4).unwrap();
    let dir = tempfile::tempdir().unwrap();
    assert!(animation::upload_in(&mut b, &before, &a, target, dir.path()).is_err());

    let mut b = Board::connect(w.clone(), true).unwrap();
    w.0.borrow_mut().lighting[2] = 1;
    assert!(animation::upload_in(&mut b, &before, &a, target, dir.path()).is_err());
    assert!(w.0.borrow().writes.is_empty());
}

#[test]
fn failed_upload_leaves_lighting_mode_untouched() {
    let w = MemoryWire::new();
    let mut b = Board::connect(w.clone(), true).unwrap();
    let before = lighting::read(&mut b).unwrap();
    w.0.borrow_mut().fail_at = Some(5);
    let a = animation::marquee(
        "CLAUDE",
        m([255, 85, 0], Direction::Right, Style::Scroll, 120),
    )
    .unwrap();
    let target = animation::lighting_for([255, 85, 0], 4).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let e = animation::upload_in(&mut b, &before, &a, target, dir.path()).unwrap_err();
    assert!(format!("{e:#}").contains("lighting mode was not changed"));
    assert_eq!(lighting::read(&mut b).unwrap(), before);
}
