# Ark's jump (B)

Native traces, JP and EU (2026-10-09; runner and states in the job's
`tmp/jump/`). Bank `$80` and `$84` code is at the same addresses in both
ROMs.

## Start

`COP 2B` jumps when every bit of its mask is in `$0454`, the held buttons
minus `$045A`. The jump sets `$045A |= $8000`: B must be released and pressed
again for another jump.

| State | B test | Jump |
|---|---|---|
| Standing, Down/Up | `$84:8914`, `8A39` | `$84:9710` |
| Standing, Right/Left | `$84:8B24`, `8C0F` | `$84:96DE` |
| Walking, Down/Up | `$84:8D70`, `8DEB` | `$84:9710` |
| Walking, Right/Left | `$84:8E66`, `8EE1` | `$84:96DE` |
| Dashing | `$84:9078`/`90C9`/`911A`/`916B` | `$84:99B1`/`99C5`/`99D9`/`99ED` |
| Carrying a pot | `$84:AC25`.. | `$84:AE94`/`AE9C`/`AEA4`/`AF14` |
| On a rope | `$84:9CE7`.. | `$84:9DA2`, then `9E41`/`9E84` |

`$84:9252` is the thrust (A), not the jump. Ark's `+$04` bit `$10` means
"not attacking"; only the jump attack clears it (`$84:9867`).

## The ground jump

`$84:96DE`/`9710` pick by facing (`974B` Down, `9739` Up, `975D` Right,
`978F` Left): the crouch script (`$84:A606`/`A624`/`A642`/`A646`), 2 frames
(`COP C1 02`), the air script (`$84:A612`/`A630`/`A652`) and `JSR $9990`
(`$097C |= 4`, `COP AF 39`, `$0972` = the height stream). The air loop is
`COP 02 19`: 25 passes of `$84:98D2` (steering), `$84:9972` (the walk
stream to Ark) and `$84:97C1` (the A window, `+$26` 8 to `$12`). Then
`$84:98AB` jumps again at once if B is pressed, else sound `$0F` and
`$84:87C1`.

J is the first crouch frame, one frame after `$0454` shows B.

| Frame | h (`$0970`) | Pose (resource 3) | `$0986` | Sound (port 3) |
|---|---|---|---|---|
| J..J+2 | 0 | list 0 (Down), 3 (Up), 6 (Right/Left) | 0 | |
| J+3..J+14 | H[0..12] | list 1/4/7, 12 frames | `$0400`, then `$1400` | J+4: `$0E` |
| J+15..J+26 | H[12..24] | list 2/5/8, 12 frames | `$1400` | |
| J+27 | 0 | stand | `$1000` | |
| J+28 | 0 | | 0 | `$0F` |

H = -5 -10 -13 -16 -19 -21 -23 -25 -26 -27 -28 -28 -28 -27 -26 -25 -23 -21
-19 -16 -13 -10 -5 0 (stream `$39` at `$7F:668A`: `1:-5, 2:-3, 2:-2, 2:-1,
1:0, 2:+1, 2:+2, 2:+3, 1:+5`, a delta held count + 1 frames, added by
`$84:C199`/`C205`). Left is mirrored. Ark is drawn at y + h (`$80:F0BA`);
his place does not change with h.

- Steering: on frames with `$0042` even, `$84:98D2` reads the pad and starts
  a walk stream (`COP AF` 1 Down, 2 Up, 3 Right, `$52` Left, 4..7 the
  diagonals, 0 none); Ark then moves 1, then 2 pixels (a diagonal 1 and 1).
  No move in the crouch; 34 px in 23 frames with a held direction. The
  facing does not change; walls stop him as on the ground.
- In the air the ground test `$80:CC00` runs, but no pit falls
  (`$80:CC80`), no safe place is kept, no lip drops (`$80:CF50`), no rope
  is entered (`$80:CCCC`). He clears a 2-cell pit (`$10F`).
- Landing on a pit: the fall at J+27 (`$84:9F53`), no `$0F`. On a lip: the
  drop `$84:9EBC` at J+27.
- Enemy hits skip Ark while h <= -16 (`$85:F856`, EU `$85:F8EE`; source
  only), J+6..J+22.
- `$0F` comes at J+28 with no direction held, at J+29 with one held (the
  walk starts at J+27: still, still, then a pixel at J+29).
- The shadow: see below.
- With a weapon, A in the loop's passes 8..18 (J+10..J+20) is the jump
  attack (`docs/combat.md`).

## The dash jump

`$84:99B1`/`99C5`/`99D9`/`99ED` (Down/Up/Right/Left). Here J is the first
air frame, the frame after `$0454` shows B; there is no crouch.

| Frame | h (stream `$3A`) | Pose | Sound (port 3) |
|---|---|---|---|
| J..J+11 | -4 -8 -11 -14 -16 -18 -20 -21 -22 -23 -23 -23 | resource 3 `$09` (Down), `$0B` (Up), `$0D` (Right, Left mirrored), 7+5 frames | `$0E` at J+1 |
| J+12..J+23 | -23 -22 -21 -20 -18 -16 -14 -11 -8 -4 0 0 | `$0A`/`$0C`/`$0E`, 8+4 frames, the last held | |
| J+24 | 0 | the last record; controller `$84:8F0E` | |
| J+25..J+31 | 0 | resource 1 `$26` (Down), `$27` (Up), `$28` (Right/Left), 4+3 frames | |
| J+32 | | the dash on if the way is held, else the brake | brake `$0D` at J+33 |

- The dash's stream (3, 2, 2) runs on through the jump with its phase; the
  pad does not steer; 56 px in J..J+23. No `$0F`.
- J+24..J+31 is the dash's release grace (`$84:8F0E`, then `COP 5F` to
  `8F1D`/`8F6A`/`8FB7`/`9004`): the same way held resumes the dash at J+32,
  a perpendicular press at J+27 turns it at J+28, B with the dash's way at
  J+27 is a new dash jump at J+28. The brake: J+32..J+47 (2,2,1,2,1,2,1,2,1,
  0,0,1,0,0,0,0), resource 0 `$09`/`$0A`/`$0B`; Ark stands at J+48.

## The carry jump

`$84:AE94` Down, `AE9C` Up, `AEA4` Right, `AF14` Left; J is the first air
frame; no crouch, no sounds.

| Frame | h = `$0999` | Ark (resource 3) |
|---|---|---|
| J..J+11 | -5 -10 -13 -16 -19 -21 -23 -25 -26 -27 -28 -28 | `$1B` (Down), `$1E` (Up), `$21` (Right/Left), 8+4 frames |
| J+12..J+21 | -28 -27 -26 -25 -23 -21 -19 -16 -13 -10 | `$1C`/`$1F`/`$22`, 6+1 frames |
| J+22 (Down/Up/Right) | -5 | the carry stand, resource 0 `$03`/`$04`/`$05`; h 0 at J+23 |
| J+22, J+23 (Left) | -5, 0 | still `$22` (`COP 02 0D` at `$84:AF55`, else `0B` at `AEE5`) |
| J+24 (Left) | 0 | the carry stand `$05`, mirrored |

- Steering: every frame (`COP 61`), the carry walk stream +1,+2 from J+1;
  the facing stays; walls stop him. Control returns on the landing frame;
  the jump-throw window is `$84:AF84` (`+$24` 8 to `$15`, with A).
- The pot (`$0DF4`) is at Ark's place with h; lists `$25` (Down), `$26`
  (Up), `$27` (Right/Left) of the helper art. One 16x16 OBJ at (x + dx, y +
  h + dy):

| Facing | J | J+1..J+8 | J+9..J+18 | J+19..air end | Landing |
|---|---|---|---|---|---|
| Down | (-8,-34) | (-8,-46) | (-8,-44) | (-8,-40) | (-8,-44) |
| Up | (-8,-40) | (-8,-46) | (-8,-46) | (-8,-42) | (-8,-45) |
| Right | (-2,-37) | (-6,-46) | (-9,-46) | (-5,-44) | (-8,-45) |
| Left | (-14,-37) | (-10,-46) | (-7,-46) | (-11,-44) | (-8,-45) |

## The rope jump

`$84:9E41`: the horizontal poses, the same frames, `$0E`, no `$0F`, no A
window; back to `$84:9C95` at J+28, then the lean `$84:9BF8` by `$0042`
parity. Jumping onto a rope: the landing frame enters it, `$0F`, the same
lean. `$84:9BC6` (wobble and fall) needs `+$04` bit `$10` clear (an
attack), not a jump.

## The shadow

`$84:A917` is Ark's ground shadow, always on (the jump changes its list):
helper art (JP `$A2:C000`, EU `$A4:C000`) list 0 (`$C7EE`, one 8x8, tile
`$112`, at (x-4, y-8)), 1 (`$C806`, two 8x8, tile `$113`) or 2 (`$C825`,
tile `$114`, at (x-8, y-8) and (x, y-8)); palette 2, priority 2; at Ark's
ground place. The list by d = -(h of the previous frame): 48 or more 0, 24
or more 1, else 2 (ground jump list 1 at J+11..J+19 counting J from the
crouch; carry jump J+8..J+16; dash jump never). Hidden with `$097C & $EA`,
Ark `+$04 & $8000`, or the cell at (x, y-8) in `$84:AAC9` (attributes
`$08`-`$0B`, `$0E`, `$0F`, `$1A`, `$1F`), where `$84:A957`..`A98A` moves it
down a row (code only).
