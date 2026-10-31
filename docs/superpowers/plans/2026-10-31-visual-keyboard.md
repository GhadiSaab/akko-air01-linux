# Visual Air 01 TUI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Continue directly under the user's explicit request to replace the slot-list UI with a physical keyboard editor; the existing approved design already permits an accurate keyboard layout.

**Goal:** Draw the 84-key Air 01 with clickable keycaps and edit assignments directly.
**Architecture:** Ratatui, already used by the application, renders a custom keyboard widget. One geometry model drives drawing, hit testing and spatial navigation. The existing verified operations remain unchanged.
**Tech Stack:** Rust, Ratatui 0.30, Crossterm, existing protocol/backend.
**Spec:** Existing Air01 design plus the user's explicit visual-keyboard correction. No new hardware capability is requested.

## Design and decisions

Use the exact physical ANSI Air01 geometry. Render Fn, Ctrl, Option, Command,
space and arrow-key positions from the official photo, cross-referenced to
vendor coordinates and device3013 matrix positions. All 84 positions are known,
including Fn at5 and lighting at87 (raw action 0a:09:00:00 on the captured board).
The interface shows physical legends and current assignments; remapped keys
are amber and the selected key is mint. A compact rendering fits 80x24, with
larger two-line keycaps when there is room. Smaller terminals show a resize
message and cannot start edits.

Click a key or use spatial arrow navigation and Enter to open a searchable
assignment picker. Click/Enter an assignment to preview the existing verified
transaction. Apply/Cancel are clickable; confirmation preserves backup and
read-back safeguards. Profile and layer tabs are clickable; profile preview
never activates hardware. PgUp/PgDn also preview profiles. Tab cycles layers.
Advanced raw input remains available in the picker. Mouse capture is restored
on every exit. Preserve backup/restore, Ctrl/Fn preset, and profile activation.

Approaches: keep Ratatui and build the correct custom widget (chosen because it
already supports mouse/custom widgets); changing to another engine adds a full
rewrite without solving physical mapping. A terminal image of the keyboard
would look photographic but requires graphics-protocol support and has less
legible assignment overlays, so use terminal-native keycaps.

## Review focus

Wrong physical-to-matrix mapping; mouse targets diverging from drawing after
resize; accidental writes during navigation; transparent Fn entries; terminal
mouse/raw-state cleanup. Hardware configuration must not be modified for UI
validation; use existing current settings and simulated transactions.

### Task 1: Keyboard model and spatial navigation

Files: src/keyboard.rs, tests/keyboard.rs.
- [x] Test 84 unique positions, Fn/Ctrl/arrow/lighting locations, scaling at80x24/120x36/160x50, hit testing and geometric navigation; observe failures.
- [x] Implement factual layout data and pure geometry/navigation functions.
- [x] Run tests and commit.

### Task 2: Visual keyboard and direct editing

Files: src/tui.rs, tests/tui.rs.
- [x] Test physical navigation, mouse profile/layer/key selection, assignment filtering/selection, cancellation, confirmation, minimum-size and rendered keycaps; observe failures.
- [x] Render keycaps, inspector, tabs, searchable picker and confirmation popups with shared hit geometry.
- [x] Enable/disable mouse capture with terminal guard. Route mouse/key intents through existing backup/fresh-snapshot/verified-apply path.
- [x] Verify tests, formatting, Clippy and a read-only PTY/rendered screenshot; commit.

### Task 3: Review, package and install

Files: README.md, NOTICE, tests as required.
- [x] Document new controls and layout evidence.
- [x] Request independent read-only review and fix material findings with regressions.
- [x] Build Nix package, install through existing user-local package link, verify installed CLI and read-only TUI; commit. No keyboard writes required.
