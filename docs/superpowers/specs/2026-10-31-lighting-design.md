# Visual lighting workspace

User requests RGB presets and custom colors, fully usable and pleasing TUI/UX.
Continue the approved Rust/Ratatui application. Builder choice: dedicated Lighting
workspace alongside the physical keymap, with a keyboard color preview, preset
library, RGB/hex editor, brightness/speed/effect controls, and named saved presets.
Mouse and keyboard must reach every control; controls share hit-test geometry.
Draft edits do not write hardware. Apply explicitly commits, backs up exact
original LED parameters, checks identity/profile/staleness, paces writes, verifies
readback and attempts verified recovery on failure. Undo restores last applied
lighting. Keymaps remain untouched. Lighting belongs to the active board profile;
keymap preview profile must not silently redirect it. Switching workspaces keeps
drafts. Small terminals show resize guidance. Errors remain visible.

Approaches: extend a modal (cramped), external CLI (fails visual usability), or
integrated workspace (chosen by builder under the user's continuation request).
Use current Ratatui engine; no new dependencies. Presets store one global RGB/effect
setup, not per-key paint. Unsupported host-fed music/screen/per-key modes are not
selectable. Current unknown parameters are preserved exactly for recovery.
Backups and saved presets are separate versioned JSON, keeping keymap schema1
compatible. No firmware/reset operations. Hardware tests restore original lighting.
