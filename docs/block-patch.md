# COP 46: block copy of map cells

Research for the tower doors and walls that scripts open by copying map
cells. JP addresses unless marked. The handler and its helpers have the
same address in EU. Evidence: source reads, and one native per-frame WRAM
record on `$101` (JP checkpoint `first-tower-neutral-stable`,
`tmp/tower/jp2/ctl.state` plus route lines 579-615; `$0498` poked to 0 at
frame 65910).

## Handler `$80:9532` (EU same)

Encoding: `02 46 n lim sx sy dx dy wait`, 9 bytes (7 operand bytes). The
job-folder `coplen.json` gives 3; that is wrong.

| Operand | Meaning |
|---|---|
| `n` | cells per row minus 1 (the row is `n+1` cells wide) |
| `lim` | last row offset, in pixels (`$00`, `$10`, ... ) |
| `sx sy` | source cell (signed byte × 16, `$80:BC2F`) |
| `dx dy` | destination cell (signed byte × 16) |
| `wait` | sleep written to `E+$0E` when the copy ends |

Two per-actor bytes, set by the script before the COP:

- `7F:201A,X` = row offset `r` in pixels (counter). Scripts store 0.
- `7F:201B,X` = layer select `L`.

Script idiom: `LDA #$0L00; STA $7F:201A,X; COP 46 ...`.

Each time the COP runs:

1. Queue check. If `$0910 >= $80`, or `$0910 + n*16 >= $80`: set
   `E+$0A` to the COP, `E+$0E = 0`, and yield. The COP runs again next
   frame with the same counter. `$0910` is the byte fill of the VRAM queue
   at `$088E` (8-byte entries, 2 entries for each cell on screen, 16
   entries in total). Note: the check uses `n*16`, not `(n+1)*16`.
2. If `lim < r` (unsigned byte compare): the copy is done. `E+$0E = wait`.
   The script continues **in the same frame** after the 9 bytes. The
   sleep takes effect at the script's next yield.
3. Otherwise, copy one row. For `i = 0..=n`: destination cell
   `(dx+i, dy + r/16)` gets the tile of source cell `(sx+i, sy + r/16)`.
   Then `r += $10`, `E+$0A` = the COP, and the actor yields (one row each
   frame). The step path does not write `E+$0E`.

Rows copied = `lim/16 + 1`. The source column advances with `$8D:8CE1`
(next cell, wraps at the end of the map row). The destination is computed
again for each cell from pixels through `$8D:8C7E`, so it also wraps.

Layer select (`$80:95DF`):

| `L` | Source | Writer | Effect |
|---|---|---|---|
| `0` | first layer `$7E:A000` | `$8D:8DF8` | first layer: collision and picture |
| `1..$7F` | second layer `$7E:E000` | `$8D:8ED6` | second layer: picture only |
| `>= $80` | buffer `$7E:C000` (`$088B` geometry) | `8DF8` if `L & 3 == 0`, else `8ED6` | not used in chapter 1 |

Only the source tile ID (`word & $1FF`) is used. The writer rebuilds the
word: `tile | (attr[tile] & $7F) << 9`, with `attr` at `$7F:0000` (first
layer) or `$7F:0200` (second layer). So the collision class comes from the
tile ID, the same as for COP 44. Collision code reads only the first
layer (`LDA $7E:A001,X` has many sites in the ROM; `LDA $7E:E001,X` has none).
A second-layer copy never changes collision.

### Helpers

- `$8D:8C7E`: in `$1C` = x pixels, `$22` = y pixels, X = layer (0 or 2;
  `>= $80` uses the `$088B` layout). Out: X = byte offset of the cell in
  the layer, A = column. Masks with `$085A,X`/`$085E,X` (wrap), row stride
  = 16 × `$0827,X` cells (screens wide).
- `$8D:8CE1`: X += 2. At the end of a map row it wraps to the row start.
- `$8D:8DF8` / `$8D:8ED6`: write one cell word (first / second layer), then
  queue its four 8×8 tile words from `$7E:2000+tile*8` (second layer
  `$7E:3000`) as two column entries, each only if that half is near the
  screen (camera `$081E/$0822` or `$0820/$0824`). Cells off screen change
  in WRAM only. This is the same writer that COP 44 uses.
- `$80:95DF`: the per-row loop. It runs `n+1` times: read the source word,
  call the writer, advance the source, and add 16 to the destination x.

## Measurement on `$101` (JP)

Controller `$90:905E` (EU `$90:9269`). Layer width is 16 cells (256 px).
The door is at rows 1-4, columns 7-8, far above the camera. For that
reason the VRAM queue stays empty (`$0910 = 0`) during the whole copy.

| Frame | Script | Cells written |
|---|---|---|
| 65910 | polls `$0498` (`E+$0A = $9064`); poke to 0 | |
| 65911 | flags `$280/$281`, pad lock `$045E=$FFF0`, sound `$32`, COP#1 row 0 | L2 (7,4),(8,4): `00C5,00C6` → 0 |
| 65912 | COP#1 row 1 | L2 (7,5),(8,5) ← (7,6),(8,6) (0 → 0) |
| 65913 | COP#1 done (`E+$0E=8`), COP#2 row 0, yield | L2 (7,3),(8,3): `00BD,00BE` → 0 |
| 65914-21 | sleep 8..0 | |
| 65922 | COP#2 row 1 | L2 (7,4),(8,4) ← row 5 (0 → 0) |
| 65923 | COP#2 done (`E+$0E=8`), `L=0`, COP#3 row 0, yield | L1 (7,4): `1B07` → `00C4` (tile `$107` attr `$0D` → tile `$C4` attr 0). (8,4) is `0EC5` already |
| 65924-31 | sleep | |
| 65932 | COP#3 done (wait 0), `COP 29 $FFF0` unlocks the pad, `COP A7` deletes the controller | |

The pad is locked for 21 frames (65911-65931). Sources: L1 row 47 holds the
spare cells `00C4 0EC5`. After the copy, the door's lower two rows on the
second layer are empty. Rows 1-2 (`B5 B6`, `BD BE`) stay. The stairs cell
(7,4) on the first layer becomes passable (attr 0).

## COP 46 in chapter 1 tower scripts

All are in bank `$90`. Cells are `(column,row)` ranges, inclusive.
"Rows" = `lim/16+1`. `wait` is 0 unless shown.

| Map | Site JP / EU | L | w×rows | Source | Destination | wait |
|---|---|---|---|---|---|---|
| `$101` ctl `905E` | `9080` / `928B` | 2 | 2×2 | (7,5)-(8,6) | (7,4)-(8,5) | 8 |
| | `9090` / `929B` | 2 | 2×2 | (7,4)-(8,5) | (7,3)-(8,4) | 8 |
| | `90A0` / `92AB` | 0 | 2×1 | (0,47)-(1,47) | (7,4)-(8,4) | 0 |
| `$102` drop `90B0` → `90CD` (EU `92BB`/`92D8`) | `9105` / `9310` | 2 | 2×2 | (37,9)-(38,10) | (37,8)-(38,9) | 8 |
| | `9115` / `9320` | 2 | 2×2 | (37,8)-(38,9) | (37,7)-(38,8) | 8 |
| | `9125` / `9330` | 0 | 2×5 | (62,0)-(63,4) | (37,6)-(38,10) | |
| | `9135` / `9340` | 2 | 2×5 | (62,0)-(63,4) | (37,6)-(38,10) | |
| `$108` statue `9585` (EU `982B`) | `95DC` / `9882` | 0 | 2×4 | (45,43)-(46,46) | (9,8)-(10,11) | |
| `$108` statue `95F4` (EU `989A`) | `964B` / `98F1` | 0 | 2×4 | (45,43)-(46,46) | (37,8)-(38,11) | |
| `$10C` object `9812`, hit path `9831` (EU `9AD7`) | `984E` / `9AF4` | 2 | 1×2 | (24,21)-(24,22) | (24,23)-(24,24) | |
| `$109` ctl `985D` (EU `9B03`) | `9884` / `9B2A` | 2 | 2×2 | (23,13)-(24,14) | (23,12)-(24,13) | |
| | `9898` / `9B3E` | 2 | 2×2 | (23,12)-(24,13) | (23,11)-(24,12) | |
| | `98AF` / `9B55` | 0 | 2×4 | (49,10)-(50,13) | (23,10)-(24,13) | |
| | `98BF` / `9B65` | 2 | 2×4 | (49,11)-(50,14) | (23,10)-(24,13) | |
| `$10A` ctl `98CF` (EU `9B75`) | `98FE` / `9BA4` | 0 | 8×15 | (52,11)-(59,25) | (21,21)-(28,35) | |
| | `990E` / `9BB4` | 2 | 8×15 | (52,11)-(59,25) | (21,21)-(28,35) | |
| `$10B` ctl `991E` (EU `9BC4`) | `9945` / `9BEB` | 0 | 2×1 | (46,2)-(47,2) | (23,4)-(24,4) | |
| | `9955` / `9BFB` | 2 | 2×5 | (46,0)-(47,4) | (23,1)-(24,5) | |
| `$111` ctl `9AC6` | `9AEF` / `9DAA` | 0 | 4×2 | (50,3)-(53,4) | (22,5)-(25,6) | |
| | `9AFF` / `9DBA` | 2 | 2×4 | (51,3)-(52,6) | (23,5)-(24,8) | |
| | `9B16` / `9DD1` | 0 | 4×2 | (58,3)-(61,4) | (22,5)-(25,6) | |
| | `9B26` / `9DE1` | 2 | 4×5 | (58,3)-(61,7) | (22,5)-(25,9) | |
| | `9B3D` / `9DF8` | 0 | 4×2 | (50,13)-(53,14) | (22,5)-(25,6) | |
| | `9B4D` / `9E08` | 2 | 4×4 | (50,13)-(53,16) | (22,5)-(25,8) | |
| | `9B5D` / `9E18` | 2 | 6×1 | (49,17)-(54,17) | (21,9)-(26,9) | |
| | `9B6D` / `9E28` | 2 | 6×1 | (49,18)-(54,18) | (21,10)-(26,10) | |
| | `9B84` / `9E3F` | 0 | 4×2 | (58,13)-(61,14) | (22,5)-(25,6) | |
| | `9B94` / `9E4F` | 2 | 6×7 | (57,13)-(62,19) | (21,5)-(26,11) | |
| `$113` ctl `9CE0` (EU `9F9B`) | `9D0E` / `9FC9` | 0 | 6×11 | (48,12)-(53,22) | (21,12)-(26,22) | |
| | `9D4E` / `A009` | 0 | 6×11 | (54,12)-(59,22) | (21,12)-(26,22) | |
| `$118` ctl `9F33` | `9F96` / `A2AD` | 2 | 1×3 | (20,44)-(20,46) | (19,44)-(19,46) | |
| | `9FA6` / `A2BD` | 2 | 1×3 | (27,44)-(27,46) | (28,44)-(28,46) | |

`$111` pauses with `COP C1 $0010` (17 frames) between its stages. `$109`
uses `COP C1 $0008` between rows. `$10C` and `$118` also patch single
cells with `COP 43` before their COP 46. Other COP 46 sites (`$90:A953`
on maps `$12D-$137`, `$90:B28E`, `$90:D8CA`..`F01F`, banks `$91`, `$93`-`$95`)
are not in chapter 1 towers.

The `$108` statues skip their COP 46 when their flag is already set.
Guess: the map load's flag patches restore those cells on re-entry.
