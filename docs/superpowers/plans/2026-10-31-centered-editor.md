# Centered custom editor implementation plan

> **For agentic workers:** Use superpowers:executing-plans inline.

**Goal:** Correct Air01 color semantics and deliver a centered, intuitive editor.
**Architecture:** Model-specific lighting constants, persistent custom draft state,
shared bounded shell geometry and palette, renderer/input using the same layout.
**Tech Stack:** Existing Rust/Ratatui/Crossterm; no dependencies.
**Spec:** docs/superpowers/specs/2026-10-31-centered-editor-design.md

## Constraints and review focus
Wire7=rainbow,8=fixed only for device3013. Preserve raw backups, keymaps and active
profile. Distinguish byte verification from physical visual confirmation. All
controls must remain reachable at80x24 and centered with nonzero origins. No fake
rainbow key coloring. Custom draft must survive preset changes and library refresh.

- [x] Red tests: fixed/rainbow flags, Air01 preset decode and golden packets;
  implement shared model constants/methods for CLI and TUI. Verify unit tests.
- [x] Red tests: Custom entry, edited preset labeling and retained custom draft;
  implement state transitions and refresh preservation.
- [x] Red tests: bounded centered geometry, nav hits, all sliders/buttons at80x24
  and large offsets; implement common theme and redesigned lighting/keymap views.
- [ ] Render and inspect80x24/120x34/large window; fix layout issues; run all tests,
  fmt/Clippy; fresh whole-feature review. Corrected red hardware probe,
  user visual confirmation and requested pink setting; keymap preservation. Nix install
  and installed TUI smoke test; document and commit evidence.

## Review and evidence
- Fresh reviewer found one Important issue: odd terminal sizes shifted modal
  hit targets by a cell. Regression clicks the rendered Apply/Cancel labels at
  160x51,161x50,161x51; normalized all modal geometry to the bounded workspace.
- Reclaimed a status row in compact keymap mode to avoid clipping instructions.
- User confirmed corrected fixed-color packet renders solid red. Restored the
  prior raw lighting, then applied the user's new explicit request: solid pink
  #FF69B4, active profile1, brightness4. Leave pink in place.
- Full base/Fn snapshot equals the earlier snapshot: all four base and eight Fn
  maps unchanged. No new profile selection or keymap writes.
