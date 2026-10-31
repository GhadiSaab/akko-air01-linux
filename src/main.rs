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
fn open(path: Option<&str>) -> Result<Board<HidWire>> {
    let ds = transport::devices()?;
    let matching = ds
        .iter()
        .filter(|d| path.map_or(d.wired, |p| d.path == p))
        .collect::<Vec<_>>();
    let d = match matching.as_slice() {
        [d] => *d,
        [] => bail!("no matching wired Air 01; connect its USB data cable and select wired mode"),
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
                } else {
                    "read-only / use USB cable"
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
        Command::Devices => unreachable!(),
        Command::Tui => return air01::tui::run(&mut b),
        Command::Info => {
            let active = b.active_profile()?;
            println!(
                "Akko Air 01 | device {} | Gen2 | USB | firmware {:x}.{:02x} | active profile {}",
                b.device_id,
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
