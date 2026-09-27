# European timing, sound and RAM

Research for [the European timing issue](../meta/issues/european-timing.md),
measured on the reference emulator.

- **PAL.** The header's region byte `$FFD9` is 2: the European ROM runs at
  50 Hz (312 lines). Its game logic is not rescaled: walking (1, 2), the
  dash (3, 2, 2), fades (15 frames each way) and typing (a glyph a frame)
  move the same per frame as the Japanese ROM. The game only runs slower in
  real time.
- **Loads.** Loads are shorter: door `$0F`→`$10` stays dark 14 frames, not
  17, likely because a PAL frame has more CPU time. The portable world now
  uses 14 for that European load (with an end-to-end door test); Japanese
  remains 17. Other European loads are not measured yet.
- **Sound.** The same driver (bootstrap `$86:AC42`, byte-identical) and song
  data: the sound bank moves from `$C6:2191` to `$C8:2191`, the track table
  from `$96:F2A0` to `$99:F9EA` (all 59 entries, pointers `+$20000` or
  more). Sample 62's European copy wraps past the end of the ROM into bank
  0. The SPC's clock is region-free; the portable host polls its track script
  and alternating sound-effect latch every 640 samples at 32 kHz on PAL, not
  the Japanese 533. Native capture instead yields individual video-frame
  counts of 639–641 but 383,946 stereo frames over 600 PAL frames. A 12-second
  native bedroom track-4 envelope matches the source-only player with 512
  stereo frames of endpoint drift; a 1%-slower negative is rejected at 3,968.
  A separate CPU-to-APU write witness observes exact native pot/door effect
  pairs and bounded `F1/F0/FF/F4` route patterns. It is not PCM/DSP fidelity or
  unique spear-track identification; tracks `$34/$1C/$06` share parameter 5.
  Portable door contact is still four frames early and emits `$001A` before
  `$1200`, while native orders `$1200 → $001A → $1300`; spear return remains
  420 portable frames versus 405 native.
- **RAM.** The layout is the same: `$047E` map, `$0694` money, `$07ED` Prime
  Blue, `$06A4` text speed, `$0DE8`, the event flags at `$7E:06C0`, the
  actors from `$1000`. `$0450` is the current track's bank byte (`LDA
  $96:F2A2,X`), 170 in Japanese and 172 in European, not a program state.
- **Route.** The Japanese route's movement replays on the European ROM; each
  conversation needs its presses counted again (the English text has more,
  shorter pages). `crates/oracle/tests/local_eu_pal_movement.rs` performs an
  independent empty-SRAM native boot and compares two ordinary legs with a
  portable `World`: all 63 ordered `(map,x,y)` boundaries for 62 held-Right
  bedroom frames, then—after leaving the doorway/load interval deliberately
  unaligned—all 43 boundaries for 42 held-Down exterior frames from shared
  anchor `$10 (392,353)` to `(392,413)`. No frame shift or input-onset
  adjustment is used within either leg. This does not qualify other movement,
  animation, map loads or host wall-clock delivery. The native frozen-return
  script releases control at `(136,464)`; later manual Left/Up reaches
  `(120,448)`. Portable control releases at `(136,368)`, proving a missing
  96-pixel scripted descent rather than a discrepancy in the later input.
