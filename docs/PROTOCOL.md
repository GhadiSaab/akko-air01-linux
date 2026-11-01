# Air 01 protocol evidence

Device3013 is enabled through USB `3151:5002` and the verified2.4GHz receiver
`3151:5006`; receiver remaps target only its active profile.
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
| Fn slot write | `10 os profile slot …` + value at 8..12 | Air01 receiver hardware: Windows and Mac Fn slots, restored afterward |
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

## Per-key animation (light mode 25)

The vendor Gen2 serializer (`setUserGifStart`/`setUserGif`,
`js/4abb36b7.js`) uploads a frame sequence that the firmware plays itself as
`LightUserColor`, mode 25. The Air01 vendor record does not list this mode, but
device 3013 firmware 1.10 accepted and read back `19 00 04 08 FF 55 00`, and the
uploaded frames were displayed (verified on hardware by eye).

| step | Bit7 packet bytes 0..6 | data |
|---|---|---|
| start | `12 00 00 00 00 00 00` | none; wait 500 ms |
| frame page | `12 frame page 01 count delay_lo delay_hi` | 56 RGB bytes at 8.. |

Each frame is seven pages: 128 RGB triples in matrix-slot order (the keymap
slot numbers), zero padded to 392 bytes. Frame index and count are bytes, so at
most 255 frames. Delay is a little-endian u16 per frame; air01 uses one value
for all frames and repeats frames for pauses. The vendor paces pages 5 ms apart;
air01 uses 10 ms. There is no read command for the frames. Note that
monsgeek-akko-linux names `0x18` "SET_USERGIFSTART"; for this family the vendor
code sends `0x12` with byte 3 = 0 instead, and that is what air01 sends.

## Air01 2.4GHz receiver (3151:5006)

Same vendor usageFFFF:2, feature65bytes including reportID0. Status F7, target
select F6/10, normal keyboard packet, F7 reply-ready poll, FC release/read.
Status fields:0 reply ready,1 battery percent,3 keyboard online when0,5 can-send,
6 kind1=keyboard/3=combined. Air01 status tail can retain old bytes (observed
109 atbyte7); nonzero tail bytes alone do not mean RF boot.

Session selects keyboard once, leaves200ms quiet time between commands, waits
100ms before checking a keyboard reply and bounds polls. One read retry is
allowed on missing replies, never an automatic repeat of a configuration write.
Pending replies are consumed before the next command. No idle polling.
Worker timeout15seconds poisons further commands, as with USB's3second timeout.

All four base and eight Fn maps read back identically to USB. RGB/brightness,
base/Windows-Fn/Mac-Fn active-profile writes and profile selection were verified
and restored. Inactive-profile writes remain prohibited on receiver; animation
0x12 bulk uploads remain USB-only. Typing latency during configuration traffic
was observed; it stops when traffic stops. Use USB for large/frequent transfers.
