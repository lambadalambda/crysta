# The menu: Yomi's box

Research for [menus, inventory and saves](../meta/issues/menus-inventory-save.md),
from native captures on both ROMs.

Select opens the menu (`COP 2D 0020 → $85:A179`, European `$85:A211`, in the
player controllers `$84:88F0…`), only once event flag `$FE` is set (the
frozen return): before that Select does nothing. Start is a separate pause
(`$80:820C`). The menu is Yomi's box: first the hall (below), a tile-layer
room (not Mode 7) that looks like the tour's central hall (map `$041`), with
Yomi as the cursor on nine hotspots. A on a hotspot opens its screen:

| Hotspot | Screen |
|---|---|
| item door | 8×3 grid of the 27 shared slots, a row of fixed slots; A asks Use / Equip (`$0648`, the field's X uses it); L shows the description |
| weapon, armor doors | 16 pedestals each (12 shared slots, 4 fixed); A equips (`$064A`, `$064C`); see [the rooms](#the-weapon-and-armor-rooms) |
| mirror | status: attack, defense (+bonus), luck, Prime Blue, EXP, next |
| memo | options: keys (`$0634–063E`), cursor memory (`$06B4` bit 14), sound, window colour, text speed, gauge |
| jewel box | magic (a separate screen) |
| map, shelves | map; move and battle guides |

Data: level `$0656`, max life `$0657`, life `$065D`, attack `$0662`, defense
`$065F`, luck `$0666`, weapon/armor bonus `$0659`/`$065B`, EXP `$0690` (BCD,
3 bytes). Equipment stats: `$8D:BC92` (European `$8D:BB5B`), 4 bytes per item
from `$80` (power, attribute). Item icons as the shop's.

In the slice the shop items (`$1E`: S.Bulb, M.Bulb, P.Cure, Hex Rod, Leather;
`$1D`: Fire Ring, Ice Ring) and the Crystal Spear can end up in the menu.

## The hall

Captured with `palnative` from `tower-align-east-rest.state` (JP; `warp $102
520 460`) and `local/depth/states/eu-spear.state` with `flags 254` (EU).
Addresses are Japanese, European in brackets. Bank `$85` code is +`$98` in
European unless given.

### Opening and closing

S is the first frame with Select held; C the same for the Select that closes.

| Frame | Japanese | European |
|---|---|---|
| S+1 | `INC $049A`, `$097C` \|= `$8800`; Ark's script → `$85:A0F0` (anim `$1C`, the pose changes at S+15) | same |
| S+1..S+19 | field shown, its BG animations go on | same |
| S+20..S+34 | fade out, brightness 15 → 0, one step a frame (LUM 37 … 4, 0) | same |
| S+35..S+60 | black (forced blank, loading) | S+35..S+57 |
| S+61 | hall at full brightness at once (`$85:A95A`: `INIDISP = $0F`), no fade-in, no mosaic | S+58 |
| S+62 | the label window's gradient appears | S+59 |
| C+0..C+10 | hall still shown (`$85:A333`: `COP C1 08 00`, then the reload) | same |
| C+11 | black at once (no fade-out) | same |
| C+35..C+49 | field fades in, 15 steps (LUM 4 … 39) | C+32..C+46 |
| C+32 | Ark's script → `$85:A113` (anim `$1D`) | |
| C+49 | `$097C` &= `~$8000` | C+46 |
| C+57 | `$049A` = 0, `$097C` = 0; Ark's controller again at C+58 (`$84:A303`) | C+54 |

Ark comes back at his field position, the camera too (`$7E:46E4..474D` is
saved to `$7E:474E` on opening and copied back on closing; HUD layout `$06BE`
is saved to `$7F:0CBE` and set to 1).

### Code path

- `$85:A179` [`A211`]: gate: `($7A)` non-zero and flag `$FE` (`COP 08`),
  else `TSB $045A #$2000` (Select masked until released) and end. Then pause
  the field, `COP C1 10 00` (the fade starts at S+20), fade (`$8D:A3EE`),
  then the loads.
- `$85:A3EF` [`A487`]: forced blank, then load stage A from the table
  `$85:A40B`. Opening runs stage 9, then stage `$0A`, then stage 0
  (`$06BC` = 0; table `$85:A25C`: 1 map, 2..4 the door rooms, 5 the jewel box).
  - Stage 9 `$85:A811`: Yomi's OBJ tiles, LZ `$B1:D322` [`$B3:D8A3`] →
    VRAM `$5A00` (OBJ tiles `$1A0–$1BF`).
  - Stage `$0A` `$85:A842`: BG chars LZ `$E1:5DD5` [`$E3:5DD5`] → VRAM 0 and
    palette `$B1:DEFA` [`$B2:FF15`] → colours 16..127, both as map `$041`;
    the six `$AF:E45B…` [`$B1:E9A1…`] palette overrides as `$041`
    (`COP 5A`, bank first). Own data: metatiles `$AD:B7E4` [`$AC:F824`] →
    `$7E:3000` (`$1000` bytes), layer 1 `$B0:B111` [`$B2:B213`] → `$7E:D000`,
    layer 2 `$B1:E32E` [`$B3:E7CF`] → `$7E:D800` (`COP 5B`; both
    dimension-prefixed, 32×32 cells, the screen is the top-left 16×14).
    OBJ colours: `$B1:D831` [`$B3:DDB2`] → 128 (24), `$B2:8B58` [`$B4:90BB`]
    → 160 (16), `$B2:8C40` [`$B4:91C3`] → 240 (16, Yomi).
  - Stage 0 `$85:A425`: layer size from `$B0:B111`, camera (0,0), HUD sheet
    LZ `$A9:F02F` [`$AB:F02F`], bytes `$400..$15FF` → VRAM `$4200`; Ark
    moved to (128,208).
- The hall is not map `$041`'s background: tiles and colours are shared,
  but the metatiles and layers are separate (the layer 1 cells match `$041`'s
  `$B0:AAC5` in 984 of 1024 cells, the metatiles in 3875 of 4096 bytes).
- Tasks: the hall task `$85:AC2F` [`ACC7`] (`COP E7`), which spawns Yomi
  `$85:C5DD` (entity `$1540` in the captures, `+$26` of the task); a helper
  `$85:D1EE` (`COP AB`, kept in `$7F:0CE0`).

### Hotspots

Current hotspot: `$06A6` (0..8). On close it is reset to 7 (item door),
with `$06A8/AA/AC/B8/BC` = 0, unless cursor memory (`$06B4` bit 14) is on.

Table: `$92:BCA3` [`$92:BE51`] has one pointer per screen (`$06BC`); the
hall's is `$92:BCB9` [`$92:BE67`], 12 bytes per hotspot: x, y, then the
entry pointers for Up, Down, Left, Right (0 = no move). Positions are
Yomi's (entity x/y = screen pixels, camera 0).

| # | x, y | Up | Down | Left | Right | JP label | EU label |
|---|---|---|---|---|---|---|---|
| 0 | 136, 136 | 3 | – | 1 | 2 | せっていメモ | Settings Memo |
| 1 | 88, 136 | 4 | – | – | 0 | つよさの鏡 | Strength Mirror |
| 2 | 184, 136 | 5 | – | 0 | – | 地図 | Map |
| 3 | 136, 104 | 7 | 0 | 4 | 5 | 宝石箱 | Jewelry Box |
| 4 | 64, 104 | 6 | 1 | – | 3 | 戦いの手引書 | Battle Handbook |
| 5 | 208, 104 | 8 | 2 | 3 | – | 旅の手引書 | Guidebook |
| 6 | 72, 56 | – | 4 | – | 7 | 武器のとびら | Weapon Door |
| 7 | 136, 56 | – | 3 | 6 | 8 | 道具のとびら | Item Door |
| 8 | 200, 56 | – | 5 | 7 | – | 防具のとびら | Armor Door |

A 3×3 grid, no wrap. Both ROMs measured: same graph and positions.

Input (`$85:AC82` [`AD1A`], each frame): held `$0454`. A or Select first
(below), then L, then `$85:CB2A` [`CBC2`]: Up before Down before Left
before Right. A move writes the new index, sets the moved direction in
`$045A` and `$045C` (Up/Down if pressed, else Left/Right), retargets Yomi
(`$85:CC08`) and redraws the label. The pad code (`$86:8000`) masks a
`$045A` button until released; for `$045C` buttons it unmasks them after 16
held frames, then every 5. So a held direction steps again at P+16 (P = the
press frame); a new direction is taken from P+4 even while Yomi still moves
(a one-frame tap at P+2 or P+3 is lost to the label redraw).

### Yomi

- Moves from P+1. `$85:C662`: half-cosine ease between her position and the
  target. `$87:CDB5` stores the midpoint in `$7F:2004/2006` and half the
  distance in `$7F:2000/2002`; each frame the phase `$7F:2008` += step
  (capped at `$80`), and `$87:CE36` [`$87:CDF3`] sets position = midpoint ±
  half × `T[phase]` × 2 / 256, T the cosine bytes at `$81:F462`
  (127 … −127). Step 20 when |dx|+|dy| to the midpoint is under 24, else 9.
  In the hall only 3 ↔ 0 is fast (7 frames); all other moves take 15.
- 136 → 72: 135 132 129 124 118 111 104 98 91 85 80 76 73 73 72 (the last
  two 73s are phases 117 and 126; the exact target comes in place of
  phase `$80`).
- Sprite: anim set `COP D8 $AB:DE71`, anim 3, attributes `$3D00`. 4 to 6
  OBJ of 8×8, palette 7, priority 3: wings `$1AC/$1AD/$1BC`, head
  `$1A8/$1A9/$1B9`, body `$1B8`, each a left/right mirrored pair. The flap
  cycle is 25 frames (compositions `$AB:DFEB`, `E053`, `E0BB`, `E08E`,
  `E0F6`, `E15E`, `E131`, `E199`, `E018`) and runs on while she moves. At
  (x, y) the head and body columns are at OAM x−8 and x−1, the head row at
  y−13..y−11 and the body 8 below (a 2 px bob), the wings at x−11 and
  x+1/x+2.

### Label

The text `$92:B23F` [`$92:B4DE`] (`COP 1C`, then `COP 1F`): `C4 01`, `C2`
window, `C8 00` (no delay), `CE A6 06 <table>` (entry `$06A6` of
`$92:B24E` [`$92:B4ED`]), `D4`. An entry is `C9 n` (glyphs start n×4 px into
the window: centred), the glyphs, `D4`.

- JP window `C2 0A 18 0E 02`: BG3 frame cols 9..24, rows 23..26; text at
  (80 + 4n, 192), glyph tiles `$60`.. palette 0.
- EU window `C2 02 18 1C 02`: frame cols 1..30, rows 23..26.
- After a move: empty window at P+2, the new label at P+4.

L (`$85:ACA0`): the description `$92:B2D3` [`$92:B583`], table `$92:B305`
[`$92:B592`], while L is held. JP grows the window in 8 `C2` steps from
(10,24) 14×2 to (2,23) 28×4; EU opens (2,23) 28×4 at once. On release:
`$92:8095` [`$92:80AF`] (`D7`, close), then the label. For example 7: そこは
道具の部屋だぜ。行ってみるかい？; 3: そこには プライムブルーが
はいっているんだぜ。使うのかい？

### HUD

BG3, the field's sheet and tiles (`docs/combat-graphics.md` §3), layout 1
(`$06BE` = 1), drawn into `$7F:D000` (cells col,row):

| Item | Cells |
|---|---|
| "LEVEL" `$3D $3E $2E` | (1..3, 2) |
| level | (5, 1..2) |
| "LIFE" `$02 $03` | (7..8, 2) |
| life "/" max | (10..14, 1..2), "/" at col 12 |
| "ITEM" `$0C $0D` | (18..19, 2) |
| gem icon | (24..25, 1..2) |
| gems | ends at col 28, rows 1..2 |

Labels palette 2, digits palette 3/4, all priority 1, as in the field. The
item icon is OBJ 0..3 at (160,8), palette 1, priority 3 (tiles
`$24 $25 $34 $35` with no item; `$0648` → `$09C7`). Same in both ROMs.

### A on a hotspot

`$85:ACB5` [`AD4D`]: Select → `$85:A333` (close). A on hotspot 2 goes to
`$85:ACF7`: `$8D:A080` (map available) or, if not, `COP 36 1B` and an
8-frame wait. A elsewhere: `COP 37 23`, close the label (`$92:8095`), and
jump through `$85:ACDC` [`$85:AD74`]:

| # | Target | Screen |
|---|---|---|
| 0 | `$85:BB9F` | options, over the dimmed hall |
| 1 | `$85:C239` | status, over the hall |
| 2 | `$85:ACF7` | map (`$06BC` = 1, `$85:A2D1`) |
| 3 | `$85:B2E6` | jewel box: magic screen |
| 4 | `$85:B499` | battle guide, over the hall |
| 5 | `$85:B7C8` | travel guide, over the hall |
| 6 | `$85:CD04` | weapon room (door animation, black, the room) |
| 7 | `$85:CCC3` | item room |
| 8 | `$85:CC81` | armor room |

### Open

- The pad word `$0454`'s key mapping (`$0634..063E`) and the meaning of
  `COP 36`/`37` (guess: sounds) were not traced.
- The window gradient's source (HDMA or colour math) was not traced.
- `$85:D1EE`'s role was not traced. The words `$06A8/AA/AC` are set by the
  door handlers (guess: the room each door leads to).

## The weapon and armor rooms

Captured with `palnative` as the hall, with weapons and armor poked into
the slots. P is the first frame with the button held; P+k is the k-th frame
after it. Addresses are Japanese, European in brackets; bank `$85` is
+`$98` in European up to `$85:CC65`, and the door handlers are
`CC81 → CD18`, `CCC3 → CD5A`, `CD04 → CD9B`.

The hall's map is 512×512 (32×32 cells of 16 px) and holds all four
screens: hall at camera (0,0), armor room (256,0), weapon room (0,256),
item room (256,256). The rooms use the hall's tiles, metatiles and layers
(stage `$0A`); only the camera, the sort and the pedestal icons differ.

| | Weapon room | Armor room |
|---|---|---|
| `$06BC` (screen) | 4 | 3 |
| Cursor | `$06AA` | `$06AC` |
| Room loop | `$85:B19B` [`B233`] | `$85:B049` [`B0E1`] |
| Stage (A from `$85:A40B`) | `$85:A6DB` [`A773`] | `$85:A641` [`A6D9`] |
| Slots, pedestals 4..19 | `$7F:8040..805F` | `$7F:8060..807F` |
| Power table | `$8D:BC92` + (id & `$3F`)×4 | `$8D:BD12` + (id − `$A0`)×4 |
| Equipped | `$064A`, bonus `$0659` | `$064C`, bonus `$065B` |
| Nav table | `$92:BFC5` [`$92:C173`] | `$92:BED5` [`$92:C083`] |

### Positions

`$92:BCA3[$06BC]` as in the hall (12 bytes: x, y, Up, Down, Left, Right);
x, y are map pixels (Yomi's entity x/y); screen = map − camera. Same
values in both ROMs. The two rooms have the same graph and screen
positions; the table below gives the screen ones (the ROM's: weapon y +
256, armor x + 256):

| # | x, y | Up | Down | Left | Right | What |
|---|---|---|---|---|---|---|
| 0 | 72, 56 | – | 8 | – | 1 | left door |
| 1 | 136, 56 | – | 3 | 0 | 2 | middle door |
| 2 | 200, 56 | – | 11 | 1 | – | right door |
| 3 | 132, 88 | 1 | 9 | 9 | 10 | ホールホル (the trash hole) |
| 4 | 116, 122 | 9 | 6 | 12 | 5 | slot 0 (fixed) |
| 5 | 148, 122 | 10 | 7 | 4 | 13 | slot 1 (fixed) |
| 6 | 116, 154 | 4 | 17 | 14 | 7 | slot 2 (fixed) |
| 7 | 148, 154 | 5 | 18 | 6 | 15 | slot 3 (fixed) |
| 8 | 68, 106 | 0 | 12 | – | 9 | slot 4 |
| 9 | 100, 106 | 3 | 4 | 8 | 10 | slot 5 |
| 10 | 164, 106 | 3 | 5 | 9 | 11 | slot 6 |
| 11 | 196, 106 | 2 | 13 | 10 | – | slot 7 |
| 12 | 52, 122 | 8 | 14 | – | 4 | slot 8 |
| 13 | 212, 122 | 11 | 15 | 5 | – | slot 9 |
| 14 | 52, 154 | 12 | 16 | – | 6 | slot 10 |
| 15 | 212, 154 | 13 | 19 | 7 | – | slot 11 |
| 16 | 68, 170 | 14 | – | – | 17 | slot 12 |
| 17 | 100, 170 | 6 | – | 16 | 18 | slot 13 |
| 18 | 164, 170 | 7 | – | 17 | 19 | slot 14 |
| 19 | 196, 170 | 15 | – | 18 | – | slot 15 |

Position p ≥ 4 is slot p − 4: `$7F:8040 + 2(p−4)` (weapon), `$7F:8060 +
2(p−4)` (armor). Slots 0..3 are the fixed items (`$9C..$9F`, `$BC..$BF`),
4..15 the 12 shared slots. `$85:C6B1` [`C749`] (weapon), `$85:C695`
[`C72D`] (armor) set `$0DD0` = item, `$0DD2` = count. The pad, the
repeat timing, Yomi's ease (`$85:CC08`) and the label redraw (P+2 empty,
P+4 new label) are the hall's (`$85:CB2A`).

Doors: weapon 0 → hall (hotspot 6), 1 → armor room position 1, 2 → item
room position 2; armor 0 → item room position 0, 1 → weapon room
position 1, 2 → hall (hotspot 8). From the hall the weapon room starts at
0, the armor room at 2 (`$85:CD04`, `$85:CC81` write the cursor).

### Entering: the sort

The stage sorts the 12 shared slots before drawing (`$85:A6FD..A74C`,
armor `A663..A6B6`; a lone `$81` in `$7F:8048` stays there, so the four
fixed slots are left alone): the power of each slot (0 when empty) into `$7F:80A0`, then
a bubble sort, descending, swapping only when the next is strictly
greater (stable). So empty slots go last, and the saved order changes. With
`$81` (3), `$82` (6), `$80`×2 (4) the slots become `82 01 80 02 81 01`.
Both ROMs.

### Transition

`$85:CD04` (or another door handler): set the target cursor, `$7F:201A` =
target screen, `$7F:201C` = door 0/1/2 (centres `$85:C954`: (72,52),
(136,52), (200,52)), spawn the door task, then `$85:CD43`. From the hall,
A first runs `COP 37 23` and the label close (above). JP:

| Frame | What |
|---|---|
| P+2 | label closed; sound (`COP 37 1A`); door half open (metatiles `$F3/$F2`, `$85:C9C8`) |
| P+3 | door open (`$F5/$F4`) |
| P+2..P+66 | the window closes on the door (below); angle `$7E:46E8` low byte +4 a frame (JP has one lag frame at P+8; EU ends at P+65) |
| P+40 | door half closed, Yomi's priority 2 (behind the door) |
| P+45 | door closed (`$F1/$F0`) |
| P+62..P+114 | black (LUM 0); EU P+61..P+108 |
| P+67 | `$7E:46E6` = 0, P+68 `$06BC` = target, loads (Ark → `$85:A246`) |
| P+99 | Yomi at the target position, priority 3, script `$85:C616`; the window opens (EU P+93) |
| P+165 | window done (`$46E6` = 0), P+167 `$7E:5DC4` = `$FF00` (EU P+157, P+159) |
| P+169, P+171 | empty label window, then the label (EU P+161, P+163) |

Back to the hall (weapon door 0): the same steps; the hall label comes at
P+162 (JP). Room to room (armor 1 → weapon 1): label at P+171 (JP).

The window is a rotating quadrilateral, visible inside, black outside
(polygon list `$7E:46E6` = 4 vertices at `$7E:46EE`, 6 bytes each;
`$8D:AFE0`). Close (`$85:C725`): centre = door centre + camera; four child
tasks ease the vertex offsets with the half-cosine ease (`$87:CDBD`,
`$87:CE36`, phase +2 a frame, 64 frames) from (−256,−226), (−256,226),
(256,226), (256,−226) (`$85:C95A`) to (±1,±1); the angle goes 0 → `$100`.
Open (`$85:C7F3`): the same offsets backwards, the angle 0 → −`$100`, and
the centre eases from the door to (128,112) with phase +1 a frame (so it
is half way when the window is done).

### Pedestals

`$85:CE9E` [`CF36`] (each stage): fills `$7E:3F00..3FFF` (metatiles
`$1E0..$1FF`) with tiles `$180..$1FF`, then for slot k with an item ORs
`$2000 | $B1:DA31[item] << 10` (priority 1, palette) into the four words of
metatile `$1E0+k`, and DMAs the icon to VRAM `$1800 + $40k`: the shop
icon (`$A8:8000` chain, `$A2:8000` tiles, `assets::shop_display::item_icon`),
two rows of 64 bytes. An empty slot gets item 0's tiles. Palette n is BG
palette n, colours 8..15 = `$AF:E43B + 32n` (the hall's overrides; the
stage also loads `$AF:E4DB` → colours `$58..$5F`).

Layer 2 cells (col,row) of metatile `$1E0+k`:

| k | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| weapon | 7,23 | 9,23 | 7,25 | 9,25 | 4,22 | 6,22 | 10,22 | 12,22 | 3,23 | 13,23 | 3,25 | 13,25 | 4,26 | 6,26 | 10,26 | 12,26 |
| armor | 22,7 | 26,7 | 22,9 | 26,9 | 20,6 | 23,6 | 25,6 | 28,6 | 19,7 | 29,7 | 19,9 | 29,9 | 20,10 | 23,10 | 25,10 | 28,10 |

When a throw empties a slot, `$85:C97A` [`CA12`] queues a fixed-source DMA
(`$A2:8000`, `$80` bytes) to the slot's VRAM, so the icon goes.

### Sprites over the pedestals

Task `$85:D00B` [`D0A3`] (entity `$1580` in the captures), each frame:

- On a pedestal (p ≥ 4) with an item that is not the equipped one: the
  power difference new − equipped (equipped 0 when none). Arrow tile `$5C`
  (V-flipped when negative) at (x+13, y + b − 2), b = (`$42` & 15) >> 2
  (gain), or y − b + 2 (loss); the digits (`$40` + digit) at y − 8, one
  digit at x+13, two at x+10 and x+17. Palette 0, priority 3.
- On the equipped item's pedestal: tile `$1BF`, palette 7, priority 3, at
  (x+8, y−8) (`$85:E6FE`).

### Labels and texts

| Text | JP | EU |
|---|---|---|
| Pedestal: name (`$92:8179`[`81A4`] entry `$0DD0`), count if > 1 | `$92:B47C` | `$92:B6FD` |
| Door/hole labels (table: 0 中央, 1 防具/武器, 2 道具/中央, 3 ホールホル) | weapon `$92:B66F`, armor `$92:B619` | `$92:B889`, `$92:B856` |
| L on a pedestal: power line, description (`$92:8E3A` [`8EA4`]) | `$92:B5B1` | |
| L on 0..3 | `$92:B68B`, `$92:B635` | |
| A on an empty pedestal: 何もないぜ！ | `$92:B6C5` | `$92:B8BC` |
| Equip? (装備する / 装備しない) | `$92:AB47` | `$92:ADE5` |
| 装備したぜ！ | `$92:AB84` | `$92:AE22` |
| Unequip? (はずす / そのまま) | `$92:AB1D` | `$92:ADC5` |
| Unequipped | `$92:AB6D` | `$92:AE04` |
| 何を すてるんだい？ (on 0..3 / with the name) | `$92:ABB8` / `$92:AB98` | `$92:AE57` / `$92:AE39` |
| 本当に すてていいのかい？ (はい / いいえ) | `$92:ABCD` | `$92:AE6C` |
| じゃあ すてるぜっ！ | `$92:ABF3` | |
| Can't throw (flag) | `$92:AC07` | `$92:AEB0` |
| アーク！ 戦いを やめる気かいっ！ (last one, waits for a button) | `$92:AC4C` | `$92:AF08` |

The label window is the hall's. The question windows are `C2 02 17 1C 04`
(cols 2..29, rows 23..26), 2 frames a glyph; L grows the window as the
hall's.

### Input

Room loop (`$0454`): A, B or Select first, then L (description while
held, then the label again), then the pad.

- B: cursor to the hall door (weapon 0, armor 2), Yomi moves there.
- Select: `COP CB 01 $85:A333`, the menu closes (not measured).
- A on 0..2: the door (`$85:CC49` [`CCE1`], armor `$85:CC65`).
- A on 3: throw mode (below).
- A on a pedestal: `COP 00 $85:AE8E` [`AF26`], then `$85:F4BD` [`F555`]
  (stats), then the loop again.

`$85:AE8E`: empty → 何もないぜ！ (P+3, gone at P+21, label P+22). A weapon
(`$80..$9F`) or armor (`$A0..`): if it is the equipped one, the unequip
question, else the equip question; `COP 1A 0A` (cursor on the first
answer; table [B, first, second]: B and the second answer close the
window). First answer: equip writes `$064A`/`$064C` = item, unequip
writes 0, then the answer text, a 30-frame wait, close.

| Frame (JP, A on the first answer) | Equip | Unequip |
|---|---|---|
| P+1 | `$064A` = item | `$064A` = 0 |
| P+3..P+13 | 装備したぜ！ typed | |
| P+46 | window closed; `$0659` = power (3 for `$81`) | |
| P+48, P+50 | empty label window, label | |
| P+52 | | `$0659` = 0 |

The question itself: P+2 window, glyphs from P+3, complete at P+19, the
cursor blinks every 16 frames. `$0662` stayed 3 in the capture. Armor:
same with `$064C`, `$065B` (cloth `$A0`: 3). EU: the same flow (equip
`$82`: `$064A` = `$82`, `$0659` = 6); its question appears at P+2, glyphs
from P+3, and the equip answer is longer (window closed at P+68, label at
P+72).

Throw mode (`$85:B220` [`B2B8`], armor `$85:B0CE`): 何を すてるんだい？ at
P+3 (instant); pedestals show the name on line 2 (何もない when empty).
The pad moves Yomi as before; B → back to the room loop (Yomi stays); A:

- empty pedestal or 0..3: `COP 36 1B` only;
- the byte before the item's description in `$92:8E3A` non-zero
  (`$85:CE50`; e.g. `$9C`): `COP 36 1B`, `$92:AC07`;
- fewer than 2 non-empty slots of 16 (`$85:CE82`, armor `CE66`): the last
  one text, waits for a button;
- else the question; はい: じゃあ すてるぜっ！, `$8D:96A0` takes one
  (`$09C7` = item), the icon goes when the count reaches 0, and if the
  equipped item has none left (`$8D:9628`) `$064A`/`$064C` = 0 and the
  stats again. Window cleared P+31, throw-mode text P+32.

### Open

- The polygon rasteriser (bank `$89`, `$7E:46E8` users) and the window
  registers were not traced; only the vertex and angle values above.
- The door metatiles `$F0..$F5` go through `$8D:8C63` (the cell from
  x, y); their art was not checked.
- `$7F:0702` = 0, `$7F:071C` = `$FFFF` (both stages) were not traced.
- Select in a room, the European exit timings, the item room, and the
  meaning of `COP 1A 0A` beyond the jump table were not measured.
