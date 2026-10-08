# World map `$03`: Mode 7 view and tower entrances

Research for [world-map-mode7](../meta/issues/world-map-mode7.md) and
[enter-the-towers](../meta/issues/enter-the-towers.md). JP addresses unless
marked; EU differences are listed at the end. "Guess" marks an unverified
reading.

## Result

A Python model of the ares PPU (`local/mode7/tools/m7.py`), fed only by the
ROM HDMA tables, VRAM, CGRAM, OAM and six per-frame registers, reproduces
native frames **pixel for pixel**: 3 JP frames (standing, walking down,
walking left) and 3 EU frames, 0 differing pixels each. Palette and tile
animation come from the CGRAM/VRAM snapshot; the model does not compute it.

## Per-frame registers

| Register | Value | Source |
|---|---|---|
| BGMODE | 7 | scene profile `$14` |
| M7A..M7D | shadows `$56/$58/$5A/$5C` | zeroed by setup `$87:990B`. HDMA rewrites A and D on every line. B = C = 0: **no rotation** (never non-zero in the captures). |
| M7X | `$5E = ((x-128) & $3FF) + 128` | camera `$87:916D..919E`, x/y = player `$1000/$1002` |
| M7Y | `$60 = ((y-160) & $3FF) + 160` | same |
| M7HOFS/M7VOFS (BG1HOFS/VOFS) | camera `$081E/$0822` = (x-128, y-160) | `$080E/$0812` are written in the same routine |
| NMI upload | `$85:FA44..FA7D` writes `$56..$61` to `$211B..$2120` | |
| CGWSEL | `$82` (from the state): clip main to black inside the colour window; sub screen as the second operand | |
| Windows | W2 = [8,247], inverted. The colour window clips main to black at x<8 and x>247. W2 also masks BG1 on main and OBJ on sub. | |
| OBSEL | `$02` (8x8/16x16, base `$4000` words) | |

M7HOFS-M7X = -128 and M7VOFS-M7Y = -160 always. Ark is the screen point
(128,160). The plane wraps at 1024 (M7SEL repeat bits = 0).

## HDMA (setup `$87:990B..995D`, all tables in ROM bank `$87`)

`COP 4F`/`COP 4E`/`JSL $8D:92C3` program 7 channels. "Line k" is table
element k. It is written at the end of scanline k and is in effect on
vcounter k+1 (screen row k, frame-buffer row k+9 in our 256x240 capture).

| Ch | Target | Mode | Table | Content |
|---|---|---|---|---|
| 1 | M7A `$211B` | 2, indirect | `$87:9992` (`F0 99AB`, `F0 9A8B`, `A0 9B6B`, `00`) | one word per line. Data = 224 contiguous words at `$87:99AB` |
| 2 | M7D `$211E` | 2, indirect | same table | M7D = M7A |
| 3 | M7SEL `$211A` | 0 | `$87:999C`: `34 02`, `01 00` | lines 0-51: V-flip. After that: 0 |
| 4 | MOSAIC `$2106` | 0 | `$87:9C1F`: `34 11`, `01 00` | lines 0-51: BG1 2x2 mosaic |
| 5 | CGADSUB/COLDATA `$2131/32` | 1 | `$87:9B6B` | lines 0-51: `$A1`, fixed 24. From line 52: `$81`, fixed F(k) |
| 6 | CGADD/CGDATA `$2121/22` | 3 | `$87:9B9C` | backdrop CGRAM[0] gradient, lines 0-51 |
| 7 | TM/TS `$212C/2D` | 1 | `$87:9C24`: `34 00 01`, `10 01 10`, `01 11 00` | 0-51: TM 0, TS BG1. 52-67: TM BG1, TS OBJ. 68+: TM BG1+OBJ, TS 0 |

- A(k) (hex): `D3 D4 D6 ... 1CB 1E4` for k=0..51 (sky, increasing). Then
  `23F` at k=52, falling to `B0` at k=223. This is a table, not a simple
  1/z (a linear fit is off by 36). Copy it from ROM.
- F(k), all three channels equal: 15 at k=52, then 14 at 54, 13 at 56, 12 at
  58, 11 at 60, 10 at 62, 9 at 64, 8 at 66, 7 at 69, 6 at 72, 5 at 76, 4 at
  80, 3 at 85, 2 at 91, 1 from 98 to the frame end (the table ends at 105).
- Backdrop (start line, colour): 0 `7227`, 1 `7648`, 2 `7A69`, 3 `7E8A`,
  5 `7EAB`, 7 `7ECC`, 10 `7EED`, 14 `7F0E`, 20 `7EED`, 25 `7ECC`, 29 `7EAB`,
  32 `7E8A`, 35 `7A69`, 37 `7A48`, 39 `7627`, 41 `7606`, 42 `71E5`,
  43 `6DC4`, 44 `6582`, 45 `5D40`, 46 `5500`, 47 `4CC0`, 48 `4480`,
  49 `3C40`, 50 `3400`, 51 `2C00` (stays `2C00` below).

## Per-line formula (ares `PPU::Background::runMode7`)

For screen row r (0..223), v = r+1, k = r, A = D = A(k) (signed). Then:

```
if k < 52:  y = 255 - (v - ((v-1) & 1)); xs = X & ~1   # mosaic 2, V-flip
else:       y = v;                       xs = X
ox = (A*clip(H-X0) & ~63) + (X0 << 8)              # B = 0
oy = (D*clip(V-Y0) & ~63) + (D*y & ~63) + (Y0 << 8)  # C = 0
u = (ox + A*xs) >> 8 ;  w = oy >> 8                  # map pixel, & 1023
clip(n) = n & 0x2000 ? n | ~1023 : n & 1023
```

Each line samples one map row w and scales by A/256 around Ark's column.
Ground examples: line 52 (the horizon) samples 241 px ahead of Ark, rows
159-160 sample Ark's row ±1, row 223 samples 44 px behind. Sky lines sample the
map mirrored (V-flip) and pixelated: the blue "rays".

Compositing:
- k<52: main = backdrop. Sub = BG1 colour, or fixed 24 where BG1 is index 0.
  Result = backdrop - sub (subtract, no halve). OBJ is not shown.
- k 52-67: main = BG1 (backdrop `2C00` where index 0, no math there). BG1
  minus (OBJ colour on sub if present, else F(k)). This is the black fog
  band at the horizon.
- k>=68: BG1 - F(k). OBJ on main without math. Mode 7 BG1 priority is 2.
  OBJ priorities 0-3 map to 1/3/4/5, so OBJ priority 0 is behind BG1.
- x<8 or x>247: black.

## Sprites

No scaling. Nothing on the map is a sprite except:
- Ark, OAM 0-2 (16x16 + two 8x8, tiles `$00..$04`, palette 0), at a fixed
  screen box around (120..136, 137..161). This is the world-map pose, not
  the town sprite (its source was not traced).
- 40 static OBJ (OAM 3-42, tiles `$101..$11F`, palette 5, y 27..63). These
  are the same in every frame. Because of TM/TS they only darken the
  horizon band (lines 52-67); none reaches line 68. Their writer is the
  type-01 record `$83:8909` on `$03` (script `$84:E3E6`, EU `$84:E3AB`;
  descriptor `$83:F92E`): after `COP BC` it sets its place to the camera
  plus (`$80`, `$35`) every frame, so it is fixed on the screen at
  (128,53). Its pose packet `$B1:BE49` (unpacked to `$7E:7000`) is the
  piece list; +$08 = `$0100` takes the second name table. Palette 5 is a
  grey ramp (1..30).
- The towers and Crysta are **map tiles** in the `$03` layer, not sprites.

The FB record `$87:990A` (EU `$87:98C7`) sets up the per-line HDMA above
(M7A/M7D scale, M7SEL, mosaic, the fog's CGADSUB/COLDATA, the backdrop,
TM/TS) when its parameter (+$06, `$80:F70E`) is 0, as on `$03` and `$23`.
Map `$02` passes 1: CGADSUB `$83`, the A/D table `$87:99A1`, COLDATA
`$9E4F` or `$9E2E` by the flag word `$815E` (`COP 08`; not traced). The
hosts draw these tables (`mode7.rs`); the runtime does not run either
script (`residents.rs` `SERVICES`).

Animation (guess): three actors `$87:98BF` (params `$0A/$0B/$1D`, from the
`$83:88FB` spawn list, `$8D:9331/$93B2`) and `$87:98EB` (`$8D:93D8/$940D`,
VRAM DMA queue `$8D:9470`) step palette and char animations, like the lava
and ice glints. A renderer must apply them to CGRAM/VRAM itself.

## Tower entrances (exit list `$81:8CC1`, 12-byte records, cells of 16 px)

| Record | Cell (x,y) w×h | Destination | Sel | Raw arrival |
|---|---|---|---|---|
| `$818CC1` | (32,31) 2×2 | `$0A` Crysta | `$66` | (496,960) |
| `$818CCD` | (13,49) 1×2 | `$100` tower 1 (southwest) | `$66` | (248,992) |
| `$818CD9` | (5,36) 1×2 | `$107` tower | `$66` | (248,992) |
| `$818CE5` | (30,8) 1×2 | `$10E` tower | `$66` | (248,992) |
| `$818CF1` | (46,11) 1×2 | `$115` tower | `$66` | (248,992) |
| `$818CFD` | (55,41) 1×2 | `$11C` tower | `$66` | (248,992) |
| `$818D09` | (41,43) 1×1 | `$12A` (guess: not a tower) | `$66` | (1144,464) |
| `$818D15` | (43,29) 1×1 | `$12B` (guess: not a tower) | `$66` | (120,944) |
| `$818D21` | (41,36) 1×1 | conditional `$F046` | — | — |

- The mechanism is the ordinary exit scan (`$8D:8797`) on origin
  (x-8, y-16). There is no script or trigger cell. Natively, Ark walked up
  from (216,880) and the map changed on the first frame of the step from
  (216,816): position 814, frame 60426 → `$100` at 60427. Guess: the scan
  runs before that frame's movement.
- No story flag gates the five tower exits on `$03`. The conditional record
  (`$8D:8911`): `$81:F046` is a list of 10-byte entries (flag word,
  destination, mode, selector, x, y), ending in `FFFF`. Exit byte 6 bit 7
  inverts the sense. Its only entry is flag `$74` → `$127` (`$66`, (248,448)).
  Without the flag there is no exit. The same cell (41,36) is the one the
  `$D8:0000` patch redraws under flag `$402`.
- Gating of towers 2-5 inside their own maps is not researched. Their loading
  scripts share subscript `$45` with `$100`.
- First tower, already qualified in `tools/tower-approach-qualification/TOWER.md`:
  `$100` → native arrival (256,1007) at 60666. The intro sets `$100`; the
  guardian `$8288CA` → `$908BF0` sets `$115`. Then exit `$81C2EE` (cell
  (15,54) 2×2, sel `$62`, raw (120,608)) → `$101`, arriving at (128,623).

## What our pipeline lacks

App: a Mode 7 renderer that implements the above. Today `plane.rs` +
`background.rs` draw `$03` flat. It needs the 128×128 Mode 7 tilemap and
8bpp chars (already decoded by `WorldMap`), the A(k) table, the
sky/fog/backdrop HDMA, the horizon OBJ band, the borders and the palette/char
animation.

Runtime and assets, for the towers:
- `admitted()` rejects `$100+`, so `enter_exit` returns `None` and tower exits
  do nothing. `ExitError::ConditionalDestination` is not evaluated
  (`$F046`).
- `SpawnList::from_rom` reads only `$83:8000`. Maps `$100`/`$101` use the
  `$82:8000 + 2*map` override (`$8288C1`, `$8288FD`), so today they are
  `Absent`.
- `StaticBackground::from_rom` allowlists `$0A..$21` and `$128` only. With
  two local edits in a scratch copy (allow `$100..$130` through
  `projected_loads`, graphics operand `00 20 01`, size `$4000`, instead of
  `00 30 03`/`$6000`), BG1 of `$100` (32×64 cells), `$101` (16×48), `$107`
  and `$12A` decode. `$100` lines up with the native frame (visual check,
  `local/mode7/jp/tower-100-bg1-decoded-vs-native.png`). Pixel identity is
  not checked: BG2, BG3 text, statues and colour math cover most of it.
- Not modelled: the `$100` BG2 layer, the bank-`$82` scene (intro and
  guardian dialogue, flags `$100`/`$115`), the arrival selectors `$66`/`$62`
  and the tower movement profile.

## EU (`Terranigma (E) [!].smc`, PAL)

Same exit records at the same addresses (`$81:8CC1`, `$81:F046`). HDMA
tables have identical bytes, at JP-`$43` in bank `$87`: A/D `$87:994F` (data
`$87:9968`), M7SEL `$9959`, mosaic `$9BDC`, CGADSUB `$9B28`, CGRAM `$9B59`,
TM/TS `$9BE1`, setup `$87:98C8`. Camera `$87:90C5`, NMI upload
`$85:FADC`. In the 256x240 capture, ares puts the frame 20 lines lower and
2 px further left: vcounter 1 is row 29, and rows past vcounter 211 are cut
off.

## Captures (`local/mode7/`, git-ignored)

`jp/{arrival-59762,walk-down-59768,walk-left-59778}` and
`eu/{arrival-73938,walk-down-73942,walk-down-73950}` each hold:
`native.png`, `native.rgb` (256×240 RGB), `prev.{state,wram,vram,cgram,oam}`
(the frame before, whose registers drew `native.png`), `ppu-hdma.json` (the
per-frame registers and every HDMA write per line) and `ours.png`. JP states
come from `local/pandora-tower-discovery/departure/journey/underworld-arrival.state`.
EU states come from a fresh replay of `eu-pandora-tour.inputs` +
`eu-world-map.inputs` (`tools/eu.jsonl`). Tools: `local/mode7/tools/`
(`cmp.py ROM state:frame`).

## Open

- Drawing the horizon band's OBJ into the fog subtraction, and the
  world-map Ark sprite source.
- The exact palette/char animation scripts (`$8D:9331..94C4`).
- The second HDMA variant at `$87:995E` (A/D table `$87:99A1`, COLDATA
  `$87:9E4F`/`$9E2E`), selected by actor param `$0006` (guess: another
  world map or a transition).
- Exit timing (check before or after movement) and gating inside towers 2-5.
