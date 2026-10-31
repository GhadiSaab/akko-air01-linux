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
