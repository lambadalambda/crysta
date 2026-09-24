# European timing, sound and RAM

Research for [the European timing issue](../meta/issues/european-timing.md),
measured on the reference emulator.

- **PAL.** The header's region byte `$FFD9` is 2: the European ROM runs at
  50 Hz (312 lines). Its game logic is not rescaled: walking (1, 2), the
  dash (3, 2, 2), fades (15 frames each way) and typing (a glyph a frame)
  move the same per frame as the Japanese ROM. The game only runs slower in
  real time.
- **Loads.** Loads are shorter: door `$0F`→`$10` stays dark 14 frames, not
  17, likely because a PAL frame has more CPU time. Other loads are not
  measured yet.
- **Sound.** The same driver (bootstrap `$86:AC42`, byte-identical) and song
  data: the sound bank moves from `$C6:2191` to `$C8:2191`, the track table
  from `$96:F2A0` to `$99:F9EA` (all 59 entries, pointers `+$20000` or
  more). Sample 62's European copy wraps past the end of the ROM into bank
  0. The SPC's clock is region-free, so tempo is the same in real time.
- **RAM.** The layout is the same: `$047E` map, `$0694` money, `$07ED` Prime
  Blue, `$06A4` text speed, `$0DE8`, the event flags at `$7E:06C0`, the
  actors from `$1000`. `$0450` is the current track's bank byte (`LDA
  $96:F2A2,X`), 170 in Japanese and 172 in European, not a program state.
- **Route.** The Japanese route's movement replays on the European ROM; each
  conversation needs its presses counted again (the English text has more,
  shorter pages).
