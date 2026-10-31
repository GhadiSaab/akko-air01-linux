# Cohesive terminal layout refinement

User finds the UI/layout clunky and not seamless, and asks to study other TUIs.
Their clarification rejects an assumption about fullscreen/window behavior.
Do not alter the launcher, compositor, terminal sizing or keyboard lighting.
Keep the requested smaller centered keyboard and correct Custom editing.

Reference: Yazi and Lazygit primary docs/repositories. Builder choice: terminal-native
surface using the terminal background, one bounded outer frame, one header with
workspace tabs, shared section separators and compact contextual shortcut footer.
Remove white cards and floating faux-desktop buttons. Keep one blue accent/focus
indicator, not simultaneous preset highlight and control focus. Both keymap and
lighting use the same shell and flat, spaced keycaps. Keyboard remains bounded.
Borders separate real sections, not each control. Keep all existing features.

Focus should follow visual reading order: preset, effect, multicolor, RGB,
brightness, speed, hex, apply/save/undo/revert/reload. Skip disabled actions and
static-mode speed. Mouse and keyboard must use the same disabled rules and layout.
A focused disabled control must not masquerade as an enabled action. Preserve
modal geometry on odd large sizes and complete compact80x24 usability.
No HID protocol, profile, theme-file or persistence changes. Existing app process
may remain open; never terminate the user's draft to test the new version.
