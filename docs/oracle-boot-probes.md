# Boot-Flow Probe Findings

Recorded 2026-08-26 from scratch probes (`crates/oracle/examples/probe_boot*.rs`,
removed after this write-up) against the local dumps in `local/`. These are
ROM-backed observations; nothing here is committed as fixture content.

## Boot timeline (Japan `Tenchi Souzou (Japan).sfc`, LoROM, NTSC)

All timings are frame counts from hard reset with the input edges shown.

| Frame | Input | Observed |
| --- | --- | --- |
| 0–399 | — | Title screen (state `0x0450` = 163) |
| 400 | Start press (8 frames) | Title dismissed; map 4 (name entry, has "NAME" header on EU) at ~449 |
| 1500 | A press | Needed: without it, later Start does nothing; selects a kana cell |
| 1800 | Start press | Name accepted, map 41 (`0x29`) at ~1882, ~95 frames, then map 15 (`0x0F`) at ~2946 |
| 2946+ | any | **Stall**: state `0x0450` = 170, player frozen at (304,112) |

The name buffer already holds the default name before any input
(`D0 3B 73 42 D1 D4` = アーク in the game's 2-byte encoding).

## Stall signature

- `0x047E` = 15 (current map), `0x0482` = 41 (pending map), `0x0480` = 30
  (current map × 2), `0x0488` = 0x0129 (room-change timer, frozen).
- The 15→41 map transition never completes; the game sits in "pending 41" with
  the room-change timer stuck. States: `163 (title) → 198 (name) → 174 → 157
  (map 41) → 170 (stuck)`.
- Input is *read* (WRAM mirrors react: `0x0456` A-bit `0x80`, `0x0920`
  down-bit, `0x0DC8` menu cursor moves with Down) but nothing advances.
- Identical stall on the European dump, except EU is force-blanked (black
  screen) during it; JP renders the name/kana grid with a dialog.
- 30,000+ idle frames change nothing. Not audio-driven, not the APU catching
  up, not text speed (`0x06A4`), not fade timing.

## Eliminated hypotheses

- Wrong button; missing confirm; name not accepted (name bytes refute).
- Input dropped by the harness (mirrors prove the game reads it).
- Slow text speed / long cutscene (idle runs are far longer than any scene).
- SRAM/battery wait, decompress/DMA resource wait (nothing ever arrives).

## Leading hypothesis

The room-change routine waits on a frame-bound handshake that instant
frame-stepping breaks — e.g. a `$4210` vblank-end poll whose NMI semantics
differ, or an SPC handshake over `$2140–2143` where the CPU never sees the
reply because LakeSnes catches the APU up only at frame end. Next step:
sample the CPU PC (shim accessor `snes_cpu_pc`/`snes_cpu_bank`) at the stuck
state and check whether it spins in a tight polling loop.

## Reusable RAM symbols (probe-derived)

- `0x0450` program state, `0x047E` current map, `0x0480` map × 2,
  `0x0482` pending map, `0x0488` room-change timer, `0x0456` button mirror
  (A = 0x80), `0x0920` secondary input mirror, `0x0DC8` menu cursor,
  `0x06A4` text speed.