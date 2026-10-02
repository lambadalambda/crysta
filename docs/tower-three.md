# Tower 3 (`$10E`-`$114`): mechanics, native code, runtime model

Research for making tower 3 playable. Addresses are JP, then EU. Banks
`$80`, `$84` and `$8D` have the same addresses in both ROMs. "Guess" marks
an unverified reading. Video times are from the longplay in `local/`.
Model: [tower two](tower-two.md). Map list: [underworld
inventory](underworld-inventory.md) (two corrections below). Enemies:
[enemy scripts](enemy-scripts.md).

## 1. Floors and mechanics

| Map | Video | What the player does | What happens |
|---|---|---|---|
| `$10E` | 39:20 | walks in | gate `$90:8F7E` param 3 (needs `$103`), as tower 2 |
| `$10F` | 39:28-40:24, 40:38 | walks up the entry corridor over a gap; crosses a row of tiles over a pit | column traps fly in and make a bridge; the row tiles crumble 60 frames after Ark comes near; a fall in the upper part leads to `$114` |
| `$114` | 40:25-40:36 | gets the Magirock (chest 8), takes the stairs back | landing controller for a fall |
| `$110` | 40:50-41:52 | presses the four red pedestals (A); dodges wall darts | each pedestal toggles a flag `$001-$004`; block n slides up and down in the chasm while its flag is set and stops where it is when the flag clears; stopped blocks make a diagonal bridge (41:26-41:50) |
| `$111` | 42:00-44:40 | Guardners, Cadets; walks up to the top door | the door opens in four steps (flag `$111`) |
| `$112` | 44:36-45:20 | presses two pedestals (one-shot, flags `$001`, `$002`) | banner "A swarm of Hiballs!", the top door opens (flag `$112`), 8 Hiballs drop from the door |
| `$113` | 45:28-46:55 | talk; fight | "Guardian: ... If you wish to enter, you must overcome this challenge." Room closes, music 5, "High Cadet appeared!", 3 copies; 3 real hits; "So it shall be. Enter the door." |

The diagonal light beams (39:38, 41:05) are background decor (the
`FB $87:98BF`/`98E8` animation actors), not traps. The darts hurt (8
damage at 41:22).

Corrections to the inventory: `$90:FBB3` on `$110`/`$112` are the
pedestal switches, not bridges; the `$10F` "traps" are the bridge tiles.

## 2. Pits and the fall

Attributes `$14` (pit) and `$12` (pit edge) are the holes. The tower 3
floors have large `$14` areas (the chasms on `$10F`, `$110`, `$112`).

Ground test (`$80:CC00`, part of Ark's collision): the attributes under
Ark's four foot samples go to `$02/$04/$06/$08`. Not while `$097C & $41`
(airborne). All four `$14`, or all in {`$12`,`$14`} with the first `$12`,
or the first `$14` and Ark's y in the cell outside 7..13 (`$80:CCB4`):
fall (`$80:CC80`): unless `$097C & 4` (already falling), `TSB $097C #2`
and `COP CB 01 $84:9F53` (Ark's controller). In the 7..13 band: teeter
(`$097C |= 8`, controller `$84:9C95`). Not on a pit and not on `$13`: the
last safe spot `$0958/$095A` and facing `$095C` are saved (`$80:CD0C`).

Fall script `$84:9F53` (both ROMs): falling pose `$84:A8BC`, `$097C |= 2`,
sound `$12`, 8 frames, sound `$10`, 39 frames. Then `JSL $8D:8756`
(carry clear: Ark's cell `$0962/$0964` is in an exit rectangle of the
map):

- In an exit: `COP 07 $801F` (flag `$1F`), idle. `$10F`'s exit
  (0,0) 48x39 has the conditional destination `$F144`: flag `$1F` ->
  `$114` mode 4 (mosaic fade) sel 0 at raw (240,128) (`$8D:8911`: first
  entry whose flag is set). Only rows 0-38 of `$10F` have it; the entry
  corridor (rows 39+) does not.
- No exit (all other pits, `$110`, `$112`): `$84:9F9B`: when `$048A` bit 15,
  damage `$84:D4F4` (life max / 32, at least 4; guess), then `$84:B803`
  puts Ark back on the last safe spot, one cell back from `$095C`; Ark
  hidden one frame, controller back to the standing script by facing.

Landing `$90:FA4E` / `$90:F7D0` (`FD` record on `$114` at (15,9) = Ark's
arrival (248,144)): only when Ark stands exactly on it. Hides Ark, locks
the pad, Ark's controller `$90:FADA` (pose `$13` loop), `$0970 = $FF00`
(Ark drawn 256 px up), then `$0970 += 4` for 64 frames (he drops in),
`$90:FAE3`: `COP C6`; unless the attribute `$3E` is `$13`, sound `$0F` and
landing pose `$14`; pad control `$84:87C1`. Flag `$1F` is not cleared in
these scripts (guess: the loader clears the low temporary flags).

## 3. Scripts

| Script | JP | EU |
|---|---|---|
| `$10F` column bridge tile (right, x+8 / left) | `$90:9C3C`, `9C46` / `9C6D` | `$90:9EF7`, `9F01` / `9F28` |
| `$10F` crumbling row tile | `$90:9C94` | `$90:9F4F` |
| pedestal switch (`FD`, param = flag) | `$90:FBB3` (callbacks `FBD7`/`FBFB`) | `$90:F935` (`F959`) |
| `$110` moving blocks 1-4 | `$90:9A29`, `9A36`, `9A43`, `9A50` | `$90:9CE4`, `9CF1`, `9CFE`, `9D0B` |
| `$110` dart launchers (left / right wall) | `$90:FC2D` / `FC12` | `$90:F9AF` / `F994` |
| `$111` pose-`$15` wall objects | `$90:FC5B` / `FC4A` | `$90:F9DD` / `F9CC` |
| `$111` door controller (door open: `9AE1`; already open: `9B7D`) | `$90:9AC6` | `$90:9D81` |
| `$112` controller | `$90:9BA4` | `$90:9E5F` |
| `$112` banner actor ("A swarm of Hiballs!", guess) | `$90:9BC7` | `$90:9E82` |
| `$112` ball wave Hiball | `$90:9BF7` | `$90:9EB2` |
| `$113` controller | `$90:9CE0` | `$90:9F9B` |
| `$113` Ark faces by `$0956` (`COP 5F`) | `$90:FB0B` | `$90:F88D` |
| `$113` door | `$90:9DEC` | `$90:A0E9` |
| High Cadet controller | `$97:C688` | `$99:9134` |
| High Cadet copies (above / left / right, mirrored) | `$97:C717`, `C7F7`, `C8DD` | `$99:91C3`, `92A3`, `9389` |
| copy struck callback (real or fake) | `$97:C9C5` | `$99:9471` |
| `$114` fall landing | `$90:FA4E` | `$90:F7D0` |
| Guardner | `$97:C345` | `$99:8DFB` |

### `$10F` bridge tiles

Column tile: `COP BA $10`, x+8, each frame `COP D4 $30`: Ark level (within
48 px of its row): `COP 86 04 08` (`07` for the left column; slides four
pose runs toward the corridor), `COP 3F 00 00 00` (its cell becomes floor,
attribute 0), sound `$19`, idle. The 8 tiles end on column 23, rows 47-54
(runtime run), a bridge over the gap.

Row tile: x+8, `COP 3F 02 00 00` (walkable), `COP D6 $30`: Ark within 48:
wait 60 frames, `COP 3F 14 00 00` (pit), pose `$0D` moving, sound `$10`,
pose `$0E`, delete. Cells (11..17, 33) and (29..35, 33).

### Pedestal switches (`$90:FBB3`)

`COP 21 $FBD7` (A from any side, as tower 2's switches). Callback: flag
`+$26 | $8000` set (`LDA $26,X; ORA #$8000; JSL $80:BBCD`), `TSB $045A
#$80`, callback `$FBFB`; `COP 0A $110`: on `$110` the next press clears the
flag (`$FBFB`: `JSL $80:BBCD` with bit 15 clear) and so on (toggle); on
`$112` the callback is removed (one-shot). Each press: sound `$23`, its
cell patched to tile `$FF` (on) / `$FE` (off) (`COP 44`).

### `$110` moving blocks (`$90:9A5B`)

`+$26` = n (1-4); `COP 3F 00` (its cell floor). Loop: 6 passes, then 12,
then 6; each pass (`COP BC`): flag n clear -> `COP 3F 00 00 00`, `RTL`
(it stays and is floor where it stands). Set -> `COP 3F 14 00 00` (the cell
it leaves is a pit), one pose run (`$0B` for the 6-runs, `$0C` for the 12),
`COP 8E`. So it oscillates; the switch stops it.

### `$112`

Controller: flag `$112` -> door already open (`$9B7D`). Else `COP 05 $001`,
`COP 05 $002`, `COP A1 $90:9BC7` (text actor: 120 frames, then text when
no box is open, `$0DC2`), flag `$003`, flag `$112`, door sequence `$9AE1`
(as `$111`: lock pad, four `COP 46` steps with sound `$20`, 16 frames
apart, unlock). Balls: `COP 05 $003`, 120 frames, random sleep (`$0408 &
$1F) << 3`, show, sound `$33`, `COP 86 04 0C` (drop), `+$04 |= 4`,
`$7F:2020 = $97`, `JML $97:B63D` (an ordinary Hiball).

### `$113` and the High Cadet

Controller `$90:9CE0` (deleted by `$11A`): Ark in (22,23)-(26,26): take
Ark (`COP DF $90:FB0B`), 60 frames, text, `COP 46` closes the room,
music 5, give Ark back, 60 frames, flag `$001`, `COP 05 $002`, take Ark,
music 1, flag `$11A`, 120 frames, text, `COP 46` opens, give back.

High Cadet controller (`$97:C68D`): x+8, `COP 05 $001`, lives `7F:102A =
3`. Round: `+$26 = 0`; three `COP E8` group spawns (9 bytes, as `A4`:
long, dx, dy, flags `$2230`) of the copies, each with life `7F:102A =
$6000` (`PHX; TYX; LDA #; STA $7F:102A,X; PLX`). Each frame on `+$26`:
1 -> new round; 3 (real hit) -> `+$26 = 0`, lives - 1; not 0: `COP EB`
(delete the group), new round; 0: flag `$002`, `JML $85:E27B` (death
script). 4 (fake hit) -> `COP EB`, wait.

Copy: `+$06 |= $10` (handles its own knockback), struck and death
callbacks `7F:1016 = 1012 = $C9C5` (bank `$97`). Flickers in (pose 6,
hidden/shown with `COP E4`) until Ark is within 96 (`COP D6 $60`); the
first copy spawns the "High Cadet appeared!" text (`COP A1 $97:CA7D`);
3 more flickers (`COP 85 03 06`, `COP E5`; copies 2 and 3 step x by -1/+1
each). Loop: line move (`COP CC 00 06 04 FF FF`, `CD`) to Ark's probe +
(0,-64) / (-64,+32) / (+64,+32); sound `$2E`, cast poses
`$0C,$0F,$12` (`$0E,$11,$14` for copy 2), sound `$2C`, spell `COP A4
$97:C2DD` (copy 1) or `$97:C315` at (+20,-8); `COP 85 02 06`; again.

Struck (`$C9C5`, any hit): life back to `$6000`; `COP 25`; the real one is
not fixed: `$0408 & 2`:

- 0 (real, 1 in 2): root (`7F:102E`) `+$26 = 3`, sound `$4B`, text
  "High Cadet was real!", pose 6 until the group is deleted.
- else (fake): sound `$24`, hide, text "...was fake!", root `+$26 = 4`
  (the others vanish), leaves the group (`7F:001E = 7F:102E = 0`, root kept
  in `7F:201C`), spawns an ordinary Cadet `COP A4 $97:BD3E` (life 10)
  at its place, 60 frames, wakes it; when the Cadet is gone (`+$04 & $80`)
  root `+$26 = 1` (new round), delete.

## 4. Runtime state (throwaway runs)

`World::enter_with_events` with flags `$100-$105`; Ark placed at each
actor; flags set with `set_flag` to trigger.

| Map | Result (JP / EU) |
|---|---|
| `$10E`, `$114` | nothing frozen |
| `$10F` | bridge tiles fly in, row tiles crumble and go; but `$14` cells are solid to Ark: no fall, and the bridge does not make the gap walkable |
| `$110` | blocks move with flags 1-4; launchers reach pose `$14`, no damage; switch A freezes at `$90:FBD7` / `$90:F959` (flag write by A); Guardner bolt freezes at `$97:C559` / `$99:900F` (`STA $0026,Y` to the parent) |
| `$111` | door opens (patches (22..25,5..6)); Guardner as `$110` |
| `$112` | the 8 balls freeze at `$90:9C0C` / `$90:9EC7` (`ASL` x3, then `STA $00:000E,X`); `COP A1` banner skipped |
| `$113` | text, room closing and flag `$001` work; the High Cadet freezes at `$97:C6AD` / `$99:9159` after the skipped `COP E8`; no copies, flag `$002` never set |

Missing services: `A1` (spawn after the parent), `E8` (group spawn as
`A4`), `EB` (delete the group), `E4`/`E5` (`8E`/`8F` with own countdown),
`5F` (jump by Ark's facing; skipped, Ark faces Down), `C6` (landing),
attribute writes of `COP 3F` (only stamps today), `CB 01` with the fall
scripts. Missing native: `ASL`; `JSL $80:BBCD` (flag write by A) and
`TSB $045A`; writes through Y to another entity (`STA $0026,Y`,
`$0004,Y`) and `PHX; TYX ... PLX` on a child's `7F:102A`/`+$04`; reads of
`7F:102E`/`7F:001E` as the group root/parent; `+$04` masks `$8030`,
`$7FEF`, `$FFDF`, `$FF7F`, `|4`; `$0DC2` (text open).

### After the runtime work

Tower 3 plays from `$10E` to the light room `$106` and the resurrection
(`world/fall.rs`, `COP 3F` attributes, the pedestals' `JSL $80:BBCD`,
`ASL`, writes into other actors, group spawns, struck callbacks). Tests:
`local_towers.rs` (crumbling row to `$114`, a fall elsewhere, a pedestal,
the ball wave, the High Cadet). Open:

- the Guardner's grab (`$97:C5A9`: `COP DF` on Ark, `TSB $097E`, Ark's
  `+$04` through `LDY $0DEA`) freezes at `$97:C5E5` / `$99:909B`;
- the landing's drop on `$114` (`$90:FA4E`) and the falling pose;
- the darts' damage; the teeter at a pit's edge.

## 5. Proposed runtime model

1. **Pits.** A collision class "pit" for attributes `$12`/`$14`: Ark may
   walk onto it. In Rust, after each step, the four foot samples as
   `$80:CC6D`: all pit -> fall state (47 frames, pose `$84:A8BC`, sounds
   `$12`, `$10`). Then the exit under Ark: if the map has one, set `$1F` and
   let the exit check follow conditional lists (`$81:dest`, first set
   flag); else damage (guess rule) and back to the last safe cell. Skip the
   teeter case at first.
2. **`COP 3F` attributes.** Keep a per-map attribute overlay: `3F a` with
   `a` = 0/2 makes the cell floor, `$14` a pit; the stamps stay as today
   for the solid ones. This gives the bridge tiles, the crumbling row and
   the moving blocks.
3. **Landing.** Recognise `$90:FA4E` / `F7D0` in Rust: when the arrival
   came from a fall, draw Ark 256 px up and drop 4 px a frame for 64
   frames, pad locked, then the landing pose.
4. **Switches.** Native `LDA $26,X; [ORA #$8000;] JSL $80:BBCD` (flag
   write by A) and `TSB $045A` (ignored); the rest exists.
5. **Ball wave.** Native `ASL A` in the random-delay idiom.
6. **High Cadet.** A group id per actor: `E8` = `A4` plus group root
   (spawner's id, or its root); `EB` deletes the root's group; `A1` =
   `A2` without flags. Native: `PHX; TYX; LDA #; STA $7F:102A,X; PLX`
   (child's own bytes), `LDA $7F:102E,X; TAY; LDA #; STA $0026,Y` (root
   scratch), `LDA $26,X; TAY; LDA $0004,Y; AND #$80` (child gone).
   Simpler alternative: a Rust fight model keyed on `$97:C688` / `$99:9134`
   (as `world/chest.rs`): three copies as foes with the copy scripts'
   moves, a hit picks real/fake by `$0408 & 2`, 3 lives, flag `$002`.
   `E4`/`E5` can be `8E`/`8F`.
7. **Darts.** Check whether pose `$14` of `$B0:FAF2` has an attack box;
   the launchers need a body for contact damage (guess).
