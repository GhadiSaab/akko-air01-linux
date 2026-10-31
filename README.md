# air01

A native Rust CLI and terminal interface for configuring the **Akko Air 01 on
Linux**. Inspect profiles, remap keys including Fn, and save or restore keymaps.
No browser, cloud account, WebKit, root GUI, or kernel module.

Tested hardware: wired USB `3151:5002`, device ID **3013**, firmware `1.10`.
Other device IDs and ambiguous protocol replies are rejected; firmware versions
are not allowlisted. This is an early, independent project, not an
Akko product. There is no firmware flashing or factory-reset command.

## Why settings can appear to disappear

The keyboard has four normal profiles. A mapping saved in profile 0 does not
change profile 2. On the test board, profiles 0 and 3 contained a Ctrl/Fn swap
while active profile 2 still had the defaults. `air01 profiles` shows all four.
This explains that observation; it does not establish every possible cause of
settings loss.

## Install

With Rust 1.88 or newer:

```sh
cargo install --locked --path .
air01 info
```

NixOS:

```sh
nix build
./result/bin/air01 info
nix profile install .
```

The Nix package builds with a pinned nixpkgs input and Cargo.lock. The Linux
HID backend does not require libudev development libraries. The flake also
provides a development shell: `nix develop`.

### Device permissions

If your existing VIA/Akko rule already permits HID access, do not add another.
Otherwise, on NixOS add this to a system module and rebuild, then replug:

```nix
services.udev.extraRules = ''
  SUBSYSTEM=="hidraw", ATTRS{idVendor}=="3151", ATTRS{idProduct}=="5002", MODE="0660", TAG+="uaccess"
'';
```

Other distributions can install `udev/70-air01.rules` and reload udev. The rule
grants access to the active local session; do not run the application as root.
Close other keyboard configurators before using air01. The process lock only
coordinates other air01 instances, not browsers or unrelated applications.

## CLI

Connect a USB **data** cable and select wired mode. Writes over the receiver or
Bluetooth are not supported in this release. Changes stored on the board can
then be used wirelessly.

```sh
air01 devices
air01 info
air01 profiles
air01 --profile 2 keymap
air01 --profile 2 keymap --layer fn-win

air01 backup my-keymaps.json
air01 --dry-run swap-fn-ctrl --all-profiles
air01 swap-fn-ctrl --all-profiles

air01 --profile 2 remap 5 LeftCtrl
air01 --profile 2 remap 11 Fn
air01 --profile 2 remap 42 Escape --layer fn-win

air01 profile 0
air01 --dry-run restore my-keymaps.json
air01 restore my-keymaps.json
air01 restore my-keymaps.json --activate
```

Slots are hardware matrix positions, not visual row/column numbers. On the
verified Air 01, **5 is physical Fn and 11 is physical left Ctrl**. The swap
preset sets slot 5 to LeftCtrl and 11 to Fn, regardless of their current state.
It defaults to the active profile; `--all-profiles` makes the assignment
consistent across all four. It skips already-correct entries, including both
observed encodings of LeftCtrl. Keep another Fn assigned before replacing one.

Actions accept a key name (`A`, `F1`, `LeftCtrl`, `Escape`), `Fn`, `Disabled`, or
an exact four-byte value such as `raw:00:00:e0:00`. Raw entries are for people
who know the device format. Unknown entries are preserved in snapshots.

`--dry-run` prints the plan without configuration writes. It is not accepted
with the interactive TUI. A dry-run backup only prints its destination.
`--allow-no-fn` explicitly permits a CLI keymap transaction with no Fn key;
normal operations refuse that result.

A normal restore preserves the currently active profile. Add `--activate` to
select the snapshot's saved active profile after restoring its maps. If that
final selection fails, the error explicitly distinguishes it from the already
restored maps.

## Backups and recovery

Mutating CLI/TUI operations automatically save a private, mode-0600 backup in
`$XDG_STATE_HOME/air01/backups`, or `~/.local/state/air01/backups`. The path is
printed before any writes. Explicit backup destinations are never overwritten.

Snapshots cover **four base keymaps and eight Windows/Mac Fn maps**, plus the
active profile and device identity. They do **not** include macros, RGB,
per-key paint, sleep timers or other device settings. Restoring is therefore
limited to those keymaps and optional profile selection.

Writes are paced and read back. A failed or mismatched write triggers an
attempt to restore the affected entries. Errors distinguish verified recovery
from **INCOMPLETE** recovery and name the affected slots. If recovery cannot be
verified, close the application, reconnect the cable, inspect the profile, and
restore the printed backup. No write is blindly retried.

The transport returns an error after three seconds without a worker reply and
refuses further commands. Linux HID feature ioctls cannot be cancelled through
hidapi, so a timed-out write has an unknown outcome; air01 does not claim it was
rolled back. Reconnect before continuing. Do not run another configurator
against a timed-out device while this process is still exiting.

## Terminal interface

```sh
air01 tui
```

Use a terminal of at least 80×24. Initial reading takes about 12 seconds over
USB. Left/right previews a profile without activating it. Up/down selects a
slot. Tab cycles base, Windows Fn, and Mac Fn maps. `e` edits; `s` previews the
Ctrl/Fn preset for the viewed profile; `a` activates that profile. `b` writes a
fresh backup; `r` requests the absolute path of a snapshot to restore. Mutations
require `y` confirmation. `q` exits; Ctrl+C exits from any mode.

## Validation and development

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo run --example inspect    # read-only, requires the supported board
```

Tests cover packet layouts, identity/protocol gating, malformed snapshots,
Fn-preserving ordering, stale plans, no-op flash avoidance, send/read-back
failures, recovery failures, backup permissions, CLI dry-run behavior and TUI
navigation. Hardware checks confirmed all four base profiles after the swap,
with only the requested slots changed and Fn maps unchanged. Physical key
behavior and persistence across a power cycle must be verified by the person
using the keyboard; a successful USB read-back alone does not prove them.

Firmware `1.10` has been exercised for normal-profile remaps. Fn-map write
shapes are based on the cited Gen2 firmware/protocol research and tested with a
simulated device, but have not yet been exercised on this Air 01. Restore is
verified per slot; a crash or power loss mid-restore is not atomic. The original
snapshot remains available for recovery.

See [protocol notes](docs/PROTOCOL.md) and [attribution](NOTICE). Licensed under
[GPL-3.0-or-later](LICENSE). No repository has been published by this setup.
