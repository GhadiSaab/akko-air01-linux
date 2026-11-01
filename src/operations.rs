//! Plans are immutable and include the old value. All preconditions are checked
//! before the first write; failures report whether recovery was verified.
use crate::{
    device::{Board, Layer},
    protocol::{Action, check_profile, check_slot, packet},
    snapshot::{ProfileSnapshot, Snapshot},
    transport::{Wire, state_dir},
};
use anyhow::{Context, Result, bail, ensure};
use std::{
    collections::HashSet,
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub profile: u8,
    pub layer: Layer,
    pub slot: u8,
    pub before: Action,
    pub after: Action,
}
pub fn entries(p: &ProfileSnapshot, l: Layer) -> &[Action] {
    match l {
        Layer::Base => &p.base,
        Layer::FnWin => &p.fn_win,
        Layer::FnMac => &p.fn_mac,
    }
}
pub fn plan_remap(
    s: &Snapshot,
    profile: u8,
    layer: Layer,
    slot: u8,
    action: Action,
) -> Result<Vec<Change>> {
    s.validate()?;
    check_profile(profile)?;
    check_slot(slot)?;
    let before = entries(s.profile(profile)?, layer)[slot as usize];
    Ok(if before == action {
        vec![]
    } else {
        vec![Change {
            profile,
            layer,
            slot,
            before,
            after: action,
        }]
    })
}
pub fn plan_swap(s: &Snapshot, profiles: &[u8]) -> Result<Vec<Change>> {
    s.validate()?;
    let mut out = vec![];
    let mut seen = HashSet::new();
    for &p in profiles {
        ensure!(seen.insert(p), "duplicate target profile {p}");
        out.extend(plan_remap(s, p, Layer::Base, 11, Action::FN)?);
        if s.profile(p)?.base[5].label() != "LeftCtrl" {
            out.extend(plan_remap(s, p, Layer::Base, 5, Action::CTRL)?);
        }
    }
    Ok(out)
}
pub fn plan_restore(before: &Snapshot, after: &Snapshot) -> Result<Vec<Change>> {
    before.validate()?;
    after.validate()?;
    ensure!(
        before.device_id == after.device_id,
        "snapshot identity mismatch"
    );
    let mut out = vec![];
    for pr in 0..4 {
        for layer in [Layer::Base, Layer::FnWin, Layer::FnMac] {
            for slot in 0..128 {
                out.extend(plan_remap(
                    before,
                    pr,
                    layer,
                    slot,
                    entries(after.profile(pr)?, layer)[slot as usize],
                )?);
            }
        }
    }
    Ok(out)
}
fn change_packet(c: &Change, value: Action) -> Result<[u8; 65]> {
    check_profile(c.profile)?;
    check_slot(c.slot)?;
    match c.layer {
        Layer::Base => packet(0x0a, &[c.profile, c.slot, 0, 0, 1, 0], Some(value)),
        Layer::FnWin => packet(0x10, &[0, c.profile, c.slot], Some(value)),
        Layer::FnMac => packet(0x10, &[1, c.profile, c.slot], Some(value)),
    }
}
fn read_entry<W: Wire>(b: &mut Board<W>, c: &Change) -> Result<Action> {
    let r = b.page(c.profile, c.layer, c.slot / 16)?;
    let at = (c.slot % 16) as usize * 4;
    Ok(Action(r[at..at + 4].try_into()?))
}
fn write_verify<W: Wire>(b: &mut Board<W>, c: &Change, value: Action) -> Result<()> {
    ensure!(
        !b.receiver || b.active_profile()? == c.profile,
        "receiver target profile is no longer active; recovery/write stopped"
    );
    let result = b.wire.send(change_packet(c, value)?);
    // A send error may still have reached the device; allow the flash endpoint to settle.
    b.wire.wait(Duration::from_millis(500));
    result?;
    ensure!(
        read_entry(b, c)? == value,
        "read-back mismatch at profile {} {} slot {}",
        c.profile,
        c.layer,
        c.slot
    );
    Ok(())
}
pub fn apply<W: Wire>(
    b: &mut Board<W>,
    s: &Snapshot,
    changes: &[Change],
    allow_no_fn: bool,
) -> Result<usize> {
    s.validate()?;
    ensure!(
        b.can_configure(),
        "configuration requires USB or a verified2.4GHz receiver"
    );
    ensure!(
        !b.receiver || changes.iter().all(|c| c.profile == s.active_profile),
        "over2.4GHz, remap only the active profile; activate another profile first or use USB for all-profile changes"
    );
    b.check_identity()?;
    ensure!(
        b.active_profile()? == s.active_profile,
        "active profile changed; read a new snapshot"
    );
    let mut coordinates = HashSet::new();
    let mut affected = HashSet::new();
    for c in changes {
        check_profile(c.profile)?;
        check_slot(c.slot)?;
        ensure!(c.before != c.after, "plan contains an unchanged slot");
        ensure!(
            coordinates.insert((c.profile, c.layer as u8, c.slot)),
            "duplicate target slot"
        );
        ensure!(
            entries(s.profile(c.profile)?, c.layer)[c.slot as usize] == c.before,
            "plan original value disagrees with backup"
        );
        affected.insert(c.profile);
    }
    for pr in affected {
        let mut final_base = b.matrix(pr, Layer::Base)?;
        for c in changes.iter().filter(|c| c.profile == pr) {
            let live = if c.layer == Layer::Base {
                final_base[c.slot as usize]
            } else {
                read_entry(b, c)?
            };
            ensure!(
                live == c.before,
                "stale keymap at profile {} {} slot {}; no writes sent",
                pr,
                c.layer,
                c.slot
            );
        }
        for c in changes
            .iter()
            .filter(|c| c.profile == pr && c.layer == Layer::Base)
        {
            final_base[c.slot as usize] = c.after;
        }
        ensure!(
            allow_no_fn || final_base.contains(&Action::FN),
            "profile {pr} would have no Fn key; assign a replacement Fn first (or explicitly allow no Fn)"
        );
    }
    let mut ordered = changes.to_vec();
    ordered.sort_by_key(|c| {
        if c.layer == Layer::Base && c.after == Action::FN {
            0
        } else if c.layer == Layer::Base && c.before == Action::FN {
            2
        } else {
            1
        }
    });
    let mut attempted = Vec::<Change>::new();
    let outcome = (|| -> Result<()> {
        for c in &ordered {
            ensure!(
                b.active_profile()? == s.active_profile,
                "active profile changed during transaction"
            );
            attempted.push(c.clone());
            write_verify(b, c, c.after)?;
        }
        ensure!(
            b.active_profile()? == s.active_profile,
            "active profile changed during transaction"
        );
        Ok(())
    })();
    if let Err(cause) = outcome {
        let mut failures = vec![];
        for c in attempted.iter().rev() {
            let recovery = (|| -> Result<()> {
                b.check_identity()?;
                let now = read_entry(b, c)?;
                if now == c.before {
                    return Ok(());
                }
                ensure!(
                    now == c.after,
                    "slot changed externally; refusing to overwrite it"
                );
                if c.layer == Layer::Base
                    && now == Action::FN
                    && c.before != Action::FN
                    && s.profile(c.profile)?.base.contains(&Action::FN)
                {
                    let live = b.matrix(c.profile, Layer::Base)?;
                    ensure!(
                        live.iter()
                            .enumerate()
                            .any(|(i, a)| i != c.slot as usize && *a == Action::FN),
                        "retaining replacement Fn because another Fn could not be positively verified"
                    );
                }
                write_verify(b, c, c.before)
            })();
            if let Err(e) = recovery {
                failures.push(format!(
                    "profile {} {} slot {}: {e}",
                    c.profile, c.layer, c.slot
                ));
            }
        }
        if failures.is_empty() {
            bail!(
                "apply failed: {cause}; rollback verified for {} attempted slots",
                attempted.len()
            );
        }
        bail!(
            "apply failed: {cause}; rollback INCOMPLETE: {}. Restore from the printed backup after reconnecting",
            failures.join("; ")
        );
    }
    Ok(changes.len())
}
pub fn select_profile<W: Wire>(b: &mut Board<W>, p: u8) -> Result<()> {
    check_profile(p)?;
    ensure!(
        b.can_configure(),
        "profile changes require USB or a verified2.4GHz receiver"
    );
    b.check_identity()?;
    let old = b.active_profile()?;
    if old == p {
        return Ok(());
    }
    let change = (|| -> Result<()> {
        let result = b.wire.send(packet(0x04, &[p], None)?);
        b.wire.wait(Duration::from_secs(1));
        result?;
        ensure!(
            b.active_profile()? == p,
            "profile selection did not read back"
        );
        Ok(())
    })();
    if let Err(cause) = change {
        let recovery = (|| -> Result<()> {
            b.check_identity()?;
            if b.active_profile()? != old {
                let result = b.wire.send(packet(0x04, &[old], None)?);
                b.wire.wait(Duration::from_secs(1));
                result?;
            }
            ensure!(b.active_profile()? == old, "profile rollback mismatch");
            Ok(())
        })();
        match recovery {
            Ok(()) => bail!("profile selection failed: {cause}; original profile restored"),
            Err(e) => bail!("profile selection failed: {cause}; rollback INCOMPLETE: {e}"),
        }
    }
    Ok(())
}
pub fn save_snapshot(s: &Snapshot, path: &Path) -> Result<()> {
    s.validate()?;
    use std::os::unix::fs::OpenOptionsExt;
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .with_context(|| {
            format!(
                "cannot create backup {}; existing files are never overwritten",
                path.display()
            )
        })?;
    serde_json::to_writer_pretty(&mut f, s)?;
    writeln!(f)?;
    f.sync_all()?;
    Ok(())
}
pub fn load_snapshot(path: &Path) -> Result<Snapshot> {
    let f = std::fs::File::open(path)?;
    ensure!(f.metadata()?.len() <= 1024 * 1024, "snapshot exceeds 1 MiB");
    let s: Snapshot = serde_json::from_reader(f)?;
    s.validate()?;
    Ok(s)
}
pub fn auto_backup(s: &Snapshot) -> Result<PathBuf> {
    let dir = state_dir()?.join("backups");
    std::fs::create_dir_all(&dir)?;
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let path = dir.join(format!("air01-{stamp}.json"));
    save_snapshot(s, &path)?;
    Ok(path)
}

/// Refresh a long-lived UI plan before creating its automatic recovery snapshot.
pub fn fresh_for_plan<W: Wire>(
    board: &mut Board<W>,
    expected: &Snapshot,
    changes: &[Change],
) -> Result<Snapshot> {
    expected.validate()?;
    let fresh = board.snapshot()?;
    ensure!(
        fresh.active_profile == expected.active_profile,
        "active profile changed while confirmation was pending; read a new plan"
    );
    for c in changes {
        check_profile(c.profile)?;
        check_slot(c.slot)?;
        ensure!(
            entries(expected.profile(c.profile)?, c.layer)[c.slot as usize] == c.before,
            "plan original value disagrees with cached snapshot"
        );
        ensure!(
            entries(fresh.profile(c.profile)?, c.layer)[c.slot as usize] == c.before,
            "target changed while confirmation was pending; read a new plan"
        );
    }
    Ok(fresh)
}
