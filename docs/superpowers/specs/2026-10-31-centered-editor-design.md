# Centered, intuitive keyboard editor refinement

User requests an Apple-inspired UI/UX, a smaller central keyboard, Custom editing,
and correct physical colors. This refines the existing approved visual Rust TUI.
Builder choice: use the existing Ratatui engine with a neutral light canvas, white
surfaces, subdued separators and blue primary actions. Adapt Apple's hierarchy
and spacing rather than mimic controls the terminal cannot render.

Center a bounded workspace (max112x36) on large terminals. Put the lighting
keyboard centered above a compact preset list and grouped controls, not alongside
a tall sidebar. Keymap keyboard also gets bounded centered geometry. Keep80x24
working; identical rectangles drive drawing and mouse interaction. Center workspace
navigation and dialogs. Primary button says Apply to keyboard; drafts remain
separate from applied hardware. Secondary Save preset, Undo, Revert, Reload stay
reachable via mouse and Tab. Labels distinguish draft vs device state.

Custom is a real selectable draft entry, retained across preset switches. Editing
RGB/effect/brightness/speed/rainbow selects Custom; built-in names never label
modified values. Exact matching preset can be shown independently of browse cursor.
Preserve custom draft when refreshing saved themes.

Root cause: vendor b1d82b61.js maps yc3123_akko_air01_3m_1k to class d8, explicitly
DAZZLE=7 NORMAL=8 with COMMONCOLOR red/green/blue/orange/magenta/yellow/white.
Correct this model's encode/decode and all UI/CLI paths. Raw backups remain exact;
never migrate backup bytes. No existing user theme directory was present.
The prior readback tests proved byte transport, not visible color semantics.
No per-key rainbow arrangement will be fabricated as a faithful hardware preview;
label animated rainbow as device-controlled and show a neutral representation.
