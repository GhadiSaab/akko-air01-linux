use air01::{
    device::{Board, Layer},
    operations::{self, Change},
    protocol::{Action, check_profile},
    snapshot::Snapshot,
    transport::{self, HidWire},
};
use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
#[derive(Parser)]
#[command(
    version,
    about = "Akko Air 01 native Linux configurator. USB writes, verified and backed up."
)]
struct Cli {
    /// Select the settings HID node when more than one board is connected.
    #[arg(long, global = true)]
    device: Option<String>,
    /// Normal profile (0..3); defaults to the currently active profile.
    #[arg(short,long,global=true,value_parser=clap::value_parser!(u8).range(0..4))]
    profile: Option<u8>,
    /// Print intended changes without writing configuration or backup files.
    #[arg(long, global = true)]
    dry_run: bool,
    /// Explicitly allow a base profile with no Fn key.
    #[arg(long, global = true)]
    allow_no_fn: bool,
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    /// Inspect or configure global backlight and reusable RGB presets.
    Lighting {
        #[command(subcommand)]
        command: LightCommand,
    },
    /// List vendor settings interfaces; does not write to devices.
    Devices,
    /// Open the terminal interface.
    Tui,
    /// Identify the wired board and its active profile.
    Info,
    /// Inspect all four profiles and locate the Fn/Ctrl assignments.
    Profiles,
    /// Show a profile's slot numbers and exact four-byte entries.
    Keymap {
        #[arg(long, default_value = "base")]
        layer: Layer,
    },
    /// Save all four base maps and their Windows/Mac Fn maps (not macros/RGB).
    Backup { path: PathBuf },
    /// Restore a snapshot, preserving the current active profile unless --activate is given.
    Restore {
        path: PathBuf,
        #[arg(long)]
        activate: bool,
    },
    /// Change the active profile and verify the selection.
    Profile {
        #[arg(value_parser=clap::value_parser!(u8).range(0..4))]
        index: u8,
    },
    /// Change one matrix slot. ACTION: key name, Fn, Disabled, or raw:00:00:e0:00.
    Remap {
        #[arg(value_parser=clap::value_parser!(u8).range(0..128))]
        slot: u8,
        action: Action,
        #[arg(long, default_value = "base")]
        layer: Layer,
    },
    /// Idempotently set physical Fn as LeftCtrl and physical LeftCtrl as Fn.
    SwapFnCtrl {
        #[arg(long, conflicts_with = "profile")]
        all_profiles: bool,
    },
}
#[derive(Subcommand)]
enum LightCommand {
    Show,
    Presets,
    Set {
        #[arg(long, value_parser=air01::lighting::parse_color)]
        color: Option<[u8; 3]>,
        #[arg(long,value_parser=clap::value_parser!(u8).range(1..20))]
        effect: Option<u8>,
        #[arg(long,value_parser=clap::value_parser!(u8).range(0..5))]
        brightness: Option<u8>,
        #[arg(long,value_parser=clap::value_parser!(u8).range(0..5))]
        speed: Option<u8>,
        #[arg(long, conflicts_with = "color")]
        rainbow: bool,
    },
    Preset {
        name: String,
    },
    Save {
        name: String,
    },
    Restore {
        path: PathBuf,
    },
    /// Store a scrolling text animation on the keyboard (light mode 25).
    Marquee {
        /// Text to scroll: A-Z, 0-9, space, - or !.
        #[arg(default_value = "CLAUDE")]
        text: String,
        #[arg(long, value_parser=air01::lighting::parse_color, default_value = "#FF5500")]
        color: [u8; 3],
        #[arg(long, value_enum, default_value = "right")]
        direction: air01::animation::Direction,
        /// letters: one big letter at a time; scroll: one continuous banner.
        #[arg(long, value_enum, default_value = "letters")]
        style: air01::animation::Style,
        /// Milliseconds each frame (one column step) is shown.
        #[arg(long, default_value_t = 60, value_parser=clap::value_parser!(u16).range(1..))]
        delay_ms: u16,
        /// Pause on each centered letter in the letters style.
        #[arg(long, default_value_t = 700)]
        hold_ms: u16,
        #[arg(long, value_parser=clap::value_parser!(u8).range(0..5))]
        brightness: Option<u8>,
        /// Print the frames as text instead of touching the keyboard.
        #[arg(long)]
        preview: bool,
    },
}
fn marquee_cli(cli: &Cli, cmd: &LightCommand) -> Result<()> {
    use air01::{animation, lighting};
    let LightCommand::Marquee {
        text,
        color,
        direction,
        style,
        delay_ms,
        hold_ms,
        brightness,
        preview,
    } = cmd
    else {
        unreachable!()
    };
    let anim = animation::marquee(
        text,
        animation::Marquee {
            color: *color,
            direction: *direction,
            style: *style,
            delay_ms: *delay_ms,
            hold_ms: *hold_ms,
        },
    )?;
    if *preview {
        for (i, f) in anim.frames.iter().enumerate() {
            println!("frame {i}");
            print!("{}", animation::render_ascii(f));
        }
        return Ok(());
    }
    let mut b = open(cli.device.as_deref())?;
    let before = lighting::read(&mut b)?;
    if let Some(profile) = cli.profile {
        anyhow::ensure!(
            profile == before.profile,
            "lighting controls the active profile {}; activate the desired profile first",
            before.profile
        );
    }
    let target =
        animation::lighting_for(*color, brightness.unwrap_or(before.lighting.brightness()))?;
    println!(
        "Profile {}: {} {} → marquee {text:?} {} | {} frames × {delay_ms} ms | brightness {}",
        before.profile,
        before.lighting.effect(),
        before.lighting.hex(),
        target.hex(),
        anim.frames.len(),
        target.brightness()
    );
    if cli.dry_run {
        println!(
            "Dry run: would send {} reports, then select mode {}; no writes.",
            1 + anim.frames.len() * animation::PAGES as usize,
            animation::MODE
        );
        return Ok(());
    }
    let up = animation::upload(&mut b, &before, &anim, target)?;
    println!(
        "Uploaded {} reports; mode {} selected and read back. Previous lighting backup: {}",
        up.packets,
        animation::MODE,
        up.backup.display()
    );
    println!(
        "Restore it with: air01 lighting restore {}",
        up.backup.display()
    );
    Ok(())
}
fn lighting_cli(cli: &Cli, cmd: &LightCommand) -> Result<()> {
    use air01::lighting::{self, Lighting};
    if matches!(cmd, LightCommand::Marquee { .. }) {
        return marquee_cli(cli, cmd);
    }
    if matches!(cmd, LightCommand::Presets) {
        let report = lighting::library_report()?;
        for warning in report.warnings {
            eprintln!("{warning}");
        }
        for p in report.presets {
            println!(
                "{:<32} {} {}{}",
                p.name,
                p.lighting.hex(),
                p.lighting.effect(),
                if p.custom { " (saved)" } else { "" }
            );
        }
        return Ok(());
    }
    let restored = if let LightCommand::Restore { path } = cmd {
        Some(lighting::load(path)?)
    } else {
        None
    };
    let preset = if let LightCommand::Preset { name } = cmd {
        Some(
            lighting::library()?
                .into_iter()
                .find(|p| p.name.eq_ignore_ascii_case(name))
                .context("unknown preset; use lighting presets")?
                .lighting,
        )
    } else {
        None
    };
    if let LightCommand::Save { name } = cmd {
        lighting::theme_path(std::path::Path::new("."), name)?;
    }
    let mut b = open(cli.device.as_deref())?;
    let before = lighting::read(&mut b)?;
    if let Some(profile) = cli.profile {
        anyhow::ensure!(
            profile == before.profile,
            "lighting controls the active profile {}; activate the desired profile first",
            before.profile
        );
    }
    if let LightCommand::Save { name } = cmd {
        if cli.dry_run {
            println!("Dry run: would save preset {name}; no file created.");
        } else {
            println!("Saved {}", lighting::save_theme(name, &before)?.display());
        }
        return Ok(());
    }
    let target = match cmd {
        LightCommand::Show => {
            println!("{}", serde_json::to_string_pretty(&before)?);
            return Ok(());
        }
        LightCommand::Set {
            color,
            effect,
            brightness,
            speed,
            rainbow,
        } => {
            let mut target = before.lighting;
            if let Some(effect) = effect {
                target = Lighting::new(
                    *effect,
                    target.color(),
                    target.brightness(),
                    target.speed(),
                    target.rainbow(),
                )?;
            }
            if let Some(brightness) = brightness {
                target.params[2] = *brightness;
            }
            if let Some(speed) = speed {
                target.params[1] = 4 - *speed;
            }
            if let Some(color) = color {
                target = target.with_color(*color);
            }
            if *rainbow {
                target = target.with_rainbow(true);
            }
            target
        }
        LightCommand::Preset { .. } => preset.context("missing preset")?,
        LightCommand::Restore { .. } => {
            let saved = restored.context("missing lighting backup")?;
            anyhow::ensure!(
                saved.profile == before.profile,
                "backup belongs to profile {}; activate it first",
                saved.profile
            );
            saved.lighting
        }
        _ => unreachable!(),
    };
    println!(
        "Profile {}: {} {} → {} {} | brightness {} | speed {}",
        before.profile,
        before.lighting.effect(),
        before.lighting.hex(),
        target.effect(),
        target.hex(),
        target.brightness(),
        target.speed()
    );
    if cli.dry_run {
        println!("Dry run: no writes or backup files.");
    } else if let Some(path) = lighting::apply(&mut b, &before, target)? {
        println!("Read-back verified. Backup: {}", path.display());
    } else {
        println!("Already configured; no writes needed.");
    }
    Ok(())
}
fn open(path: Option<&str>) -> Result<Board<HidWire>> {
    let ds = transport::devices()?;
    let matching = ds
        .iter()
        .filter(|d| path.map_or(d.wired || d.receiver, |p| d.path == p))
        .collect::<Vec<_>>();
    // Prefer the cable when this board exposes both paths.
    let matching = if path.is_none() && matching.iter().any(|d| d.wired) {
        matching.into_iter().filter(|d| d.wired).collect::<Vec<_>>()
    } else {
        matching
    };
    let d = match matching.as_slice() {
        [d] => *d,
        [] => bail!(
            "no Air01 configuration interface; connect its USB data cable or2.4GHz receiver and wake the keyboard"
        ),
        _ => bail!("multiple boards; use --device /dev/hidrawN from `air01 devices`"),
    };
    Board::connect(HidWire::open(d)?, d.wired)
}
fn print_plan(changes: &[Change]) {
    for c in changes {
        println!(
            "profile {} {} slot {:3}: {} -> {}",
            c.profile,
            c.layer,
            c.slot,
            c.before.label(),
            c.after.label()
        );
    }
    if changes.is_empty() {
        println!("Already configured; no writes needed.");
    }
}
fn apply_cli(
    board: &mut Board<HidWire>,
    s: &Snapshot,
    changes: &[Change],
    cli: &Cli,
) -> Result<()> {
    print_plan(changes);
    if cli.dry_run {
        println!("Dry run: {} slot changes; no writes sent.", changes.len());
        return Ok(());
    }
    if changes.is_empty() {
        return Ok(());
    }
    let path = operations::auto_backup(s)?;
    println!("Backup: {}", path.display());
    let n = operations::apply(board, s, changes, cli.allow_no_fn)
        .with_context(|| format!("transaction failed; backup: {}", path.display()))?;
    println!(
        "Verified {n} slot changes. Active profile remains {}.",
        s.active_profile
    );
    Ok(())
}
fn run(cli: Cli) -> Result<()> {
    if let Command::Lighting { ref command } = cli.command {
        return lighting_cli(&cli, command);
    }
    if let Command::Devices = cli.command {
        let ds = transport::devices()?;
        if ds.is_empty() {
            println!("No Akko settings interfaces found.");
        }
        for d in ds {
            println!(
                "{} {:04x}:{:04x} {} ({})",
                d.path,
                d.vid,
                d.pid,
                d.product,
                if d.wired {
                    "wired"
                } else if d.receiver {
                    "2.4 GHz receiver"
                } else {
                    "unsupported interface"
                }
            );
        }
        return Ok(());
    }
    if matches!(cli.command, Command::Tui) {
        anyhow::ensure!(
            !cli.dry_run,
            "dry-run is not supported with tui; use CLI commands to preview changes"
        );
        anyhow::ensure!(
            cli.profile.is_none() && !cli.allow_no_fn,
            "select profiles and edit Fn from inside the TUI"
        );
    }
    if cli.dry_run
        && let Command::Backup { ref path } = cli.command
    {
        println!(
            "Dry run: would read all profiles and save {}; no file created.",
            path.display()
        );
        return Ok(());
    }
    // File validation happens before hardware discovery or I/O.
    let restore = if let Command::Restore { ref path, .. } = cli.command {
        Some(operations::load_snapshot(path).context("invalid restore snapshot")?)
    } else {
        None
    };
    let mut b = open(cli.device.as_deref())?;
    match &cli.command {
        Command::Devices | Command::Lighting { .. } => unreachable!(),
        Command::Tui => return air01::tui::run(&mut b),
        Command::Info => {
            let active = b.active_profile()?;
            println!(
                "Akko Air 01 | device {} | Gen2 | {} | firmware {:x}.{:02x} | active profile {}",
                b.device_id,
                b.link_name(),
                b.version >> 8,
                b.version & 255,
                active
            );
        }
        Command::Profiles => {
            let active = b.active_profile()?;
            for p in 0..4 {
                let m = b.matrix(p, Layer::Base)?;
                let f = m
                    .iter()
                    .enumerate()
                    .filter(|(_, a)| **a == Action::FN)
                    .map(|(i, _)| i.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                println!(
                    "{} profile {p}: slot 5 = {:12} slot 11 = {:12} Fn slots: {f}",
                    if active == p { "*" } else { " " },
                    m[5].label(),
                    m[11].label()
                );
            }
        }
        Command::Keymap { layer } => {
            let p = cli.profile.unwrap_or(b.active_profile()?);
            for (i, a) in b.matrix(p, *layer)?.iter().enumerate() {
                println!(
                    "{i:3} {:24} {:02x}:{:02x}:{:02x}:{:02x}",
                    a.label(),
                    a.0[0],
                    a.0[1],
                    a.0[2],
                    a.0[3]
                );
            }
        }
        Command::Backup { path } => {
            let s = b.snapshot()?;
            operations::save_snapshot(&s, path)?;
            println!(
                "Saved {} (four base profiles and eight Fn maps; no macros/RGB).",
                path.display()
            );
        }
        Command::Profile { index } => {
            check_profile(*index)?;
            let active = b.active_profile()?;
            if active == *index {
                println!("Profile {index} is already active.");
            } else if cli.dry_run {
                println!("Dry run: active profile {active} -> {index}; no writes sent.");
            } else {
                let s = b.snapshot()?;
                let path = operations::auto_backup(&s)?;
                println!("Backup: {}", path.display());
                operations::select_profile(&mut b, *index)?;
                println!("Active profile {index} verified.");
            }
        }
        Command::Remap {
            slot,
            action,
            layer,
        } => {
            let s = b.snapshot()?;
            let p = cli.profile.unwrap_or(s.active_profile);
            let changes = operations::plan_remap(&s, p, *layer, *slot, *action)?;
            apply_cli(&mut b, &s, &changes, &cli)?;
        }
        Command::SwapFnCtrl { all_profiles } => {
            let s = b.snapshot()?;
            let profiles = if *all_profiles {
                vec![0, 1, 2, 3]
            } else {
                vec![cli.profile.unwrap_or(s.active_profile)]
            };
            let changes = operations::plan_swap(&s, &profiles)?;
            apply_cli(&mut b, &s, &changes, &cli)?;
        }
        Command::Restore { activate, .. } => {
            let s = b.snapshot()?;
            let target = restore.context("restore snapshot missing")?;
            let changes = operations::plan_restore(&s, &target)?;
            // One backup covers both the keymap transaction and an optional profile switch.
            print_plan(&changes);
            if cli.dry_run {
                println!(
                    "Dry run: {} slot changes{}; no writes sent.",
                    changes.len(),
                    if *activate {
                        " and saved profile selection"
                    } else {
                        ""
                    }
                );
            } else if !changes.is_empty()
                || (*activate && s.active_profile != target.active_profile)
            {
                let path = operations::auto_backup(&s)?;
                println!("Backup: {}", path.display());
                operations::apply(&mut b, &s, &changes, cli.allow_no_fn)
                    .with_context(|| format!("restore failed; backup: {}", path.display()))?;
                if *activate {
                    operations::select_profile(&mut b, target.active_profile).with_context(
                        || {
                            format!(
                                "maps restored, profile selection failed; backup: {}",
                                path.display()
                            )
                        },
                    )?;
                }
                println!("Restore read-back verified.");
            }
        }
    }
    Ok(())
}
fn main() {
    if let Err(e) = run(Cli::parse()) {
        eprintln!("Error: {e:#}");
        std::process::exit(1);
    }
}
