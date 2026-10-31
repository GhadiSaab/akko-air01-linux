//! Physical ANSI Air 01 geometry, cross-checked against device3013's matrix.
//! Coordinates are measured layout facts, not a default-keycode-to-slot guess.
use crate::protocol::Action;
use ratatui::layout::{Position, Rect};
#[derive(Clone, Copy, Debug)]
pub struct Key {
    pub slot: u8,
    pub legend: &'static str,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub default: Action,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}
const KEYS: &[Key] = &[
    Key {
        slot: 0,
        legend: "Esc",
        x: 0,
        y: 0,
        width: 40,
        default: Action([0, 0, 41, 0]),
    },
    Key {
        slot: 6,
        legend: "F1",
        x: 46,
        y: 0,
        width: 40,
        default: Action([0, 0, 58, 0]),
    },
    Key {
        slot: 12,
        legend: "F2",
        x: 92,
        y: 0,
        width: 40,
        default: Action([0, 0, 59, 0]),
    },
    Key {
        slot: 18,
        legend: "F3",
        x: 138,
        y: 0,
        width: 40,
        default: Action([0, 0, 60, 0]),
    },
    Key {
        slot: 24,
        legend: "F4",
        x: 184,
        y: 0,
        width: 40,
        default: Action([0, 0, 61, 0]),
    },
    Key {
        slot: 30,
        legend: "F5",
        x: 230,
        y: 0,
        width: 40,
        default: Action([0, 0, 62, 0]),
    },
    Key {
        slot: 36,
        legend: "F6",
        x: 276,
        y: 0,
        width: 40,
        default: Action([0, 0, 63, 0]),
    },
    Key {
        slot: 42,
        legend: "F7",
        x: 322,
        y: 0,
        width: 40,
        default: Action([0, 0, 64, 0]),
    },
    Key {
        slot: 48,
        legend: "F8",
        x: 368,
        y: 0,
        width: 40,
        default: Action([0, 0, 65, 0]),
    },
    Key {
        slot: 54,
        legend: "F9",
        x: 414,
        y: 0,
        width: 40,
        default: Action([0, 0, 66, 0]),
    },
    Key {
        slot: 60,
        legend: "F10",
        x: 460,
        y: 0,
        width: 40,
        default: Action([0, 0, 67, 0]),
    },
    Key {
        slot: 66,
        legend: "F11",
        x: 506,
        y: 0,
        width: 40,
        default: Action([0, 0, 68, 0]),
    },
    Key {
        slot: 72,
        legend: "F12",
        x: 552,
        y: 0,
        width: 40,
        default: Action([0, 0, 69, 0]),
    },
    Key {
        slot: 78,
        legend: "Prt",
        x: 598,
        y: 0,
        width: 40,
        default: Action([0, 0, 70, 0]),
    },
    Key {
        slot: 84,
        legend: "Ins",
        x: 644,
        y: 0,
        width: 40,
        default: Action([0, 0, 73, 0]),
    },
    Key {
        slot: 87,
        legend: "Light",
        x: 690,
        y: 0,
        width: 40,
        default: Action([10, 9, 0, 0]),
    },
    Key {
        slot: 1,
        legend: "`",
        x: 0,
        y: 1,
        width: 40,
        default: Action([0, 0, 53, 0]),
    },
    Key {
        slot: 7,
        legend: "1",
        x: 46,
        y: 1,
        width: 40,
        default: Action([0, 0, 30, 0]),
    },
    Key {
        slot: 13,
        legend: "2",
        x: 92,
        y: 1,
        width: 40,
        default: Action([0, 0, 31, 0]),
    },
    Key {
        slot: 19,
        legend: "3",
        x: 138,
        y: 1,
        width: 40,
        default: Action([0, 0, 32, 0]),
    },
    Key {
        slot: 25,
        legend: "4",
        x: 184,
        y: 1,
        width: 40,
        default: Action([0, 0, 33, 0]),
    },
    Key {
        slot: 31,
        legend: "5",
        x: 230,
        y: 1,
        width: 40,
        default: Action([0, 0, 34, 0]),
    },
    Key {
        slot: 37,
        legend: "6",
        x: 276,
        y: 1,
        width: 40,
        default: Action([0, 0, 35, 0]),
    },
    Key {
        slot: 43,
        legend: "7",
        x: 322,
        y: 1,
        width: 40,
        default: Action([0, 0, 36, 0]),
    },
    Key {
        slot: 49,
        legend: "8",
        x: 368,
        y: 1,
        width: 40,
        default: Action([0, 0, 37, 0]),
    },
    Key {
        slot: 55,
        legend: "9",
        x: 414,
        y: 1,
        width: 40,
        default: Action([0, 0, 38, 0]),
    },
    Key {
        slot: 61,
        legend: "0",
        x: 460,
        y: 1,
        width: 40,
        default: Action([0, 0, 39, 0]),
    },
    Key {
        slot: 67,
        legend: "-",
        x: 506,
        y: 1,
        width: 40,
        default: Action([0, 0, 45, 0]),
    },
    Key {
        slot: 73,
        legend: "=",
        x: 552,
        y: 1,
        width: 40,
        default: Action([0, 0, 46, 0]),
    },
    Key {
        slot: 79,
        legend: "Bksp",
        x: 598,
        y: 1,
        width: 86,
        default: Action([0, 0, 42, 0]),
    },
    Key {
        slot: 85,
        legend: "Home",
        x: 690,
        y: 1,
        width: 40,
        default: Action([0, 0, 74, 0]),
    },
    Key {
        slot: 2,
        legend: "Tab",
        x: 0,
        y: 2,
        width: 66,
        default: Action([0, 0, 43, 0]),
    },
    Key {
        slot: 8,
        legend: "Q",
        x: 72,
        y: 2,
        width: 40,
        default: Action([0, 0, 20, 0]),
    },
    Key {
        slot: 14,
        legend: "W",
        x: 118,
        y: 2,
        width: 40,
        default: Action([0, 0, 26, 0]),
    },
    Key {
        slot: 20,
        legend: "E",
        x: 164,
        y: 2,
        width: 40,
        default: Action([0, 0, 8, 0]),
    },
    Key {
        slot: 26,
        legend: "R",
        x: 210,
        y: 2,
        width: 40,
        default: Action([0, 0, 21, 0]),
    },
    Key {
        slot: 32,
        legend: "T",
        x: 256,
        y: 2,
        width: 40,
        default: Action([0, 0, 23, 0]),
    },
    Key {
        slot: 38,
        legend: "Y",
        x: 302,
        y: 2,
        width: 40,
        default: Action([0, 0, 28, 0]),
    },
    Key {
        slot: 44,
        legend: "U",
        x: 348,
        y: 2,
        width: 40,
        default: Action([0, 0, 24, 0]),
    },
    Key {
        slot: 50,
        legend: "I",
        x: 394,
        y: 2,
        width: 40,
        default: Action([0, 0, 12, 0]),
    },
    Key {
        slot: 56,
        legend: "O",
        x: 440,
        y: 2,
        width: 40,
        default: Action([0, 0, 18, 0]),
    },
    Key {
        slot: 62,
        legend: "P",
        x: 486,
        y: 2,
        width: 40,
        default: Action([0, 0, 19, 0]),
    },
    Key {
        slot: 68,
        legend: "[",
        x: 532,
        y: 2,
        width: 40,
        default: Action([0, 0, 47, 0]),
    },
    Key {
        slot: 74,
        legend: "]",
        x: 578,
        y: 2,
        width: 40,
        default: Action([0, 0, 48, 0]),
    },
    Key {
        slot: 80,
        legend: "\\",
        x: 624,
        y: 2,
        width: 60,
        default: Action([0, 0, 49, 0]),
    },
    Key {
        slot: 86,
        legend: "End",
        x: 690,
        y: 2,
        width: 40,
        default: Action([0, 0, 77, 0]),
    },
    Key {
        slot: 3,
        legend: "Caps",
        x: 0,
        y: 3,
        width: 72,
        default: Action([0, 0, 57, 0]),
    },
    Key {
        slot: 9,
        legend: "A",
        x: 78,
        y: 3,
        width: 40,
        default: Action([0, 0, 4, 0]),
    },
    Key {
        slot: 15,
        legend: "S",
        x: 124,
        y: 3,
        width: 40,
        default: Action([0, 0, 22, 0]),
    },
    Key {
        slot: 21,
        legend: "D",
        x: 170,
        y: 3,
        width: 40,
        default: Action([0, 0, 7, 0]),
    },
    Key {
        slot: 27,
        legend: "F",
        x: 216,
        y: 3,
        width: 40,
        default: Action([0, 0, 9, 0]),
    },
    Key {
        slot: 33,
        legend: "G",
        x: 262,
        y: 3,
        width: 40,
        default: Action([0, 0, 10, 0]),
    },
    Key {
        slot: 39,
        legend: "H",
        x: 308,
        y: 3,
        width: 40,
        default: Action([0, 0, 11, 0]),
    },
    Key {
        slot: 45,
        legend: "J",
        x: 354,
        y: 3,
        width: 40,
        default: Action([0, 0, 13, 0]),
    },
    Key {
        slot: 51,
        legend: "K",
        x: 400,
        y: 3,
        width: 40,
        default: Action([0, 0, 14, 0]),
    },
    Key {
        slot: 57,
        legend: "L",
        x: 446,
        y: 3,
        width: 40,
        default: Action([0, 0, 15, 0]),
    },
    Key {
        slot: 63,
        legend: ";",
        x: 492,
        y: 3,
        width: 40,
        default: Action([0, 0, 51, 0]),
    },
    Key {
        slot: 69,
        legend: "'",
        x: 538,
        y: 3,
        width: 40,
        default: Action([0, 0, 52, 0]),
    },
    Key {
        slot: 75,
        legend: "Enter",
        x: 584,
        y: 3,
        width: 100,
        default: Action([0, 0, 40, 0]),
    },
    Key {
        slot: 81,
        legend: "PgUp",
        x: 690,
        y: 3,
        width: 40,
        default: Action([0, 0, 75, 0]),
    },
    Key {
        slot: 4,
        legend: "Shift",
        x: 0,
        y: 4,
        width: 92,
        default: Action([0, 0, 225, 0]),
    },
    Key {
        slot: 10,
        legend: "Z",
        x: 99,
        y: 4,
        width: 40,
        default: Action([0, 0, 29, 0]),
    },
    Key {
        slot: 16,
        legend: "X",
        x: 145,
        y: 4,
        width: 40,
        default: Action([0, 0, 27, 0]),
    },
    Key {
        slot: 22,
        legend: "C",
        x: 191,
        y: 4,
        width: 40,
        default: Action([0, 0, 6, 0]),
    },
    Key {
        slot: 28,
        legend: "V",
        x: 237,
        y: 4,
        width: 40,
        default: Action([0, 0, 25, 0]),
    },
    Key {
        slot: 34,
        legend: "B",
        x: 283,
        y: 4,
        width: 40,
        default: Action([0, 0, 5, 0]),
    },
    Key {
        slot: 40,
        legend: "N",
        x: 329,
        y: 4,
        width: 40,
        default: Action([0, 0, 17, 0]),
    },
    Key {
        slot: 46,
        legend: "M",
        x: 375,
        y: 4,
        width: 40,
        default: Action([0, 0, 16, 0]),
    },
    Key {
        slot: 52,
        legend: ",",
        x: 421,
        y: 4,
        width: 40,
        default: Action([0, 0, 54, 0]),
    },
    Key {
        slot: 58,
        legend: ".",
        x: 467,
        y: 4,
        width: 40,
        default: Action([0, 0, 55, 0]),
    },
    Key {
        slot: 64,
        legend: "/",
        x: 513,
        y: 4,
        width: 40,
        default: Action([0, 0, 56, 0]),
    },
    Key {
        slot: 70,
        legend: "Shift",
        x: 559,
        y: 4,
        width: 79,
        default: Action([0, 0, 229, 0]),
    },
    Key {
        slot: 82,
        legend: "↑",
        x: 644,
        y: 4,
        width: 40,
        default: Action([0, 0, 82, 0]),
    },
    Key {
        slot: 88,
        legend: "PgDn",
        x: 690,
        y: 4,
        width: 40,
        default: Action([0, 0, 78, 0]),
    },
    Key {
        slot: 5,
        legend: "Fn",
        x: 0,
        y: 5,
        width: 40,
        default: Action::FN,
    },
    Key {
        slot: 11,
        legend: "Ctrl",
        x: 46,
        y: 5,
        width: 40,
        default: Action([0, 0, 224, 0]),
    },
    Key {
        slot: 17,
        legend: "Opt",
        x: 92,
        y: 5,
        width: 40,
        default: Action([0, 0, 227, 0]),
    },
    Key {
        slot: 23,
        legend: "Cmd",
        x: 138,
        y: 5,
        width: 48,
        default: Action([0, 0, 226, 0]),
    },
    Key {
        slot: 41,
        legend: "Space",
        x: 192,
        y: 5,
        width: 292,
        default: Action([0, 0, 44, 0]),
    },
    Key {
        slot: 65,
        legend: "Cmd",
        x: 490,
        y: 5,
        width: 48,
        default: Action([0, 0, 230, 0]),
    },
    Key {
        slot: 71,
        legend: "Opt",
        x: 544,
        y: 5,
        width: 48,
        default: Action([0, 0, 231, 0]),
    },
    Key {
        slot: 77,
        legend: "←",
        x: 598,
        y: 5,
        width: 40,
        default: Action([0, 0, 80, 0]),
    },
    Key {
        slot: 83,
        legend: "↓",
        x: 644,
        y: 5,
        width: 40,
        default: Action([0, 0, 81, 0]),
    },
    Key {
        slot: 89,
        legend: "→",
        x: 690,
        y: 5,
        width: 40,
        default: Action([0, 0, 79, 0]),
    },
];
pub fn keys() -> &'static [Key] {
    KEYS
}
pub fn key(slot: u8) -> Option<&'static Key> {
    KEYS.iter().find(|k| k.slot == slot)
}
pub fn rectangles(area: Rect, large: bool) -> Vec<(u8, Rect)> {
    let pitch = if large { 3 } else { 2 };
    let height = if large { 4 } else { 3 };
    let needed = 5 * pitch + height;
    if area.width < 70 || area.height < needed {
        return vec![];
    }
    let top = area.y + (area.height - needed) / 2;
    KEYS.iter()
        .map(|k| {
            let left = u32::from(k.x) * u32::from(area.width) / 732;
            let right = u32::from(k.x + k.width) * u32::from(area.width) / 732;
            (
                k.slot,
                Rect::new(
                    area.x + left as u16,
                    top + k.y * pitch,
                    (right - left) as u16,
                    height,
                ),
            )
        })
        .collect()
}
pub fn hit(area: Rect, large: bool, x: u16, y: u16) -> Option<u8> {
    rectangles(area, large)
        .iter()
        .rev()
        .find(|(_, r)| r.contains(Position::new(x, y)))
        .map(|(s, _)| *s)
}
pub fn neighbor(slot: u8, direction: Direction) -> u8 {
    let Some(current) = key(slot) else {
        return 0;
    };
    let center = |k: &Key| i32::from(k.x) * 2 + i32::from(k.width);
    let cx = center(current);
    match direction {
        Direction::Left | Direction::Right => KEYS
            .iter()
            .filter(|k| {
                k.y == current.y
                    && if direction == Direction::Left {
                        center(k) < cx
                    } else {
                        center(k) > cx
                    }
            })
            .min_by_key(|k| (center(k) - cx).abs())
            .map_or(slot, |k| k.slot),
        Direction::Up | Direction::Down => {
            let row = KEYS
                .iter()
                .filter(|k| {
                    if direction == Direction::Up {
                        k.y < current.y
                    } else {
                        k.y > current.y
                    }
                })
                .min_by_key(|k| k.y.abs_diff(current.y))
                .map(|k| k.y);
            KEYS.iter()
                .filter(|k| Some(k.y) == row)
                .min_by_key(|k| (center(k) - cx).abs())
                .map_or(slot, |k| k.slot)
        }
    }
}
