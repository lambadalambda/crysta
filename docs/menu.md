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
| weapon, armor doors | 12 pedestals each; A equips (`$064A`, `$064C`) |
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
  two frames: phase `$80` gives 73, then the exact target).
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
