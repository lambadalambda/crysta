# Combat graphics

Sources for the graphics that [combat](combat.md) needs: Ark's thrust with the
spear, the damage digits, the explosion, the gem drop and the tower HUD.
Addresses are Japanese (JP); the table in §5 gives the European (EU) ones.
All data bytes are equal in both ROMs; only the addresses move.

Evidence: source reads, and native captures (poseprobe `cap`/`log`) from the
JP checkpoint `first-tower-neutral-stable` (map `$101`) with `$064A=$81`,
`$0659=3` poked, thrusts in all four facings (17 frames each). For the kill:
blob slot `$1080` poked to (112,592), layer `E+$16=0`, life 1, Ark
`7F:1020=200`; A two frames after the pokes gives a gem drop. EU: the same
thrust from `tmp/tower/eud/ctl.state` plus route lines 579-615.

## 1. Ark's spear thrust

### Tiles: all from resource 4's sheet

There is **no separate weapon tile upload**. Every part of resource 4 (palette 0
and palette 1 alike) indexes resource 4's raw graphics window. The table
`$80:A24F` entry 4 is `00 A0 A5 | 00 C0 A0`: base `$A5:A000`, graphics
`$A0:C000` (EU `$A7:A000`, `$A2:C000`). The graphics are raw 4bpp, 16 tiles a
row, 512 tiles (`$4000` bytes, not compressed). `$A5:C0A0` is a misreading.

| Item | Source |
|---|---|
| Tile n (low 9 bits of the part word) | 32 bytes at `$A0:C000 + 32·n` |
| Large part n | n, n+1 (top), n+16, n+17 (bottom) |
| Spear shaft/tip | `$0E $1E $2E $3E`, `$0F $1F`, `$05 $15`, `$40-$47`, `$1C6 $1D6 $1E6 $1F6`, `$1D3`, `$1DC-$1DF`, `$1E2`/`$1E4` (large) |

All weapons share these tiles (the thrust scripts hard-code resource 4). Only
palette 1 changes per weapon.

Upload (per entity draw, `$80:F0B1` → `$80:F189`, `$80:F136`): the
composition's distinct source tiles are listed in order (`$7F:0CC2`). Distinct
tile i goes to OBJ slot `s = 2i` for i < 8, else `2i + 16` (`$80:F08A`), as a
16×16 block: the top row (n, n+1) to OBJ tiles s, s+1 (VRAM word
`$4000 + 16·s`), and the bottom row (n+16, n+17) to s+16, s+17. A small part
uses only the top-left tile of its block. The OAM tile is s. The thrust needs
4-9 slots (OBJ tiles `$00-$1F`, and `$20`/`$30` for the 9th).

Measured: the bottom rows reach VRAM **one frame after** the top rows. On
the first frame of each new composition (Right, Down: every record; Up: none
in these captures), VRAM still holds the previous record's bottom rows. Guess:
this tearing shows on screen for one frame.

### Palettes

| OBJ palette | CGRAM | Contents |
|---|---|---|
| 0 | `$80-$8F` | `$B1:D831`, 16 colours (`COP 5A`, `$80:F941`) |
| 1, colours 0-7 | `$90-$97` | `$B1:D851`, 8 colours (`$80:F915`: `COP 5A B1 D851 90 08`): 0 = transparent, 1 = `$1063` |
| 1, colours 2-7 | `$92-$97` | weapon colours, overwritten by `$85:D1F8` (below) |
| 1, colour 15 | `$9F` | `$7BDE` (shared OBJ palette `$B2:8B38` +30, the map load's `40 00 20 90`) |

Palette-1 parts use colour indices 1-7 and 15 only. Colours 8-14 belong to the
room (in `$101`: `$0186 $0249 $0374 $2100 $4660 $0153 $02DF`).

`$85:D1F8` (JSL, A = forced index or 0):

```
k = A ? A : ($064A ? $85:D25E[$064A - $80] : 0)
if k == $04D0 (cache): return;  $04D0 = k
copy 12 bytes $B1:D871 + 12·k  ->  $7F:0724 (CGRAM $92-$97)
  ($048A & $4000: no copy; & $0001 without $0400: $7F:07E4 (pal 7), with $8000: $7F:07A4 (pal 5))
```

Callers: `COP CB flags addr` (`$80:ADBD`, Ark's script start; flag bit 1 forces
k = 3), and the menu (`$85:B4B1`: `$04D0 = −1`, k = 3). A thrust runs `COP CB 00`,
so the colours change on the first action after equipping. Measured: the poke
alone leaves k = 0; the thrust's first frame (P+1) writes k = 3.

Weapon map `$85:D25E` (32 bytes, items `$80-$9F`):
`00 03 00 05 02 03 03 01 03 06 04 05 01 04 07 06 05 00 07 01 06 00 04 00 00 00 00 00 01 03 03 02`.
The spear `$81` → k = 3 → `$B1:D895`: `$44C0 $5160 $6200 $6682 $6F24 $7BCA`.
Eight entries k = 0..7 at `$B1:D871..D8D1`. Entry 0 (no weapon): `$00CC $00F0
$0134 $15F7 $2ABA $3F9E`.

### Raster recipe (a thrust frame)

1. Composition c from the list (resource 4 lists `$00` Down, `$01` Up, `$02`
   Right, Left = Right mirrored); parts at c+13, 7 bytes each.
2. For each part, in order (an earlier part wins on overlap): size, then the
   X/Y offsets and mirror rules from [ark-sprites](ark-sprites.md). Pixel Y is
   OAM Y + 1.
3. Tile pixels: sheet `$A0:C000` at tile n (large: n, n+1, n+16, n+17), with the
   part's H/V flip, XORed with the entity's mirror.
4. Colour: palette bits 9-11. 0 → `$B1:D831[i]`. 1 → i = 1: `$1063`, i = 2..7:
   `$B1:D871 + 12·k` colour i−2, i = 15: `$7BDE`. Index 0 is transparent.
   Priority is 2 for all thrust parts.

Verified (JP, all 4 facings, 66 frames; EU Up, 17 frames): every Ark OAM entry's
VRAM tiles equal the sheet tiles of the matching part, and the palettes equal
the ROM sources. The one exception is the bottom-row lag above.

## 2. Shared OBJ sheet: digits, gem, explosion

The map load uploads one LZ packet (see [compression](compression.md)):
`$A9:F02F`, 8192 bytes = 256 4bpp tiles → VRAM word `$4000` (OBJ tiles
`$00-$FF`, subscript `80 00 10 00 …`). In `$101` VRAM equals the packet
except Ark's slots (`$00-$1F`), the HUD item icon (`$6E $6F $7E $7F`) and
room art (`$B0-$B9`, `$C0-$C9`). Tile n is 32 bytes at packet offset 32·n.
Hazard: the explosion and gem tiles `$2A-$2F`/`$3A-$3F` are Ark slots 13-15.
Thrusts use at most 9 slots.

OBJ palette 2 (CGRAM `$A0-$AF`) = `$B2:8B58` (map load, `40 00 20 90` covers
`$B2:8B38..8B78` → `$90-$AF`).

### Damage digits

`COP D5 dy attr` (`$80:B3B6`) spawns `$87:A1BF` with `E+$08 = attr << 8`
(`& $FEFF`) and the BCD value from `7F:201A`. `$85:E55C` emits one 8×8 OBJ per
digit (no leading zeros): tile `$40 + d`, attribute `E+$08`. The digits are
7 px apart, from x − 3·count. Measured (1 digit): OAM = (x − 3 − camX,
y − camY − 1).

| `COP D5 00 …` | Attr | OBJ palette | Colour 1 | Colour 14 (fill) |
|---|---|---|---|---|
| `38` normal | `$3800` | 4, priority 3 | `$0000` | `$7FFF` (`$8D:AC2D` → `$7F:079C`) |
| `3A` critical | `$3A00` | 5, priority 3 | room (`$101`: `$1042`) | cycles every frame: `$03FF $7BFF $0000 $7C00 $001F $7C1F $03E0 $7FE0` |
| `36` damage to Ark | `$3600` | 3, priority 3 | `$0000` | `$08DF` (`$8D:AC26` → `$7F:077C`) |

The digit tiles use only colours 1 (outline) and 14. The critical cycle is
palette animation 9 (`$DA:8000` table → `$DA:897B`: 8 records `01 src DE 01 00`,
colours at `$DA:89AC`). Guess: the map's palette-animation actor `$87:98EB`
runs it; CGRAM `$DE` holds a cycle colour in most journey checkpoints.
Measured: normal digit OAM tile `$45`, palette 4, priority 3. `3A` and `36` are
from source only.

### Explosion (pose `$16`)

At death the enemy runs `COP D8 00 C0 A2` (animation base `$A2:C000`),
`E+$08 = $3000`, `COP 80 16`. List `$16` at base `$A2:C000` has 13 records of
2 frames (compositions `$A2:D6B7 D6E4 C7D6 D6FC D745 D79C D80F D882 D8FC D97D
D9D4 DA0F DA35`). The part tile numbers are OBJ tile numbers directly (no
upload): `$2A-$2D $3A-$3D $50 $5F $80-$83 $8A-$93 $9A-$A2 $A9-$AC`, all palette
2. Priority = part 2 | `$3000` → 3. Measured: OAM tiles, palette 2 and
priority 3 match each record.

### Gem drop (poses `$0B`/`$0C`/`$0D`, same base `$A2:C000`)

| List | Gems | Tiles (8×8, palette 2) |
|---|---|---|
| `$0B` | < 10 | `$4D`, `$4E`, `$5E`, `$4F`+`$5E`, `$5F`+`$5E` (13 records) |
| `$0C` | < 100 | `$2E`/`$3E` (8×16), `$A1`, `$A2`, `$4F`, `$5F` (10 records) |
| `$0D` | ≥ 100 | `$2F`/`$3F` with H-flip copies, `$2C` (16×16), `$8E` (16×16), `$4F`, `$5F` (12 records) |

Measured (`$0B`): OAM tiles `$4D`, `$4E`, `$5E`, `$4F`, `$5F`, palette 2,
**priority 2** (no `$3000` here), with the bounce from the entity height.

## 3. HUD (BG3)

| Item | Source |
|---|---|
| BG3 chars (2bpp, VRAM word `$7000`, BG34NBA = 7) | LZ `$A9:9000`, 16 bytes a tile. Tiles `$00-$5F` match in `$101`; `$60-` are text glyphs |
| BG3 map | `$7F:D000` → VRAM `$6800` (combat.md §7) |
| BG3 palettes (CGRAM `$00-$1F`, 4 colours each) | `$B2:8B78`, 64 bytes (map load `40 00 20 00`); colour 0 is the room's backdrop |

| BG3 palette | CGRAM | Colours 1-3 | Used by |
|---|---|---|---|
| 2 | `$09-$0B` | `$5E3F $7EF1 $0000` | labels, frame `$2820` |
| 3 | `$0D-$0F` | `$0C43 $0FFF $7FFF` | digit tops, "/" top, gem icon |
| 4 | `$11-$13` | `$0C43 $01DF $033F` | digit bottoms, "/" bottom |

Tiles (from the `$A9:9000` packet): "LEVEL" `$3D $3E`, a box edge `$2E`,
"ITEM" `$0C $0D` (row 1, cols 4-8), "LIFE" `$02 $03` (cols 24-25), digits
`$21-$2A` top and `$31-$3A` bottom, "/" `$2F`/`$3F`, gem icon `$40 $41 / $50 $51`,
frame `$20` (solid colour 3). The labels are palette 2, priority 1 (`$28xx`). Digits and the gem icon are
palette 3/4, priority 1.

The HUD item icon is OBJ, not BG3: OAM 0-3, tiles `$6E $6F $7E $7F`, palette 1,
priority 3, at (56,16). Its upload is not traced.

## 4. Verification summary

| Check | Result |
|---|---|
| Thrust tiles vs `$A0:C000` (JP 4 facings, EU Up) | equal (except the one-frame bottom-row lag) |
| OBJ palette 1 after the thrust | `$B1:D851` 0-1, `$B1:D895` 2-7, `$7BDE` 15 (EU `$B3:DE16`) |
| VRAM `$4000` digits/gem/explosion tiles at hit, explosion, gem | equal to `$A9:F02F` |
| VRAM `$7000` tiles `$00-$5F` | equal to `$A9:9000` (EU `$AB:9000`) |
| CGRAM `$01-$1F` | `$B2:8B7A..` (EU `$B4:90DD..`) |
| EU data | `$A2:C000` sheet, both LZ packets, all palettes, `$85:D2F6` map, the `$DC:897B` cycle: byte-equal to JP |

## 5. EU addresses

| JP | EU |
|---|---|
| res4 `$A5:A000`, sheet `$A0:C000` | `$A7:A000`, `$A2:C000` |
| `$85:D1F8`, map `$85:D25E`; callers `$80:ADDD`, `$85:B4B1` | `$85:D290`, `$85:D2F6`; `$80:ADDD`, `$85:B549` |
| `$B1:D831` / `D851` / weapon colours `D871` (spear `D895`) | `$B3:DDB2` / `DDD2` / `DDF2` (spear `DE16`) |
| `$B2:8B38`, `8B58`, `8B78` | `$B4:909B`, `90BB`, `90DB` |
| LZ `$A9:F02F`, `$A9:9000` | `$AB:F02F`, `$AB:9000` |
| digits `$87:A1BF`, drawer `$85:E55C` | `$87:A17C`, `$85:E5F4` |
| explosion `$85:E365` `COP D8 00 C0 A2` | `$85:E3FD` `COP D8 00 C0 A4` (base `$A4:C000`) |
| `$8D:AC09` (digit colours 14), `$80:F915`, `COP CB` `$80:ADBD` | same |
| cycle `$DA:8000`/`897B`/`89AC` | `$DC:8000`/`897B`/`89AC` |

## Open questions

- The thrust's one-frame bottom-row lag: is it visible on screen, or does it
  only show in the capture timing?
- The HUD item icon upload (`$6E`/`$7E`, palette 1) and its source.
- Who starts palette animation 9 in each map (guess: `$87:98EB` actors).
- The critical (`3A`) and Ark-hit (`36`) digits were not captured on screen.
- The `$048A` destination variants of `$85:D1F8` (palettes 5/7) were not observed.
