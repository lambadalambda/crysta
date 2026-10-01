# Tower 2 (`$107`-`$10C`): mechanics, native code, runtime model

Research for making tower 2 playable. Addresses are JP, then EU. "Guess"
marks an unverified reading. Video times are from the longplay in `local/`.
Map list and flags: [underworld inventory](underworld-inventory.md); the
`COP 46` sites: [block patch](block-patch.md); enemies:
[enemy scripts](enemy-scripts.md).

## 1. Floors and mechanics

| Map | Video | What the player does | What happens |
|---|---|---|---|
| `$107` | 29:35 | walks in | pan, title, gate (`$90:8F7E`, needs flag `$101`) |
| `$108` | 29:45-31:25 | chest (30 gems, 30:25), hint "Pay heed to the statues and the color of the jewel in the forehead." (30:30); pushes a statue sideways (31:10-31:20) | the statue slides one cell, flag `$284`/`$285`, the wall behind it becomes stairs up (`COP 46`) |
| `$109` | about 33:00 | two hidden switches; hint "Switches that glow gold. What changes do they bring?" | each switch: flag `$286`/`$287`, sound `$23`, cell patch (`COP 43`); both: controller `$90:985D`/`9B03` sets `$28C`/`$28D` and opens the stairs |
| `$10A` | 33:10-34:40 | four switches; two free push blocks | all four flags `$288-$28B`: controller `$90:98CF`/`9B75` sets `$28E`/`$28F` and copies an 8x15 cell area (a path) |
| `$10B` | 34:40-35:55 | pushes two blocks onto their marks | right marks: flags `$002`, `$003`, controller `$90:991E`/`9BC4` sets `$290`/`$291` and opens the door at (23,4). A wrong mark: flag `$001`, the 12 Hiballs `$90:938B` appear |
| `$10C` | 35:55-36:45 | guardian talk, door to the light | as tower 1's top |

The switches (`$90:971B` etc., header `+$04 = $D100`, `+$06 = $0200`) are
hidden, bodiless actors with a `COP 21` interaction callback from any side:
A next to the switch (guess: not stepping on it). The callback sets the
flag, removes itself and redirects to an idle script that patches the
switch cell (`COP 43 col row $00FF`). They do not freeze today. Not checked:
whether the runtime's A probe reaches a hidden, bodiless actor.

## 2. Scripts

| Script | JP | EU |
|---|---|---|
| statue, pushed Right (from its left) | `$90:9585` (code `958A`) | `$90:982B` |
| statue, pushed Left | `$90:95F4` (`95F9`) | `$90:989A` |
| `$10A` block spawner (spawn, delete self) | `$90:9663` | `$90:9909` |
| `$10B` block watchers | `$90:9672`, `$90:96DC` | `$90:9918`, `$90:9982` |
| push block (spawned) | `$90:FC6E` | `$90:F9F0` |
| push tests Up / Down / Left / Right | `$90:FE3D` / `FEAC` / `FF12` / `FF85` | `$90:FBBF` / `FC2E` / `FC94` / `FD07` |
| take Ark / give him back | `$90:FDA7` / `$90:FDBF` | `$90:FB29` / `$90:FB41` |
| switches `$109` | `$90:9716`, `9740` | `$90:99BC`, `99E6` |
| switches `$10A` | `$90:976A`, `9794`, `97BE`, `97E8` | `$90:9A10`, `9A3A`, `9A64`, `9A8E` |
| Cadet | `$97:BD39` (target pick `BDB8..BE5F`) | `$99:87E4` (`88D1` = JP `BE26`) |

### Statue (`$90:958A`)

`COP B1 8` (x+8); `COP 08 $284 -> 95E8` (already pushed: x+16, seal
`$0F`, idle). `COP 3F 05 00 00` (its cell solid). Loop: counter `+$26 =
60`; each frame `COP BD`, `JSR $FF85`; carry clear resets the counter,
carry set decrements it; below 0 (61 frames of pushing) it goes on:
`JSR $FDA7`; `COP CB 00 $84:A39F` (Ark's push pose, Right; Left uses
`A3A3`); `COP 3F 0F 00 00`, `COP 3F 11 00 01`; `COP 81 03` + `COP 8E` (the
slide); `COP 3F 0F 00 00`; `JSR $FDBF`; `COP 07 $8284`; `COP 36 $32`;
`COP 46` (2x4 cells, flag `$284` -> (9,8), `$285` -> (37,8)); idle.

### Push tests (`$FE3D`, `FEAC`, `FF12`, `FF85`)

Same shape; carry set = "Ark pushes this actor in direction d". Y = Ark
(`$0DEA`), X = the actor, `B = (bx, bw, by, bh)` = the actor's box
`$7F:0028/2A/2C/2E,X` (pose record bytes 4-7, `docs/enemy-scripts.md`).

All need:

- Ark `+$14 == d` (0 Down, 1 Up, 2 Left, 3 Right) and the pad bit for d
  held (`$0400` Down, `$0800` Up, `$0200` Left, `$0100` Right).
- `$066C & $00E0 == 0`: no status (guess: the Cadet's "immobilized" and
  similar; `docs/combat.md`).
- `$0986 & $0200 == 0` (set at `$84:DAA1`, cleared at `$84:DAE3`, `$84:88DC`;
  guess: Ark holds or lifts something).
- Ark `+$04 & $0048 == 0` (`$40` hurt; `$08` not known) and `+$04 & $0010`
  set (on the ground: `$80:CBB1` sets `$0986` bit 11 when it is clear).
- `$097C == 0`: no action.
- Edge (within 1 px, `|diff| < 2`) and span (strict low end, inclusive high):

| d | edge | span |
|---|---|---|
| Up | `Ark.y = y + by + bh + 16` | `x + bx < Ark.x <= x + bx + bw` |
| Down | `Ark.y = y + by` | same |
| Left (`FF12`) | `Ark.x = x + bx + bw + 8` | `y + by + 7 < Ark.y <= y + by + 7 + bh` |
| Right (`FF85`) | `Ark.x = x + bx - 8` | same |

### Take and give back (`$FDA7`, `$FDBF`)

`$FDA7`: points Ark's controller entity (`$0DEE`) at `$90:FDB7`
(`COP C1 $0400`, then `JML $84:87C1`): the pad does nothing for up to 1024
frames. `$FDBF`: `COP 71 0 $FFFF $FE3B` (Ark busy, dead or `$097C != 0`:
return carry set), then the `$066C`/`$0986` tests as above (carry set),
else the controller goes to the standing script for `$0956` (Ark's
facing): `$84:8CDE` Down, `8CEF` Up, `8D11` Left, `8D00` Right (both ROMs),
carry clear.

### Push block (`$90:FC6E`)

Shows itself, `COP DA $FCF2` and `$7F:100A = $FCF2` (wall stop -> bumped),
`COP 3F 05 00 00`. Each frame: `$0978 & $0080` -> skip the frame;
then the four tests in turn; the first that holds counts `+$26` up to 50
(any miss resets it). At 50, direction d: `JSR $FDA7`; `COP CB 00` push
pose (`$84:A394` Up, `A389` Down, `A3A3` Left, `A39F` Right); `COP 40 00 00
00` (its cells passable); `COP C2..C5 $FCF2` (Up/Down/Left/Right: the cell
beyond is solid -> `$FCF2`); `COP 36 $18`; `COP 81 p` + `COP 8E` (one
cell); test again and `COP 2B 00 mask target`: while the button is held,
the next cell. Then `COP 3F 05 00 00`, `JSR $FDBF`, back to the loop.
`$FCF2` (bumped): `JSR $FDBF`, sound `$19`, seal, back to the loop.

`$0978` is Ark's `+$04`, copied each frame at `$87:91A8` (EU `$87:9102`).
Bit 7 is set on entities taken out of play (`$84:D0B4` for Ark,
`$85:E27E` death script; guess: Ark dying or falling).

### `$10B` watchers (`$90:9677`, `96E1`)

`COP 99 $90:FC6E $8034` spawns the block; `TYA; STA $0026,X` keeps the
child. Each frame (`COP BC`): `LDY $26,X`, compare the child's x/y:

| Watcher | right mark | wrong marks |
|---|---|---|
| `9677` (block at 152,368) | x = `$90`: sound `$20`, flag `$002` | x = `$B0`, y = `$160`/`$180` |
| `96E1` (block at 600,368) | x = `$270`: sound `$20`, flag `$003` | x = `$250`, y = `$160`/`$180` |

A wrong mark: sound `$1B`, flag `$001` (the 12 Hiballs wait for it).
Either way the watcher takes the block's x/y, shows itself, deletes the
block (`PHX; TYX; COP A7; PLX`), seals its cell, `JSR $FDBF`, idles. So a
block stops on its first mark.

### Cadet target pick (`$97:BDB8..BE5F`)

Same layer as Ark (recognised idiom), then by `+$26` (random 0-3) the
target `$7F:2004/2006` = probe `$0966/$0968` + (0, +72), (0, -56),
(-64, +8) or (+64, +8). Then `|tx - x|` (`SBC $0000,X`, `EOR #$FFFF; INC`),
`PHA`, `|ty - y|`, `CMP $01,S`: the larger axis picks the pose (6 / 7 / 8,
8 mirrored) for `COP CC 00 p 01 20 FF`; `PLA`. The freeze is `SBC
$0000,X` at `$97:BE2B` / `$99:88D6`.

## 3. Services (missing in `actors.rs`: skipped by length)

| COP | Handler | Use here | Runtime |
|---|---|---|---|
| `40 m dx dy` | `$80:93D4` | clear the attribute of every cell under the box (`AND #$81FF`) | missing |
| `43 col row tile` | `$80:9486` | absolute cell patch (as `44`) | missing |
| `C2..C5 target` | `$80:AB2E..AB88` | jump when the cell beyond the box (Up, Down, Left, Right) is solid | missing |
| `2B 00 mask target` | `$80:9007` | jump while the pad mask is held | missing |
| `71 m1 m2 t` | `$80:A016` | Ark busy test | missing |
| `DA w` | `$80:B530` | wall wake callbacks | missing |
| `CB 00 script` | `$80:ADBD` | Ark's push poses | kind 0 not modelled |
| `3F`, `46`, `81`, `8E`, `99`, `07`, `08`, `05`, `21`, `C0`, `BC`, `BD`, `36`, `B1`, `BA`, `CC`, `D7` | | | present |

## 4. Enemies and frozen scripts

Throwaway run: `World::enter_with_events` with flags `$100`/`$101`, 300
idle frames, Ark at (128,128) and again next to the actors.

| Map | Enemies | Frozen (JP / EU) |
|---|---|---|
| `$107` | none | none |
| `$108` | flyer `$97:B98D` x3 (EU `$99:8438`), Hiball `$97:B555` x7 (EU `$99:8000`) | statues `$90:FF85` / `FD07`, `$90:FF12` / `FC94` |
| `$109` | flyer x2, Hiball x2, Cadet `$97:BD39` x2 | Cadet `$97:BE2B` / `$99:88D6` (when Ark is on its layer) |
| `$10A` | Hiball `$97:B635` x4 (EU `$99:80E0`), flyer x2, Cadet x2 | Cadet `BE2B` / `88D6`; both blocks `$90:FC9B` / `FA1D` |
| `$10B` | Hiball `$90:938B` x12 (wait for flag `$001`) | watchers `$90:967E` / `9924`, `$90:96E8` / `998E`; blocks `FC9B` / `FA1D` |
| `$10C` | none (guardian `$90:9965`, door, object `$90:9812`) | none |

The flyers and Hiballs run. The statues and blocks (packet `$B0:FAF2`)
report `body false`: their art and box are not loaded.

## 5. Proposed runtime model

1. **Push test, as a Rust idiom.** In the script loop's `JSR`, recognise
   the four tests by their bytes (`LDY $0DEA; LDA $0014,Y; CMP #d` ...) and
   the `BCS`/`BCC` after the call; evaluate in Rust: facing `d`, the pad
   bit held, not hurt, no action (`$097C`), and the edge/span table on
   the actor's sprite box (from `$B0:FAF2`, `boxes::packet_list`). Status
   `$066C & $E0`, `$0986 & $0200`, airborne: not modelled, so they pass.
   The native machine is not a good fit: Y on Ark's entity, `EOR`, `,X`
   position reads, the box words and a carry across `RTS` would all be new.
2. **Take/give back.** Recognise `JSR $FDA7` / `$FB29` (pad off: as `COP
   CB` kind 1 does) and `JSR $FDBF` / `$FB41` (release, carry clear when
   Ark is not busy). Model `COP CB 00` with the four push scripts as
   "Ark moves with the actor": add the actor's per-frame displacement to
   Ark while held.
3. **`$0978`**: add to `READABLE` in `native.rs`; the world keeps it 0
   (no out-of-play state yet).
4. **Cells**: `COP 40` drops the actor's stamps under its box; `COP C2..C5`
   probe the cell beyond the box through the collision map; `COP 2B` as
   `COP 2F` with the pad; `COP 43` as `COP 44` with absolute cells.
5. **`$10B` watchers**: a Rust model (as `world/chest.rs`): keyed on the
   watcher scripts, it owns the spawned block, compares its position with
   the constants read from the ROM, plays the sound, sets `$001`/`$002`/
   `$003`, then turns the block into the static watcher (seal, release).
   Smaller alternative: let `COP 99` return the child, accept `TYA; STA
   $0026,X`, and add a native `LDY $0026,X; LDA $0000,Y/$0002,Y` read of
   a linked child's position.
6. **Cadet**: in `native.rs`, read-only fields `$0000,X`/`$0002,X` (the
   actor's x, y) for `LDA`/`SBC`/`CMP`, `EOR #`, and `CMP $01,S` against a
   pushed wide A. The rest (`$7F:2004/2006`, the probe, `PHA`/`PLA`) is
   there. Later Cadet code (cast, `COP 71`, `5F`, `D2`) is not checked.
7. **Bodies**: load packet `$B0:FAF2` for the statues and blocks (needed
   for the box and the drawing).
