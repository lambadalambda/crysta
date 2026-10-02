# Tower 4 (`$115`-`$11B`): mechanics, native code, runtime model

Research for making tower 4 playable. Addresses are JP, then EU. Banks
`$80`, `$84` (below `$84:D000`) and `$8D` have the same addresses in both
ROMs. "Guess" marks an unverified reading. Video times are from the
longplay in `local/`. Models: [tower three](tower-three.md),
[tower two](tower-two.md). Map list: [underworld
inventory](underworld-inventory.md) (corrections below). Enemies:
[enemy scripts](enemy-scripts.md).

## 1. Floors and mechanics

| Map | Video | What the player does | What happens |
|---|---|---|---|
| `$115` | 49:40 | walks in | gate `$90:8F7E` param 5 (needs `$105`) |
| `$118` | 49:50-50:20 | walks up the centre platform from the door (23,54) | guardian talk, flag `$001`, "Three Cadets appeared!" (50:00); the platform is walled (columns 19 and 28, rows 43-52) until the three die; then the walls at (19,44..46) and (28,44..46) open (`COP 43`, `COP 46`), flag `$11C` |
| `$118` | 50:27, 50:33 | crosses the ropes (row 45 to the sides, row 36 between the sides) | tightrope (§2) |
| `$118` | 51:21-51:27 | takes the drop of a Cadet (`FF $90:A065` / `$90:A38B`, A), jumps into the pit | "Your courage is at test. At times you must take bold chances." (flag `$11B`); pit (13,24) 22x40 -> `$117` |
| `$117`, `$116`, `$118` top, `$119` | 51:30-57:10 | Guardners, Hiballs, Cadets, flyers; ropes (52:20, 53:40-54:05); chest 10 on `$119`'s island | "Guardner appeared!", "Ark's toasted!" (52:55); four holes on `$117` -> `$116`; `$116`'s stairs lead back to `$117`; `$117`'s top stairs to the top rooms of `$118`, and from there to `$119` (route: guess) |
| `$11A` | 57:15-58:20 | Crystal Thread (chest 11, 57:18); rope row 34 to the centre platform (57:38-58:00); the door (23,33) | without `$11D`: "I have prepared a most entertaining show for you. Savor it. Enjoy." and `COP 14` -> `$11B` |
| `$11B` | 58:20-60:05 | fights the ball chain | "Introducing the Dancing Huball Troupe!"; 8 dancing balls; each hit turns one into an ordinary Hiball; at 0: end text, flag `$001` -> `$11D`, back to `$11A` (raw (376,552), sel `$20`) |
| `$11A` | 60:10-60:20 | the door again | with `$11D`: door opens, `COP 14` -> `$106` |

Corrections to the inventory: there are no burner or spike actors. The
tripods and spike rows are map art (with the `FB $87:98E8` animation
actors). "Ark's toasted!" (52:55) is a status of Ark (§2), not a tile. The
rope is the attribute `$12` (§2), not an actor. The `$116` landings are
4 `FD $90:FA4E` records (one per hole of `$117`).

Fall exits (all flag `$1F`, mode 4, as tower 3):

| From | Exit (cells) | Conditional | To (raw) / landing `FD` |
|---|---|---|---|
| `$117` | (10,32), (10,47), (32,32), (32,47), 6x6 each | `$81:F150`, `F15C`, `F168`, `F174` | `$116` (208,480), (224,720), (544,544), (544,720) / (13,31), (14,46), (34,35), (34,46) |
| `$118` | (13,24) 22x40 | `$81:F180` | `$117` (368,528) / (23,34) |
| `$119` | (0,0) 48x64 (whole map, ropes too) | `$81:F18C` | `$118` (368,384) / (23,25) |

`$116` and `$11A` pits have no exit: damage and back (the existing rule).

## 2. Native mechanics

### Tightrope (attribute `$12`)

Ropes are one-cell rows of `$12` between rows of `$14`: `$118` rows 36
and 45, `$119` rows 41 and 48, `$11A` rows 34, 41 and 48. Ground test
`$80:CC00` (Ark's four samples, `$02/$04` the top row at `y + 7F:002C`,
`$06/$08` the next row): all four in {`$12`,`$14`} and the top-left `$14`
and the top sample's y in the cell in 7..13 (`$80:CCB4`): rope. Else
(top-left `$12`, or outside the band): fall (`$80:CC80`, as a pit). On a
rope, unless `$097C & 4` (falling) or `& 8` already: `$097C |= 8`, Ark's
controller `+$0A = $84:9C95` (the "teeter" of `docs/tower-three.md`). Any
sample not in {`$12`,`$14`} clears bit 8 (`$80:CCF0`); the rope controller
then goes back to pad control `$84:87C1`.

Rope controller `$84:9C95` (both ROMs; only the `JSL $84:D6DB` / `D6C1`
operand differs): `$097C & $100` (carrying): drop it (`JSL $84:D6DB` with
`$C5CB`). Ark `+$04` bit 4 clear (guess: Ark knocked) -> `$84:9BC6`:
wobble pose (`COP 84 07 89` / `06 88` by frame parity), 4 frames, the
fall script `$84:9F53`. `$0986 & $3C00` (guess: Ark hit) -> lean by frame
parity. Else pose by facing (`$84:A8F0` up/down, `A8F4`), then each frame
`COP 2B`: `$8000` (attack) -> `$84:9DA2` (thrust from the rope, guess),
Up -> lean up `$84:9C44`, Down -> lean down `$84:9BFE`, Right -> walk
`$84:9D3A` (pose `$84:A8E1`, `COP 84 0F 0A 01`), Left -> `$84:9D6E`
(`A8E5`, mirrored). Movement itself is the normal applier: only Left and
Right move on the rope.

Lean (`$9BFE`; `$9C44` mirrored): `TSB $045A`, facing kept in `+$26`, pose
`$84:A8F6` 16 frames, pose `$84:A3B0`, drop a carried item, then 60 frames:
the same direction again -> fall (`$9BD9`/`$9BCC`); the opposite ->
recover (`$9C8A`, back to the walk by `+$26`); time out -> fall.

Collision: in the directional tables (`$80:D542`, `D8E8`, `DC60`, `DFDC`,
pair tables `$40` apart) type 18 takes the Open handlers in tables 0 and
1 of all four directions and in the Left/Right "3" tables, and the Solid
ones (as 12) in the four "2" tables and the Up/Down "3" tables. The C2-C5
tests of actors (`$80:C0C7`) block on every attribute but 0, 1 and `$16`.

### Falls

As tower 3 (`world/fall.rs`): the exits and the landings above. New: the
whole-map exit of `$119`, so a fall off a rope there goes down to `$118`;
on `$118` and `$11A` a fall off a rope over a pit without exit costs life.

### Lips (attribute `$08`, guess)

Rows of `$08` above `$117`'s holes, on `$118` (27..30, 23..24) and along the
pits of `$119`/`$11A`. Down, Left, Right first samples: Open; Up first:
`$80:D506` (Partial unless `$097C & 4`). Both top samples of class 8
(table `$80:CF30`: `$08`, `$13`, `$1F`) -> `$80:CF50`: `$097C |= 1`, Ark
`COP CB 01 $84:9ECC` (facing 1) / `$84:9EBC`: pose `$13` with move
selector `$27`, sound `$10` (guess: a slide off the lip; carrying:
`$84:9F13`/`9F33`). Not on the needed path: `$117`'s holes can be entered
from below, `$118`'s pit from the sides.

### Statuses

"Ark's toasted!" is a status: `$85:DAC3` / `$85:DB5B` gives Ark the status
of the attacker's stats record (chance by level, guess), into `$066C` and
`$0670,X`; `$84:D32B` runs it, the burn by `COP DF $84:DBF1` / EU
`$84:DBBE` (Ark `+$04 |= $10`, poses 4 and 5, text, 62 frames). Which T4
attack burns: guess, the Guardner's vacuum flames or the T4 Hiballs.

## 3. Scripts

| Script | JP | EU |
|---|---|---|
| `$118` guardian (rect (20,44)-(28,46), sets `$001`) | `$90:9E2E` | `$90:A12B` |
| `$118` Three Cadets (wait `$001`, then the Cadet) | `$90:9EEA` | `$90:A201` |
| `$118` controller (`$0498` - 3, then `$11C`, walls) | `$90:9F33` | `$90:A24A` |
| `$118` hint drop (`FF` p5, flag `$11B`) | `$90:A065` | `$90:A38B` |
| `$11A` door (`$11D`: -> `$106`; else text, -> `$11B`) | `$90:9FB6` | `$90:A2CD` |
| `$11A` ring (stats `$04`, patrols, guess) | `$97:B434` | `$97:BC34` |
| `$11B` controller (wait `$001`, `$11D`, -> `$11A`) | `$90:A04B` | `$90:A371` |
| `$11B` show controller | `$97:CB03` | `$99:95B4` |
| show head / ball struck callback | `$97:CEBF` / `CF21` | `$99:9975` / `99D7` |
| show end (death script, text, `$001`) | `$97:CD07` | `$99:97BD` |
| Guardner / bolt | `$97:C345` / `C50A` | `$99:8DFB` / `8FC0` |
| Guardner vacuum / Ark asleep / watcher | `$97:C40F` / `C5A9` / `C5E5` | `$99:8EC5` / `905F` / `909B` |
| Cadet / paralysis | `$97:BD39` / `C2AF` | `$99:87E4` / `8D65` |
| flyer / its burst | `$97:BBC2` / `BCAD` | `$99:866D` / `8758` |
| Hiball (stats `$16`) | `$97:B847` | `$99:82F2` |
| rope controller / lips / fall | `$84:9C95` / `9EBC`, `9ECC` / `9F53` | same |
| fall landing | `$90:FA4E` | `$90:F7D0` |

### `$118`

Guardian `$90:9E33`: `COP 48 $11C`; Ark in (20,44)-(28,46): lock, text,
music 5, flicker, flag `$001`, pose `$0A`, text, delete. Three Cadets
`$90:9EEF`: `COP 05 $001`, random wait, flicker in, `$7F:2020 = $97`,
`COP 06 $97:BD3E` (the Cadet). Controller `$90:9F38`: flag `$11C` -> the
open state (`$9F6B`). Else `+$26 = $0498 - 3`; each frame `$0498 == +$26`:
take Ark (`$90:FB0B`), flag `$11C`, music 3, give back, `COP 43` (19,44..46)
`$01B7` and (28,44..46) `$01AF`, two `COP 46`, delete.

### `$11B`: the Dancing Huball Troupe

Controller `$97:CB08`: at (0,0), `COP 5A`, palette, 240 frames, `+$26 = 8`
(balls left), text actor `COP A1 $97:CF69`. Five formations, then again:
followers `COP A4` (flags `$0220`), `+$26 - 1` of them 3 frames apart, then
the head `COP E8` (flags `$0200`) at the same spot: (-37,46) wait 160,
(284,196) 280, (55,276) 240, (-40,52) 240, then a column at x 48, y 240 +
20k. The 3-frame loop keeps its count in `7F:0002,X` and its resume
address in `7F:0000,X`, and resumes by `STA $000A,X; RTL`. `+$26 = 0` at
any wait: `$CD07`: `COP A1 $97:CD11` (120 frames, end text `$97:CF7A`, 240
frames, flag `$001`), the controller `COP BF $85:E27B` (death script).

Ball (head `$CEBF`; followers `$CD4D`.. share the dance): palette `$0C`
(head), `+$06 |= $10`, struck and death callbacks `7F:1016 = 1012 = $CF21`
(bank `7F:1014 = $97`), life `$6200`; `COP B0 02`; moves by pose selectors
(`COP 87 04 0E 90`, `82 0D 99`, `87 06 0E 90`; the head takes 1-2 more
`82 0D 99` steps when fewer than 6 / 3 balls are left). Struck: life back
to `$6200`, hide, parent `+$26 -= 1`, `COP E8 $97:B84F` (a Hiball at its
place), its life 10 and `7F:1020 = $FFC4` (`PHX; TYX; STA $7F:...,X; PLX`),
delete. Which balls take hits (followers too?): guess, all with a body.

### Guardner sleep and vacuum

Bolt `$97:C50A` (hits Ark, callback `COP 58 59 $C551`: skip when `$0648 =
$59`, guess: an item): 1 in 2 (`$0408 & $80`): hide, `COP 71` (Ark busy:
skip), `COP A1 $97:C5E5` (watcher: deletes itself when Ark's script bank
is not `$97` or `$097E & $400` is clear), `COP A1 $97:C63C` ("Ark fell
asleep!"), root `+$26 = 1` (`STA $0026,Y`), `COP DF $97:C5A9`: Ark
`$097E |= $400`, `+$04 |= $30` (`LDY $0DEA; STA $0004,Y`), sleep poses
(`COP 89 03 07 00 05`, `COP 84 08 00 05`), `+$04 &= ~$20`, back to
`$84:87C1`, `$097E &= ~$400`. The Guardner on `+$26 = 1`: `COP E8
$97:C40F` (vacuum) until `$097E & $400` clears. Vacuum: copies the root's
`7F:101E` (`LDA $7F:001E,X; PHX; TAX; LDA $7F:101E,X; PLX`); per frame
`COP D3` picks Ark's x push (-1/0/+1) and y push toward the Guardner's y
+ 26, written to Ark's `7F:0018`/`7F:001A` (`PHX; LDX $0DEA; STA
$7F:0018,X; PLX`); every 4th frame two flame particles `$97:C4B0`; on
arrival: pose 7, text `$97:C61F` (the Guardner's vacuum, guess),
`$097E &= ~$400`, `COP 15`, delete.

## 4. Runtime state (throwaway runs)

`World::enter_with_events` with flags `$100-$105`; Ark placed at each
actor for 300 frames, life refilled, `hit_spawned` every 30 frames.

| Map | Result (JP / EU) |
|---|---|
| `$115` | nothing frozen |
| `$116`, `$117`, `$119` | Guardner capture: watcher `$97:C5E5` / `$99:909B`, vacuum `$97:C40F` / `$99:8EC5` freeze, Ark's script freezes at `$97:C5A9` / `$99:905F` (Ark stuck); flyer burst freezes at `$97:BCD4` / `$99:877F` (`LDA $002C,X; TAY`) |
| `$118` | guardian, flag `$001` and the Cadets' AI run; but no Cadet of `$118` has a body: `$82:EB0B` / `$82:EA98` is refused ("unsupported house graphics transfer", field `10 00 98 33`; the six others reuse it). They cannot be hit, `$0498` never drops, `$11C` never comes. Cadet paralysis `$97:C2AF` / `$99:8D65` freezes (`TSB $097E`) |
| ropes | Ark cannot step onto `$12` (room-core: unsupported type 18); a `$119` fall lands on `$118` (376,400) as it should |
| `$11A` | ring freezes at `$97:B46D` / `$97:BC6D` (C2-C5 loop with no yield, guess: blocked on all sides); door: text and `-> $11B`, and with `$11D` `-> $106`, both work |
| `$11B` | the controller freezes at `$97:CB33` / `$99:95E4` (`STA $7F:0002,X`) after the 240 frames; no balls |

Missing: the rope state and type 18 in room-core; the descriptor form of
`$82:EB0B`; services `58`, `89`, `15`, and with them `71`/`D3`/`BE` in the
vacuum (not reached). Native: `$097E` (read, `TSB`/`TRB`); writes to Ark
through Y or X (`STA $0004,Y`, `EOR #$8000`; `PHX; LDX $0DEA; STA
$7F:0018/001A,X; PLX`); `PHX; TAX` on the parent for `7F:101E`; `SEP #$20;
LDA $000C,X; CMP #`; `LDA $002C,X; TAY` (guess: the last child); own
bytes `7F:0000/0002,X` and the resume `STA $000A,X`; the child's
`7F:1020`. Not modelled: statuses (`$066C`), lips (`$08`).

## 5. Proposed runtime model

1. **Cadet bodies on `$118`.** Accept the descriptor form of `$82:EB0B`
   (the graphics transfer with a nonzero first word, `$0010`; guess: a VRAM
   tile offset, the guardian holds the base). Without it the first fight
   of the tower cannot end. Check `$113`'s High Cadet copies, which use it
   too.
2. **Tightrope.** room-core: type 18 as Open in the first-sample tables
   (the pair tables as listed in §2, or Open as a first cut). `fall.rs`:
   the rope band of `$80:CCB4` puts Ark in a rope state (no fall), which
   allows Left/Right only, with the rope poses; leaving the `{$12,$14}`
   cells ends it. Up/Down: the lean and its 60-frame window, then the
   existing fall. Needed on `$118` (row 45) and `$11A` (row 34).
3. **Guardner sleep and vacuum.** Random, but it leaves Ark stuck. Rust
   model keyed on the bolt (`$97:C50A` / `$99:8FC0`): on a hit, 1 in 2,
   Ark asleep (sleep pose, pad locked), text, the Guardner pulls him to
   its y + 26 at 1 px a frame, text, release. Or natively: `$097E`, the
   writes to Ark, `COP 89`, `COP 15`, the parent read.
4. **The show.** Native own bytes `7F:0000..0003,X` and `STA $000A,X`
   (resume at A) let `$97:CB33` run; then check `COP 5A` and the pose
   selectors `$90`/`$98`/`$99`, the struck callback (`E8` of a Hiball,
   the child's `7F:1020`). Alternative: a Rust fight keyed on `$97:CB03` /
   `$99:95B4` (as the High Cadet): 8 balls in formation, each hit -> a
   Hiball, at 0 the end text and flag `$001`.
5. **Smaller.** The flyer burst (`$002C` as the last child, guess); the
   Cadet paralysis (`$097E`, Ark `+$04` blink); the `$11A` ring (a yield
   when blocked on all sides); the burn status; the lips; the landing
   drop (tower 3 §5.3).
