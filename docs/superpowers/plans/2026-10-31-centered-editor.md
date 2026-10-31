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

- [ ] Red tests: fixed/rainbow flags, Air01 preset decode and golden packets;
  implement shared model constants/methods for CLI and TUI. Verify unit tests.
- [ ] Red tests: Custom entry, edited preset labeling and retained custom draft;
  implement state transitions and refresh preservation.
- [ ] Red tests: bounded centered geometry, nav hits, all sliders/buttons at80x24
  and large offsets; implement common theme and redesigned lighting/keymap views.
- [ ] Render and inspect80x24/120x34/large window; fix layout issues; run all tests,
  fmt/Clippy; fresh whole-feature review. Corrected red/green/blue hardware probe,
  user visual confirmation, original restore and keymap preservation. Nix install
  and installed TUI smoke test; document and commit evidence.
