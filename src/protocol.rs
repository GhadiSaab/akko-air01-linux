//! Gen2 feature reports. Report ID 0 is included in the 65-byte buffer.
use anyhow::{Result, bail, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Action(pub [u8; 4]);
impl Action {
    pub const FN: Self = Self([10, 1, 0, 0]);
    pub const CTRL: Self = Self([0, 0, 224, 0]);
    pub const DISABLED: Self = Self([0; 4]);
    pub fn label(self) -> String {
        if self == Self::FN {
            return "Fn".into();
        }
        if self == Self::DISABLED {
            return "Disabled".into();
        }
        if self.0[0] == 0 {
            return self.0[1..]
                .iter()
                .filter(|&&v| v != 0)
                .map(|&v| usage_label(v))
                .collect::<Vec<_>>()
                .join("+");
        }
        format!(
            "raw:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3]
        )
    }
}
pub fn usage_label(v: u8) -> String {
    match v {
        4..=29 => char::from(b'A' + v - 4).to_string(),
        30..=38 => char::from(b'1' + v - 30).to_string(),
        39 => "0".into(),
        58..=69 => format!("F{}", v - 57),
        _ => match v {
            40 => "Enter",
            41 => "Escape",
            42 => "Backspace",
            43 => "Tab",
            44 => "Space",
            45 => "Minus",
            46 => "Equal",
            47 => "LeftBracket",
            48 => "RightBracket",
            49 => "Backslash",
            51 => "Semicolon",
            52 => "Apostrophe",
            53 => "Grave",
            54 => "Comma",
            55 => "Period",
            56 => "Slash",
            57 => "CapsLock",
            70 => "PrintScreen",
            71 => "ScrollLock",
            72 => "Pause",
            73 => "Insert",
            74 => "Home",
            75 => "PageUp",
            76 => "Delete",
            77 => "End",
            78 => "PageDown",
            79 => "Right",
            80 => "Left",
            81 => "Down",
            82 => "Up",
            224 => "LeftCtrl",
            225 => "LeftShift",
            226 => "LeftAlt",
            227 => "LeftMeta",
            228 => "RightCtrl",
            229 => "RightShift",
            230 => "RightAlt",
            231 => "RightMeta",
            _ => return format!("HID:{v:02x}"),
        }
        .into(),
    }
}
pub fn packet(op: u8, args: &[u8], data: Option<Action>) -> Result<[u8; 65]> {
    ensure!(args.len() <= 6, "command header exceeds six argument bytes");
    let mut p = [0; 65];
    p[1] = op;
    p[2..2 + args.len()].copy_from_slice(args);
    p[8] = 255 - (p[1..8].iter().map(|&b| b as u16).sum::<u16>() & 255) as u8;
    if let Some(a) = data {
        p[9..13].copy_from_slice(&a.0);
    }
    Ok(p)
}
pub fn detect_gen2(options: &[u8; 64], keys: &[u8; 64], other_sleep: &[u8; 64]) -> bool {
    let plain = |p: &[u8; 64]| {
        p.chunks_exact(4)
            .filter(|k| k[0] == 0 && k[1] == 0 && (4..=231).contains(&k[2]) && k[3] == 0)
            .count()
    };
    let contradictory_sleep = (0..4).all(|i| {
        (30..=36000).contains(&u16::from_le_bytes([
            other_sleep[1 + 2 * i],
            other_sleep[2 + 2 * i],
        ]))
    });
    options[0] == 0x89 && plain(options) < 8 && plain(keys) >= 8 && !contradictory_sleep
}
pub fn check_profile(p: u8) -> Result<()> {
    ensure!(p < 4, "profile must be 0..3");
    Ok(())
}
pub fn check_slot(s: u8) -> Result<()> {
    ensure!(s < 128, "slot must be 0..127");
    Ok(())
}
impl std::str::FromStr for Action {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        if s.eq_ignore_ascii_case("fn") {
            return Ok(Self::FN);
        }
        if s.eq_ignore_ascii_case("disabled") {
            return Ok(Self::DISABLED);
        }
        if let Some(raw) = s.strip_prefix("raw:") {
            let bytes = raw
                .split(':')
                .map(|v| u8::from_str_radix(v, 16))
                .collect::<std::result::Result<Vec<_>, _>>()?;
            ensure!(bytes.len() == 4, "raw action requires four hex bytes");
            return Ok(Self(
                bytes
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("four bytes required"))?,
            ));
        }
        for v in 4..=231 {
            if usage_label(v).eq_ignore_ascii_case(s) {
                return Ok(Self([0, 0, v, 0]));
            }
        }
        bail!("unknown action {s:?}; use a key name, Fn, Disabled, or raw:00:00:e0:00")
    }
}
