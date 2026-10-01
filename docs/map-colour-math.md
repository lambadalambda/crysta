# Map colour math at load (maps `$0A..$21`, `$41..$44`)

Where the colour-math registers of a map come from, and how a decoder reads
them. Addresses are JP; EU is the same unless given. Read from the code and
checked against the oracle (JP journey state `stairs-21`, `stairs-E`; the
captures in `local/effects/{jp,eu}/`).

## Summary

Two steps set the display. The load sets TM/TS/CGWSEL/CGADSUB from a room
profile and clears the fixed colour. Then an actor of the map may set the
fixed colour, CGWSEL and WOBJSEL through `COP 76`. In the slice, only maps
`$0E`, `$20` and `$21` get a fixed colour (7, subtracted).

## 1. The load

The map-change code in bank `$8D` (one long block; it runs on every map load)
does this, in this order:

1. `$8D:8BA6..8BE9` (EU same): `$7F:0800..0802 = 0`, **`$2132 = $E0`**
   (fixed colour 0, written to the PPU), `$0471..0473 = 0`,
   `$2126 = 0`, `$2127 = $FF` (window 1 = whole line; shadows `$0464/$0465`).
2. `$8D:8BF0` `JSL $86:86ED` → `$86:955C`: reads the map header.
3. `$86:8C61`: applies the room profile (below). It writes the PPU registers
   **directly** and their shadows: `$212C/$212E` + `$0468` (TM/TMW),
   `$212D/$212F` + `$0469`, `$2130` + `$046A`, `$2131` + `$046B`. Also
   BG1SC/BG2SC, BG12NBA, BGMODE, scroll and layer flags.
4. `$86:8DE9`: window/OBSEL set from table `$86:8E5B` (16 bytes a set):
   `$2101`, `$210B`, `$210C`, `$2123`, `$2124`, `$2125` (WOBJSEL, + `$0470`),
   `$2128`, `$2129`, then words to `$083C`, `$0834`, `$0836`, `$0838`.
   All slice maps use set 0: WOBJSEL `$00` (no colour window).

No HDMA is used for these values. `$091C` stays 0: the NMI shadow copy
(`$85:FC5B`, EU `$85:FCF3`) is not needed because the registers are written
directly. The NMI write of `$7F:0800..0802` to `$2132` (`$85:F9CA`, EU
`$85:FA62`) has no effect while the bytes are 0 (no channel bits set), so the
PPU keeps the last real fixed colour.

### Header and profile format

- Header pointer: word at `$82:8000 + 2*map`; if 0, at `$83:8000 + 2*map`
  (the spawn-list table; `$82` is 0 for all slice maps). Bank is that of the
  table. Byte 0 → `$048B` (map flags). **Byte 1 = profile byte.** The spawn
  stream starts after these two bytes.
- Profile byte: bits 0..5 = profile index, bits 6..7 = window set
  (`(p >> 2) & $30` is the offset into `$86:8E5B`); `p & $C0 == $80` sets
  `$0868 = $80`.
- Profile table: word pointers at JP `$96:BB64 + 2*index` (EU `$99:C2AE`),
  bank-local. Record bytes: `+0` TM, `+1` TS, `+2` CGWSEL, `+3` CGADSUB,
  `+4` scroll/layer flags (bits 4..5 → `$0C23`), `+5` BG1SC/BG2SC bits,
  `+6` BGMODE (`$2105`), `+7`, `+8` scroll parameters.
- Profile records: JP `$96:BC02 / BC1D / BC2F / BCDA`, EU
  `$99:C34C / C367 / C379 / C424`.

## 2. The map actors

`COP 76 rr vv` (`$80:A127`) queues a write; the NMI applies it at `$86:8260`
(EU same): it writes `$21rr` and the shadow from the map at `$86:82C1`
(`$2132` → `$0471`, `$2130` → `$046A`, `$2125` → `$0470`).

- **Dark controller.** Spawn element `FE 00 5D 80 88`: script `$88:8062`
  (both ROMs) runs `COP 99 $88:8029`, which does `COP 76 32 E7`
  (fixed colour 7, all channels), `COP 76 30 40` (CGWSEL `$40`),
  `COP 76 25 21` (WOBJSEL `$21`). Window 1 is the whole line (step 1), so
  "outside the window" is empty: no black, and the subtraction applies on the
  whole screen. Unconditional in `$0E` and `$20`. In `$21` only when event
  flag `$244` is set (spawn condition `FA 44 82 ..`; guess: the flag means
  "box opened", it is set before the tour returns to `$21`). Measured: the
  shadows change 68 frames (`$0E`) and 22 frames (`$21` after the tour)
  after the map ID changes.
- **Box script, first visit of `$21`** (flag `$244` clear; record JP
  `$88:ACF5`, script `$88:ACFA`; EU record `$88:B550`, script `$88:B555`).
  At JP `$88:AD16` (EU `$88:B571`): `COP 76 32 E7`, `30 20`, `25 21`,
  `27 00`, then `COP 6A 00 03` (the glow square). Queued 20 frames after the
  load (JP 26041 → 26061). CGWSEL `$20`: no math inside the colour window.
- **Lamp actor** `$88:8038` (script `$88:803D`), in every slice map
  `$0A..$21`. It deletes itself in `$0E`, `$1F`, `$20`, `$21` (`COP 49`).
  Otherwise, under an event condition (`COP 47 $102A $802C`, not decoded),
  it spawns `$88:8000`: TM `$16`, TS 0, CGWSEL `$20`, CGADSUB `$A3`, and
  `$7F:0800..0802 = 3F 52 80` (fixed colour R31 G18 B0 through the NMI).
  It does not fire in any slice capture.

Other direct writers of fixed colour `$E7` (`$87:A249`, `$87:A45B`,
`$87:A657`; guess: light-spot actors with an HDMA window table at
`$7E:4648`) are not reached in the slice (traced: `$87:A249` is not run on
the `$20` → `$21` load).

## 3. Per-map values (JP and EU agree)

| Maps | Profile | TM | TS | CGWSEL | CGADSUB | After actors |
|---|---|---|---|---|---|---|
| `$0A` | `$08` | `$16` | `$01` | `$82` | `$33` | none; fixed 0 |
| `$0B..$0D`, `$0F..$1F` | `$06` | `$17` | `$12` | `$82` | `$21` | none; fixed 0 |
| `$0E`, `$20` | `$1B` | `$15` | `$00` | `$20` | `$B3` | CGWSEL `$40`, WOBJSEL `$21`, fixed `$E7` |
| `$21`, flag `$244` clear | `$1B` | `$15` | `$00` | `$20` | `$B3` | CGWSEL `$20`, WOBJSEL `$21`, WH1 0, fixed `$E7`, glow |
| `$21`, flag `$244` set | `$1B` | `$15` | `$00` | `$20` | `$B3` | as `$0E` |
| `$41..$44` | `$03` | `$15` | `$00` | `$80` | `$00` | none; fixed 0 |

All use window set 0 (WOBJSEL `$00` at load). CGADSUB `$B3` = subtract,
backdrop, OBJ, BG2, BG1 (not BG3, no half). With fixed colour 7 every
5-bit channel of those layers goes down by 7, clamped at 0.

Guess (not measured): between the load and the actor's first `COP 76` the
screen is still in the fade-in, so a decoder can apply the final values from
the first frame.

## Decoder recipe

1. Header: `$83:8000 + 2*map` (try `$82` first) → byte 1 = profile.
2. Profile record (bank `$96` JP / `$99` EU) → TM, TS, CGWSEL, CGADSUB.
3. Fixed colour 0, WOBJSEL from `$86:8E5B` set.
4. Walk the spawn stream (`assets::maps::actors`, with the `FA` condition):
   an `FE` element with pointer `$88:805D` gives fixed `$E7`, CGWSEL `$40`,
   WOBJSEL `$21`; the `$21` box record gives the glow values.
