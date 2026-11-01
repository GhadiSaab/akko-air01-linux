# Air01 receiver configuration

User explicitly requests2.4GHz CLI/TUI configuration before public GitHub release.
Actual receiver3151:5006 exposes usageFFFF:2. Read-only probe reaches device3013,
Gen2 firmware1.10, active profile1, fixed pink RGB and keymap/settings via relay.

Implement relay conversation F7 status, F6 keyboard target10, normal packet,
F7 reply-ready polls, FC release, read. Per-packet pacing20ms, bounded polls;
worker timeout5s on receiver and poison after timeout. Reject absent/unpaired/
asleep/boot receiver before keyboard packets. Do not blindly resend writes.
Only known USB3151:5002 wired or5006 receiver may open. Board identification and
Gen2 probing remain mandatory. Ordinary profile/lighting/keymap operations share
existing snapshots, stale guards, readback and rollback. Receiver keymap writes
are limited to the active profile: inactive writes are not established. Animation
uploads remain wired only. Bluetooth remains unsupported.

CLI prefers cable when both links are attached, else receiver. Explicit --device
works for supported interfaces. Devices/info and TUI display2.4GHz honestly.
Keep source compatibility for test/fake Wire via default is_receiver=false.
Test the relay with fake raw reports covering busy/offline/short/no-reply states,
keyboard target, report/checksum layouts and one-send semantics. Then read full
backup on actual receiver, temporary lighting/base/Fn/profile changes and exact
restore, verify unchanged full snapshots, installed TUI smoke. Preserve actual
starting settings; no firmware/reset/pairing commands.
