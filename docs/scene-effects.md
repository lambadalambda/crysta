# Scene effects: the voice glow, the freeze, the frozen people, the page icon

Research notes for `meta/issues/voice-glow-effect.md`, `freeze-effect.md`,
`frozen-townsfolk.md` and `spear-text-icon.md`. Addresses are Japanese
(`$JP` / European `$EU` where they differ). "Guess" marks what is not traced.

## Native display registers (both ROMs, same code)

- NMI `$85:F990`: CGRAM DMA from `$7F:0600` (512 bytes) every frame;
  fixed colour from `$7F:0800..0802` (raw `$2132` bytes).
- When `$091C` is set, `$85:FC5B` copies the shadows: TM/TMW `$0468`,
  TS/TSW `$0469`, CGWSEL `$046A`, CGADSUB `$046B`, W12SEL/WOBJSEL
  `$046F`/`$0470`, window edges `$0464..0467`, fixed colour `$0471..0473`.
- Scripts also write the PPU directly (inline code) and through `COP 76 rr vv`
  (`$80:A127`: queues `$21rr = vv` in `$04D6`, applied at NMI).
- HDMA: enable mask DP `$86`, channel params DP `$8C..$AF`.

## 1. The voice glow (blue door, box room)

**Trigger.**
- Blue door, map `$0C`: the door script, after the second hit, runs
  `COP 6A 00 03` at `$88:ABF8` (EU `$88:B436`), then `COP 05 06 00` (waits for
  local flag 6), `LDA #0; STA $7E:46E6` (square off), unstamps, deletes. The friend
  `$88:9A6F` spawns the fade child `COP A2 $88:9CD8 $9000` at `$88:9BB2`
  (EU child `$88:A1BB`).
- Box room, map `$21`, before `$22`: the box script (`$88:ACFA`, EU near
  `$88:B581`) runs `COP 76 32 E7` (fixed colour 7), `COP 76 30 20`
  (CGWSEL), `COP 76 25 21` (WOBJSEL), `COP 76 27 00` (WH1), `COP 6A 00 03`.
  The map already has CGWSEL `$20`, CGADSUB `$B3`, fixed `$E7` (shadows).
  This lasts as long as the box is closed.

**Fade child `$88:9CD8`** (inline code, then script):
- `TM = $16` (BG1, the light-ray layer, goes off; it is not restored until
  the next map load), `CGWSEL = $20` (no colour math inside the colour
  window), `CGADSUB = $A3` (subtract, BG1+BG2+backdrop; OBJ and BG3 not),
  `WOBJSEL = $21` (colour window = W1, not inverted), `WH1 = 0`.
- Up ramp: `COP 02 07 00` loop: fixed colour intensity +1 (all channels,
  `ORA #$E0; STA $2132`), `COP C1 10 00`, `COP 03`. Measured: one step every
  18 frames, 7 steps (JP 22773, 22791 ... 22881; EU 27712 ... 27820).
- `COP 07 04 80` (local flag 4), `COP 05 05 00` (waits for local flag 5:
  the friend sets it after page `$88:9DC3` and 60 frames).
- Down ramp: 7 steps of 18 frames to 0 (JP 23249 ... 23339).
- `WOBJSEL = 0`, `WH1 = 0`, `COP 07 06 80`, deletes. CGWSEL/CGADSUB/TM keep
  their values (no visible math at fixed colour 0).
- Per pixel outside the window, main screen BG2/backdrop:
  `c' = max(0, c - k)` per 5-bit channel, `k` the fixed intensity.

**The spinning square: `COP 6A shape speed`** (`$80:9DD6`).
- Spawns a native child running `$8D:AFA1`; `$8D:AFE2` loads shape `shape`
  from table `$8D:B038` (pointer, vertex count; 6-byte vertices x, y, edge
  links). Shape 0 = square, vertices `(±48, ±48)`.
- Each frame: angle `$7E:46E8 += speed` (byte, 256 = one turn; speed 3, one
  turn in ~85 frames); vertices rotated with `$81:F422` (sin) / `$81:F462`
  (cos), signed bytes /128 (`$8D:B2CE`); centred on the parent actor's
  `(x, y)` minus the camera; rasterised (`$8D:B15F`) into a double-buffered
  HDMA table (`$7E:5A00` / `$5A10`, data `$7E:5C00` / `$5E00`, toggled by
  `$7E:46E4`) that writes `$2126/$2127` (W1 left/right) per line
  (`$8D:B7C2`, B-bus `$26`, mode 1, indirect).
- Measured: door actor `(184,352)`, camera `(0,256)`: window lines 46..144,
  widest 137..233 (centre ≈ (185,95)). Box actor `(136,384)`, camera
  `(0,239)`: lines 88..201, centre ≈ (131,145).
- Start angle: whatever `$7E:46E8` holds (not reset; guess: it is free
  running across uses).
- Inside the window colour math is off, so the square shows the room at
  full colour inside a darkened room. Sprites are never darkened here.

**Ours.** `COP 6A` and `COP 76` are stepped over (`COSMETIC` in
`crates/crysta-runtime/src/actors.rs`); the `$88:9CD8` writes are skipped by
`display_code`. No darkening, no square, BG1 rays stay on.

**Change.**
- Add a world display state: TM bits, CGWSEL/CGADSUB, fixed colour,
  colour window enable, and a "polygon window" (shape, speed, angle, parent
  actor). Feed it from `display_code` (record writes to `$2125`, `$2130`,
  `$2131`, `$2132` (`$E0|k`), `$212C`, shadows `$0468..$046B`), from
  `COP 76` (register/value pairs) and from `COP 6A` (start; `$7E:46E6 = 0`
  stops it).
- Map loads set the shadows (map `$21`: `$20`, `$B3`, `$E7`).
- crysta-app: per pixel, inside the rotated square no math; outside,
  subtract `k` from BG2/backdrop (and OBJ where CGADSUB bit 4 is set, as in
  `$21`); hide BG1 when TM bit 0 is clear.
- Captures: `local/effects/{jp,eu}/cellar-door/`, `.../box/`.

## 2. The freeze (map `$21`, return from the box)

**Trigger** (Elle `$88:B2FE`, guide `$88:AEB8`).
- Elle: `COP 00 $88:B3B0` (EU `$88:BD6B` -> `$88:BDA9`): waits 32 frames,
  spawns six crystals `COP A2 $88:B573/B58D/B5A7/B5C1/B5DB/B5F5 $1010`
  (EU `$88:BFD2`...), returns; `COP 07 02 80` (local flag 2);
  `COP 02 02 00 { COP 38 37 37; COP C1 10 00; COP 03 }`;
  `COP BB 0E` (Elle turns blue, see 3); `COP 3B`; waits 160; text.
- Guide: waits for flag 2 (`$88:AEF1`), `COP 80 01; COP 8E`,
  `COP 00 $88:B507` (EU `$88:BF66`), the whitening.

**Whitening `$88:B507`.**
- Clears its own `+$04` bit 12; `$8D:A8EA` saves the palette buffer
  (`$7F:0600` -> `$0400`); `CGADSUB = $A3` (OBJ out of the subtraction).
- 37 times: `$8D:AA96` (each channel of all 256 colours +1, capped at 31),
  `$80:80DF` (a nested frame; only actors with `+$04` bit 12 run, i.e.
  the crystals, spawned with `$1010`). One step per frame: JP 50577..50613
  (`$7F:07E0` 39CE, 3DEF, 4210 ... 7FFF).
- `COP C1 3C 00` (60 frames white: BG grey = 31 - 7 = 24, rgb ≈ 200; sprites
  pure white), `$8D:A8FD` restores the palette (JP 50677/50678),
  `CGADSUB = $B3`, sets `+$04` bit 12, returns.

**Crystals `$88:B573..B5F5`.**
- `JSR $B60F`: `+$06 |= $4000`; `COP D8 00 C0 A2` (display table
  `$A2:C000`, entry 9 = `$A2:C0E2`: 3 frames of 9 ticks, guess on layout).
- `COP D0 0009 0000 0000 FFE8 aaaa 0080 02 FC 8000` (`$80:B136`): pose 9,
  centre = parent (Elle) + (0,-24), angle `aaaa` = 0, `$AA`, `$154`,
  `$1FE`, `$2A8`, `$352` (1024 = one turn), radius 128; per frame angle +4,
  radius -4; ends at radius 0 (33 frames). Position (`$87:C701/C72F`):
  `x = cx + r*cos(a)*2/256`, `y = cy + r*sin(a)*2/256`, table `$81:F563`
  (sin, 1024 entries, ±127; cos at +256). Then `COP D1`/`A7`.
- On screen: 8×8 OBJ tile `$73`, attribute `$24` (palette 2, priority 2).
  JP 50576..50608.

**Ours.** `$88:B507` runs as one native run (`actors/native.rs`): the
palette calls and nested frames are accepted and dropped, so there is no
whitening and the 37 frames pass at once. The crystals freeze at `JSR $B60F`
(bodiless). `COP BB` is stepped over.

**Change.**
- Make `$80:80DF` in a native run a frame boundary: yield, then resume the
  loop next frame; set `Screen.tint = Tint::Raise(step)` per step (the
  existing `Tint` in `world/fade.rs`), hold `Raise(37)` until `$8D:A8FD`,
  then `None`. While the run lasts, colour math `$A3` with fixed 7: subtract
  7 from BG only (sprites stay white).
- Model `COP D0`/`D1` (orbit) for the crystals and accept `JSR $B60F` (or its
  two parts) in native runs; art from the `$A2:C000` table, tile `$73`,
  OBJ palette 2.
- Captures: `local/effects/jp/freeze/` (route timing), and
  `local/effects/eu/freeze-stairs-entry/` (EU, reached by poking flags `$22`
  and `$240..$244` and entering by the stairs: Ark stands at `(136,128)`,
  Elle is off screen; the whitening starts at EU 34226).

## 3. The frozen people

**Native.**
- Each resident script starts with `COP 09 $1023 $8109 target` (e.g.
  `$88:8A08`): normal script while `!($23 ^ $109)`; after the freeze ($23 set)
  it falls through to `COP BB 0E` (or `0C`) and `COP 06 $88:D32D` (EU
  `$88:E0F1`): `+$06 |= $0200`, `COP 21 $D363` (frozen talk), `COP BC`,
  `RTL`. They never walk or select a pose; the header's initial selector
  stays (frozen town: 0,0,0,0,0,0,5,0,6,2,2,9; these match ours). An
  initial pose with several frames still animates (one friend in C does).
- `COP BB n` (`$80:AA8A`): `+$08 = (+$08 & $F1FF) | n << 8`, the palette
  field (bits 9-11). Measured: OAM palette = (frame palette + field) mod 8.
  Friends with palette 4 get `0E` (7), those with 5 get `0C` (6); both show
  OBJ palette 3 (attr `$27`). Elle (palette 4) + 7 = 3.
- OBJ palette 3 (CGRAM `$B0..$BF`) is `$CC:2A6C` (EU `$CE:2A6C`; table
  `$80:FC72`, selection 0 entry 3), with colour 14 = `$08DF` instead of
  `$7C1F` (patched elsewhere; guess: another loader). Same in `$0A`, `$0C`,
  `$21`, `$42`.

**Ours.** A probe (`World::enter_with_events`, flags `$20-$23 $26-$28 $2E $FE
$240-$244 $292 $296`, maps `$0A..$11`, 600 frames): no resident moves and
the selectors match native. The palette is the descriptor's own (`COP BB`
skipped), so nobody is blue. The reported walking is not reproduced (open).

**Change.** Keep a per-actor palette field from `COP BB` (bits 1-3 of the
operand); draw with OBJ palette `(own slot + field) & 7`; for slot 3 use
`$CC:2A6C` with colour 14 `$08DF`. Captures: `local/effects/jp/frozen-house-C/`,
`.../frozen-town/`.

## 4. The page icon on voice pages

**Native.** The prompt (`$CB:7A98`, 2bpp 16×16, background pixels are colour
3) is drawn by `$85:9E51`: when the page has no window (`$0DA4` bit 2,
`$C4 0`) through `$85:947F` (colour 3 -> 0, as the glyphs), otherwise
`$85:93A5`. So on a voice page the icon background is clear.

**Ours.** `draw_window` (`crates/crysta-app/src/window.rs`) draws the prompt
with colour 3 = the window shade even when `page.background_index() == 0`,
so a solid blue square shows. The glyphs are already cleared by `blit`
(`crates/assets/src/text.rs`).

**Change.** In the prompt loop skip index 3 when the page is windowless
(`!windowed`), as `blit` does for glyphs. Captures:
`local/effects/jp/spear-icon/spear-page-1.*` (icon ≈ x 184..200, y 203..219),
and the box pages (no window) in `.../box/frame-27110/27290/27470` (EU
32099/32279/32460).

## Captures

`local/effects/{jp,eu}/<scene>/`: `frame-NNNNN.png` and `.rgb` (256×240 RGB),
`frames.csv` (one row a frame: map, CGWSEL/CGADSUB/TM/TS shadows, `$091C`,
fixed colour `$0471..73` and `$7F:0800..02`, DP `$86`, player, camera, mean
RGB, visible OAM count, CGRAM hash; the freeze adds `$7F:07E0` etc.) and
`frames.oam.csv` (every visible sprite). `bright` is -1: the oracle has no
INIDISP read. DP `$86` is sampled after the frame and is not reliable.
Routes: JP from `local/pandora-tower-discovery/departure/journey` states;
EU from `local/pots/states/eu-pot-first-ready.state` plus the JP route
lines 164-247 of `tools/pandora-qualification/route.jsonl`. Tools:
`fxprobe` and scripts in the session scratch directory (not in the repo).

## Open

- The colour-14 patch of OBJ palette 3, and its source.
- Where the reported walking in the frozen town comes from (not seen).
- The exact real PPU register values (only shadows and code are read).
- The crystal display list layout at `$A2:C0E2`.
