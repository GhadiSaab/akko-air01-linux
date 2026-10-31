# Cohesive TUI implementation plan

> **For agentic workers:** Use superpowers:executing-plans inline.

**Goal:** Make both workspaces feel like one coherent terminal application.
**Architecture:** Shared shell/palette/navigation/footer; section separators;
consistent focus ordering and action gating, existing shared input geometry.
**Stack:** Rust/Ratatui/Crossterm. No dependencies.
**Spec:** docs/superpowers/specs/2026-10-31-cohesive-tui-design.md

## Constraints / review focus
Keep user's solid pink and keymaps untouched. Do not change terminal/window
behavior after user's correction. Retain mouse, keyboard, Custom, saved presets,
modal hit consistency and80x24 support. Focus follows displayed control order;
disabled controls must agree between shortcuts, mouse and Enter.

- [ ] Add failing focus-order/disabled-action and shared-shell layout tests;
  implement visual-order focus and enabled rules in src/tui/lighting.rs.
- [ ] Implement shared terminal-native shell in theme.rs, unify headers/footer,
  remove disconnected white cards/button blocks, flatten keycap styling.
  Render both workspaces at80x24,120x34 and odd large sizes; inspect.
- [ ] Run tests/fmt/clippy, fresh review, Nix install, read-only installed PTY
  interaction smoke. Update docs/screenshots. No HID writes.
