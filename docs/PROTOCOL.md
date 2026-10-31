# Air 01 protocol evidence

Only wired USB device 3013 (`3151:5002`) is enabled for configuration writes.
The USB product and internal `yc3123` name are insufficient to select commands.

The test board answered `0x89` with an opcode echo/options and `0x8A` with a
plausible keymatrix. An incompatible sleep-layout response is also checked.
Anything ambiguous is rejected before writes. Identification bytes 7/8 were
`10 01`, displayed as packed-hex version `1.10`.

| Operation | Gen2 packet bytes, excluding report ID | Evidence |
|---|---|---|
| identify | `8F …` | Air 01 hardware |
| active profile read | `84 …` | Air 01 hardware |
| base map page | `8A profile FF page 00 …` | Air 01 hardware |
| Fn map page | `90 os profile FF page …` | Air 01 hardware read |
| base slot write | `0A profile slot 00 00 01 00 checksum` + value at 8..12 | Air 01 hardware, profiles 1/2 slots 5/11 |
| Fn slot write | `10 os profile slot …` + value at 8..12 | Gen2 firmware research; not yet Air 01 hardware |
| select profile | `04 profile …` | Air 01 hardware: 2 → 3 → 2 |

Every feature buffer is 65 bytes: report ID 0 plus a 64-byte packet. Header
checksum at packet byte 7 is `FF - (sum(bytes 0..6) & FF)`. Four action bytes
sit at packet offsets 8..11. Keymap reads contain eight raw 64-byte pages.
Each map has 128 four-byte entries. OS 0 is Windows and OS 1 is Mac.

`0A 01 00 00` is Fn. Both `00 E0 00 00` and `00 00 E0 00` were observed as
LeftCtrl. Preserve raw bytes, including unknown tags and both Ctrl forms.

Read-only pre-change snapshots showed the active profile was 2. Profiles 0
and 3 had the swap; 1 and 2 did not. After applying the swap through this Rust
tool, a complete comparison found only slots 5 and 11 changed in profiles 1
and 2. All Windows/Mac Fn maps and all other base slots were identical. The
active profile remained 2. Onboard persistence after power loss remains a
separate physical check.

This is a userspace settings tool, not replacement keyboard firmware or a
Linux kernel driver. No reset, bootloader, calibration, or firmware commands
are exposed. See NOTICE for upstream research and source revisions.

## Global lighting (Air 01 wired Gen2)

Read `87` using the usual Bit7 request checksum. Reply bytes0..7 are
`87 mode inverse-speed brightness flags R G B`. Write `07` followed by those
seven parameter bytes and a **Bit8** checksum at report byte8 (byte9 with report
ID prefix), sum255 over opcode, payload and checksum. Speed wire=4-host,
brightness0..4. Air01 flags low nibble8=fixed RGB,7=rainbow; 0..6 are firmware
preset colors. High nibble is the effect option; Air01 Wave defaults to option2
(down). White uses FAFAFA. Raw bytes are preserved for backup and restoration.

Effect IDs:1 Static,2 Breathing,3 Spectrum,4 Wave,5 Ripple,6 Stars,7 Snake,
8 Reactive,9 Converge,10 Sine wave,11 Kaleidoscope,12 Line wave,14 Laser,
15 Circle,16 Dazzle,17 Rain,18 Meteor,19 Reactive off. These mode numbers come
from upstream protocol and vendor metadata; Static RGB, Breathing and rainbow Wave writes/readbacks have been
exercised on this board, followed by exact original lighting restoration. Host-fed modes and per-key uploads
are excluded. No side-light writes.

Lighting transactions recheck device3013 and active profile, reject stale state,
create an exact separate JSON backup, pace the write by1second and read back.
On failure, recovery requires the same profile and current bytes matching either
original or attempted values; external changes are never blindly overwritten.

### Air01 color correction

The vendor build `js/b1d82b61.js` maps `yc3123_akko_air01_3m_1k` to
`class d8 extends u`, which explicitly overrides `DAZZLE=7;NORMAL=8`.
The original implementation incorrectly used the generic Gen2 pair (reversed).
This produced rainbow instead of fixed RGB even though exact byte readback
passed. The user confirmed corrected `01 02 04 08 FF 00 00` renders entirely
red. They subsequently requested solid pink `#FF69B4`; it remains applied.
The Air01 palette indices are red, green, blue, orange `FF5500`, purple
`7700FF`, yellow and white. Index selection overrides otherwise-unused RGB bytes.
Raw snapshots preserve wire values exactly and are never silently converted.
