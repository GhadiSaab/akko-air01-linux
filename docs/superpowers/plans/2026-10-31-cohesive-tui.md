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

- [x] Add failing focus-order/disabled-action and shared-shell layout tests;
  implement visual-order focus and enabled rules in src/tui/lighting.rs.
- [x] Implement shared terminal-native shell in theme.rs, unify headers/footer,
  remove disconnected white cards/button blocks, flatten keycap styling.
  Render both workspaces at80x24,120x34 and odd large sizes; inspect.
- [x] Run tests/fmt/clippy, fresh review, Nix install, read-only installed PTY
  interaction smoke. Update docs/screenshots. No HID writes.

## Review / rulings
- Fresh reviewer found one Important issue: content overwrote the shared footer
  divider. Reserved bottom-4 outside content, moved keymap status into the shared
  status row, and added full buffer assertions for both views at80x24/120x34/161x51.
- Restored contextual footer shortcuts and explicit readable text on fixed dark
  keycaps, including light-default-foreground terminals. Disabled mouse actions
  preserve the prior focus, and bright RGB previews use contrasting dark legends.
- Ruling: do not run a second real-HID TUI while the user's existing session holds
  the device lock. Do not terminate it or discard its draft. Verify rendering/input
  with TestBackend, all tests, Clippy and Nix checks; installed CLI checks remain
  hardware-free. The existing terminal lifecycle/transport is unchanged.

## Delivery
- 72 tests pass; fmt check and Clippy --all-targets -D warnings pass.
- Rendered/inspected both workspaces, including80x24 and120x34. Odd large
  modal tests and shared footer-continuity assertions pass.
- Nix build and its checks succeeded; existing air01 executable/launcher now
  resolves to the shared-shell build. Installed --version and lighting --help pass.
- No hardware writes and no terminal/compositor/launcher changes in this task.
- Another session switched the shared checkout to feat/claude-lighting-animation
  after f1157e3; baf6eb0 landed there. Preserved that checkout and retained both
  UI commits on feat/cohesive-tui; delivery notes finalized in a separate worktree.
