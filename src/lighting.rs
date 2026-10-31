//! Air 01 Gen2 global backlight parameters. Protocol research: see NOTICE.
//! Keep the seven wire bytes intact for exact backups, including unknown options.
use crate::{
    device::Board,
    protocol::check_profile,
    transport::{Wire, state_dir},
};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lighting {
    pub params: [u8; 7],
}
// Air01 vendor class d8 explicitly overrides the generic Gen2 flag pair.
pub const FIXED_COLOR: u8 = 8;
pub const RAINBOW_COLOR: u8 = 7;
pub const EFFECTS: &[(u8, &str)] = &[
    (1, "Static"),
    (2, "Breathing"),
    (3, "Spectrum"),
    (4, "Wave"),
    (5, "Ripple"),
    (6, "Stars"),
    (7, "Snake"),
    (8, "Reactive"),
    (9, "Converge"),
    (10, "Sine wave"),
    (11, "Kaleidoscope"),
    (12, "Line wave"),
    (14, "Laser"),
    (15, "Circle"),
    (16, "Dazzle"),
    (17, "Rain"),
    (18, "Meteor"),
    (19, "Reactive off"),
];
impl Lighting {
    pub fn new(mode: u8, color: [u8; 3], brightness: u8, speed: u8, rainbow: bool) -> Result<Self> {
        ensure!(
            EFFECTS.iter().any(|e| e.0 == mode),
            "unsupported lighting effect"
        );
        ensure!(
            brightness <= 4 && speed <= 4,
            "brightness and speed must be 0..4"
        );
        let color = if color == [255; 3] { [250; 3] } else { color };
        Ok(Self {
            params: [
                mode,
                4 - speed,
                brightness,
                if mode == 4 { 0x20 } else { 0 }
                    | if rainbow { RAINBOW_COLOR } else { FIXED_COLOR },
                color[0],
                color[1],
                color[2],
            ],
        })
    }
    pub fn with_color(mut self, c: [u8; 3]) -> Self {
        let c = if c == [255; 3] { [250; 3] } else { c };
        self.params[3] = (self.params[3] & 0xf0) | FIXED_COLOR;
        self.params[4..7].copy_from_slice(&c);
        if self.mode() == 3 {
            self.params[0] = 1;
        }
        self
    }
    pub fn with_rainbow(mut self, on: bool) -> Self {
        if !on && self.mode() == 3 {
            self.params[0] = 1;
        }
        self.params[3] = (self.params[3] & 0xf0) | if on { RAINBOW_COLOR } else { FIXED_COLOR };
        self
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.params[1] <= 4 && self.params[2] <= 4,
            "invalid lighting speed/brightness"
        );
        Ok(())
    }
    pub fn mode(self) -> u8 {
        self.params[0]
    }
    pub fn effect(self) -> String {
        EFFECTS.iter().find(|e| e.0 == self.mode()).map_or_else(
            || match self.mode() {
                25 => "Custom animation".into(),
                m => format!("Device effect {m}"),
            },
            |e| e.1.into(),
        )
    }
    pub fn speed(self) -> u8 {
        4 - self.params[1].min(4)
    }
    pub fn brightness(self) -> u8 {
        self.params[2]
    }
    pub fn rainbow(self) -> bool {
        self.params[3] & 15 == RAINBOW_COLOR || self.mode() == 3
    }
    pub fn color(self) -> [u8; 3] {
        let raw: [u8; 3] = self.params[4..7].try_into().expect("three bytes");
        const COLORS: [[u8; 3]; 7] = [
            [255, 0, 0],
            [0, 255, 0],
            [0, 0, 255],
            [255, 85, 0],
            [119, 0, 255],
            [255, 255, 0],
            [255, 255, 255],
        ];
        if let Some(color) = COLORS.get((self.params[3] & 15) as usize) {
            return *color;
        }
        if raw == [250; 3] { [255; 3] } else { raw }
    }
    pub fn hex(self) -> String {
        let [r, g, b] = self.color();
        format!("#{r:02X}{g:02X}{b:02X}")
    }
    pub fn packet(self) -> [u8; 65] {
        let mut p = [0; 65];
        p[1] = 7;
        p[2..9].copy_from_slice(&self.params);
        p[9] = 255u8.wrapping_sub(p[1..9].iter().fold(0u8, |a, b| a.wrapping_add(*b)));
        p
    }
}
pub fn parse_color(s: &str) -> Result<[u8; 3]> {
    let s = s.strip_prefix('#').unwrap_or(s);
    ensure!(
        s.len() == 6 && s.bytes().all(|b| b.is_ascii_hexdigit()),
        "use six hex digits, e.g. #5EEAD4"
    );
    Ok([
        u8::from_str_radix(&s[0..2], 16)?,
        u8::from_str_radix(&s[2..4], 16)?,
        u8::from_str_radix(&s[4..6], 16)?,
    ])
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LightingSnapshot {
    pub schema: u32,
    pub device_id: u32,
    pub profile: u8,
    pub lighting: Lighting,
}
impl LightingSnapshot {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.schema == 1 && self.device_id == 3013,
            "unsupported lighting snapshot"
        );
        check_profile(self.profile)?;
        self.lighting.validate()
    }
}
pub fn read<W: Wire>(b: &mut Board<W>) -> Result<LightingSnapshot> {
    b.check_identity()?;
    let profile = b.active_profile()?;
    let r = b.wire.query(0x87, &[])?;
    ensure!(r[0] == 0x87, "invalid lighting response");
    ensure!(
        b.active_profile()? == profile,
        "profile changed while reading lighting"
    );
    let s = LightingSnapshot {
        schema: 1,
        device_id: b.device_id,
        profile,
        lighting: Lighting {
            params: r[1..8].try_into()?,
        },
    };
    s.validate()?;
    Ok(s)
}
pub fn save(s: &LightingSnapshot, path: &Path) -> Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    s.validate()?;
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .with_context(|| {
            format!(
                "cannot create {}; existing files are never overwritten",
                path.display()
            )
        })?;
    serde_json::to_writer_pretty(&mut f, s)?;
    writeln!(f)?;
    f.sync_all()?;
    Ok(())
}
pub fn load(path: &Path) -> Result<LightingSnapshot> {
    let f = std::fs::File::open(path)?;
    ensure!(f.metadata()?.len() <= 16384, "lighting file exceeds 16 KiB");
    let s: LightingSnapshot = serde_json::from_reader(f)?;
    s.validate()?;
    Ok(s)
}
pub fn theme_path(dir: &Path, name: &str) -> Result<PathBuf> {
    ensure!(
        !name.is_empty()
            && name.len() <= 32
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
        "preset name: 1–32 letters, digits, - or _"
    );
    Ok(dir.join(format!("{name}.json")))
}
pub fn save_theme(name: &str, s: &LightingSnapshot) -> Result<PathBuf> {
    save_theme_in(&state_dir()?.join("themes"), name, s)
}
pub fn save_theme_in(dir: &Path, name: &str, s: &LightingSnapshot) -> Result<PathBuf> {
    let path = theme_path(dir, name)?;
    ensure!(
        !name.eq_ignore_ascii_case("Custom")
            && !presets().iter().any(|p| p.name.eq_ignore_ascii_case(name)),
        "choose a name different from a built-in preset"
    );
    std::fs::create_dir_all(dir)?;
    for entry in std::fs::read_dir(dir)? {
        let p = entry?.path();
        ensure!(
            p.extension().is_none_or(|e| e != "json")
                || !p
                    .file_stem()
                    .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(name)),
            "a preset with this name already exists (case insensitive)"
        );
    }
    save(s, &path)?;
    Ok(path)
}
#[derive(Clone, Debug)]
pub struct Preset {
    pub name: String,
    pub lighting: Lighting,
    pub custom: bool,
}
pub fn presets() -> Vec<Preset> {
    [
        ("Mint", 1, [94, 234, 212], 4, false),
        ("Ember", 2, [255, 102, 40], 3, false),
        ("Ocean", 1, [56, 130, 255], 4, false),
        ("Rose", 2, [255, 85, 150], 3, false),
        ("Amethyst", 1, [166, 100, 255], 4, false),
        ("Frost", 1, [230, 245, 255], 4, false),
        ("Aurora", 4, [94, 234, 212], 4, true),
        ("Focus", 1, [255, 190, 110], 1, false),
    ]
    .into_iter()
    .map(|(name, m, c, b, r)| Preset {
        name: name.into(),
        lighting: Lighting::new(m, c, b, 2, r).expect("built-in preset"),
        custom: false,
    })
    .collect()
}
pub struct Library {
    pub presets: Vec<Preset>,
    pub warnings: Vec<String>,
}
pub fn library() -> Result<Vec<Preset>> {
    Ok(library_report()?.presets)
}
pub fn library_report() -> Result<Library> {
    library_from(&state_dir()?.join("themes"))
}
pub fn library_from(dir: &Path) -> Result<Library> {
    let mut out = Library {
        presets: presets(),
        warnings: vec![],
    };
    if !dir.exists() {
        return Ok(out);
    }
    let mut paths = std::fs::read_dir(dir)?
        .map(|e| e.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    paths.sort();
    for path in paths {
        if path.extension().is_some_and(|e| e == "json") {
            match load(&path) {
                Ok(s) => {
                    let name = path
                        .file_stem()
                        .context("invalid preset name")?
                        .to_string_lossy()
                        .into_owned();
                    out.presets.push(Preset {
                        name,
                        lighting: s.lighting,
                        custom: true,
                    });
                }
                Err(e) => out
                    .warnings
                    .push(format!("Skipped {}: {e:#}", path.display())),
            }
        }
    }
    Ok(out)
}
/// Optimistic concurrency, exact backup, one write, readback; no blind retry.
pub fn apply<W: Wire>(
    b: &mut Board<W>,
    expected: &LightingSnapshot,
    target: Lighting,
) -> Result<Option<PathBuf>> {
    apply_in(b, expected, target, &state_dir()?.join("lighting-backups"))
}
/// Use an explicit recovery directory (for embedding or isolated tests).
pub fn apply_in<W: Wire>(
    b: &mut Board<W>,
    expected: &LightingSnapshot,
    target: Lighting,
    dir: &Path,
) -> Result<Option<PathBuf>> {
    expected.validate()?;
    target.validate()?;
    ensure!(b.wired, "lighting writes require wired USB mode");
    ensure!(
        read(b)? == *expected,
        "lighting or active profile changed externally; reload before applying"
    );
    if target == expected.lighting {
        return Ok(None);
    }
    std::fs::create_dir_all(dir)?;
    let path = dir.join(format!(
        "lighting-{}.json",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    ));
    save(expected, &path)?;
    let attempt = (|| -> Result<()> {
        b.check_identity()?;
        ensure!(read(b)? == *expected, "lighting changed before write");
        b.wire.send(target.packet())?;
        b.wire.wait(Duration::from_secs(1));
        let got = read(b)?;
        ensure!(
            got.profile == expected.profile && got.lighting == target,
            "lighting readback mismatch"
        );
        Ok(())
    })();
    if let Err(error) = attempt {
        b.wire.wait(Duration::from_secs(1));
        let recovery = (|| -> Result<()> {
            let current = read(b)?;
            ensure!(
                current.profile == expected.profile,
                "profile changed; recovery stopped"
            );
            if current.lighting != expected.lighting {
                ensure!(
                    current.lighting == target,
                    "lighting changed externally; recovery stopped"
                );
                b.wire.send(expected.lighting.packet())?;
                b.wire.wait(Duration::from_secs(1));
            }
            ensure!(read(b)? == *expected, "original lighting readback mismatch");
            Ok(())
        })();
        match recovery {
            Ok(()) => bail!(
                "{error:#}; original lighting verified. Backup: {}",
                path.display()
            ),
            Err(e) => bail!(
                "{error:#}; recovery INCOMPLETE: {e:#}. Backup: {}",
                path.display()
            ),
        }
    }
    Ok(Some(path))
}
