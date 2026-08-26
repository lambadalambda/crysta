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
- CPU deadlock or crash (CPU cycles its main loop, see below).

## Root cause: SPC driver-upload handshake wedge (confirmed 2026-08-26)

The stall is a two-sided CPU↔SPC protocol deadlock during the first audio
load after name entry, not game logic and not a crash:

- The CPU is pinned in the packet-send loop at `$86:AB4C`:
  `CMP $2140 / BNE spin / INC A / STA $2140` — it waits for the SPC to echo
  the byte it just wrote to `$2140`.
- The SPC boots fine (IPL at `$FFC0`: echoes `$AA/$BB/$CC`, uploads the
  driver via the word-oriented handshake on `$2140/$2141`).
- The word stream observed mid-transfer: ack-count lows
  `00,01,02,…` with data highs `13 08 CA 0B 72 07 …` — a raw driver chunk.
- The IPL restarts its handshake phase (back to writing `$AA/$BB` and
  waiting for `$F4 == $CC`) whenever a consumed word's data byte is
  nonzero. The game only writes the `$CC` chunk marker once per chunk, so
  once the IPL lands in its `$CC` wait while the CPU waits for the next
  echo, neither side ever yields: permanent deadlock.
- Both sides keep executing valid code forever (CPU sampled at
  `$86:8009/$86:800D` alternating; SPC cycling the IPL at `$FFD2` reading
  `$F4 = 0`). Forcing per-access SPC catchup in the core does not change
  the outcome, so this is a phase-sync accuracy gap in LakeSnes's APU
  interleaving, not a simple catchup-quantization bug.
- Identical on JP and EU (EU is force-blanked in the same wait).

The earlier probe-55 "not audio-driven" conclusion is **reversed**: the SPC
handshake is exactly what blocks progress. Title and name-entry scenarios
remain reproducible; anything that needs the post-name audio load does not.

An earlier "leading hypothesis" (vblank/`$4210` handshake) was disproven:
the `$4210` NMI-wait helper at `$86:8009` works (the CPU sees the flag and
proceeds ~10×/frame); `$4210` timing is fine.

## Reusable RAM symbols (probe-derived)

- `0x0450` program state, `0x047E` current map, `0x0480` map × 2,
  `0x0482` pending map, `0x0488` room-change timer, `0x0456` button mirror
  (A = 0x80), `0x0920` secondary input mirror, `0x0DC8` menu cursor,
  `0x06A4` text speed.

## Oracle capabilities added during the investigation

- `Session::cpu_pc` / `cpu_bank` (bank `$86`, PC sampling).
- `Session::apu_ram` (full 64 KiB SPC RAM snapshot, for driver forensics).