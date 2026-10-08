# The towers' second layer

Research for [tower-second-layer](../meta/issues/tower-second-layer.md).
JP addresses unless marked. "L1" and "L2" are the map script's logical
layers (`10 01` and `10 02`, metatiles `20 00 40 00 01` / `…02`). "BG1"
and "BG2" are the hardware layers. On most tower maps **L2 is hardware
BG1**, so it is the front layer, not a backdrop.

Evidence: the profile code (read), a scan of all maps `$100..$12B` with
`assets::maps::scripts::resolve_map` (scratch program
`~/.claude/jobs/ef2e9592/tmp/bg2`), and one native capture on `$101` (EU
state `tmp/lv/out/eutower.state`, captures and layer renders in
`tmp/t101`).

## 1. The display profile

The map header (`$82:8000 + 2*map`, else `$83:8000 + 2*map`) byte 1 is the
profile byte (`docs/map-colour-math.md`). `$86:8C61` reads the 9-byte record
at `$96:BB64 + 2*(p & $3F)`:

| Byte | Use |
|---|---|
| +0..+3 | TM (and TMW), TS (and TSW), CGWSEL, CGADSUB, written to the PPU and the shadows `$0468..$046B` |
| +4 | bits 0/1: `$0846/$0848`; bits 4-5: `$0C23`; bit 6: `$0866` (camera clamp height `$100`/`$E0`); bit 7 clear: `$080C = 1` |
| +5 | bits 0-1, 2-3: BG1SC/BG2SC sizes; bits 4-5: BG3SC; bits 6-7 to `$086A`. **Bit 7 swaps the layers**: BG1SC takes `$0839` (the L2 ring, VRAM `$3C00`), BG2SC takes `$0837` (the L1 ring, `$3800`) |
| +6 | BGMODE `$2105` |
| +7, +8 | L2 scroll parameters for x and y, to `$086C/$086E` |

The profile byte's bits 6-7 select window set 0 on all tower maps:
BG12NBA `$00` (L1 and L2 share the char sheet at VRAM `$0000`), WOBJSEL
`$00` (no colour window).

### Scroll (`$8D:837A`, called every frame from `$80:81C7`)

Every frame: L1 (`$081E/$0822`) moves to the camera (`$080E/$0812`) by at
most 8 px; then `$0810 = $081E * m / d` (same for y), with `+7` =
`m<<4 | d` (`$86:8DB9` nibble split, `$8D:84B9` multiply/divide); then L2
(`$0820/$0824`) moves to `$0810/$0814` by at most 8 px. Byte 0 (`m = d =
0`) leaves `$0810/$0814` at 0 (cleared at `$86:8DAB`): a fixed L2.
`+5` bit 6 (drift, the town) and bit 5 (a table) are other modes; no tower
uses them. The NMI (`$86:A526`) writes L1 to BG1 and L2 to BG2 scroll, or
the reverse when `$086A` bit 7 (the swap) is set.

So `$11` = 1:1 means L2 scrolls exactly with L1 in the same frame.
Native `$101` (EU): `$081E/$0822 = $0820/$0824` = (0,495) and (0,512)
after a walk, `$086C/$086E = $0101`.

## 2. Per map

All tower char sheets are `80 00 20 01` (`$4000` bytes, 512 tiles); tile 0
is blank on every map, so metatile 0 is transparent. L2 always shares the
chars and CGRAM with L1 (one map palette `40 00 60 20` plus the shared
`40 00 20 00`). L2 has its own metatile set except on the sky maps. L2 cell
words have no bits above bit 8. L2 has the same dimensions as L1 except
`$113` (L1 4x3 screens, L2 3x3) and the sky maps.

| Maps | Profile (record) | TM TS CGWSEL CGADSUB | +4 +5 mode +7 +8 | L2 is | L2 high words |
|---|---|---|---|---|---|
| T1 `$101-$105`, T2 `$108-$10C`, T4 `$116-$11A`, `$125` | `$07` (`BC26`) | `17 00 80 20` | `E4 80 09 11 11` | BG1, front, opaque | 92..996 words (20-67%) |
| T3 `$10F-$113` | `$04` (`BC0B`) | `17 12 82 21` | `A4 80 09 11 11` | BG1, added light | 0..630 (`$113`: 0) |
| `$121` (T5 junction) | `$05` (`BC14`) | `17 12 82 21` | `E4 80 09 11 11` | as T3 | 192/240 |
| Outside `$100 $107 $10E $115 $11C` | `$12` (`BC89`) | `17 00 80 02` | `A4 00 09 00 00` | BG2, fixed sky behind L1 | 0 |
| `$106` light room | `$16` (`BCAD`) | `16 01 82 36` | `C0 80 01 11 11` | BG1 on the subscreen, added | 0 |
| `$11B` (T4 arena), `$123` (Shadowkeeper) | `$2B` (`BD6A`), `$17` (`BCB6`) | `15 02 82 B1` | `40/00 00 09 11 11` | BG2 on the subscreen, subtracted | 528/544, 2664/2892 |
| `$114 $11D-$120 $127 $128` | `$0E` (`BC65`) | `15 00 80 20` | `64 00 09 00 00` | not named, not shown | - |
| `$122` | `$0E` | `15 00 80 20` | as above | named (same as `$121`), but BG2 is not in TM: hidden (guess: no `COP 76` turns it on) | - |
| `$12A $12B` | `$0F` (`BC6E`) | `15 00 80 20` | `24 00 09 00 00` | not named | - |
| `$10D $124 $126 $129` | no header / no script / no layers | | | | |

Profile `$04`/`$05` is the Crysta indoor profile `$06` (`$96:BC1D`, `17 12 82
21 64 80 09 11 11`) except byte +4, so tower 3 and `$121` show L2 exactly as
the Crysta rays. The sky maps have the same L2 (`$CA:7E71`, metatiles
`$C5:50B8` = their L1 set) and the same darkening element `FB 00 AA B4 97`
in their scene record as `$100` (`docs/tower-entry.md`).

L2 sources (JP `$CB:6725` style; EU is two banks higher):

| Map | L2 | metatiles | Map | L2 | metatiles |
|---|---|---|---|---|---|
| `$101` | `$CC:6879` | `$CA:58DB` | `$10F` | `$CC:1E73` | `$CB:6DE2` |
| `$102` | `$CB:479E` | `$CA:58DB` | `$110` | `$CC:1AE8` | `$CB:6DE2` |
| `$103` | `$CB:7694` | `$CA:58DB` | `$111` | `$CC:280A` | `$CB:6DE2` |
| `$104` | `$CB:3FE1` | `$CA:58DB` | `$112` | `$CC:290B` | `$CB:6DE2` |
| `$105` | `$CC:0000` | `$CA:58DB` | `$113` | `$CD:0D8C` | `$CB:6DE2` |
| `$108` | `$CB:3D3E` | `$C9:7B06` | `$116` | `$CB:427B` | `$CB:7C98` |
| `$109` | `$CB:18C0` | `$C9:7B06` | `$117` | `$CB:5423` | `$CB:7C98` |
| `$10A` | `$CB:4A29` | `$C9:7B06` | `$118` | `$CB:3A98` | `$CB:7C98` |
| `$10B` | `$CC:08EB` | `$C9:7B06` | `$119` | `$CB:58FE` | `$CB:7C98` |
| `$10C` | `$CC:1371` | `$C9:7B06` | `$11A` | `$CB:4F38` | `$CB:7C98` |
| `$125` | `$CD:0423` | `$C9:7B06` | `$121`/`$122` | `$CD:177E` | `$CB:6DE2` |
| `$106` | `$CB:6725` | `$CA:6739` | `$11B` | `$CC:620A` | `$C7:44D5` |
| `$123` | `$CB:64D4` | `$C7:44D5` | sky | `$CA:7E71` | `$C5:50B8` |

### Native check (`$101`, EU)

Shadows: TM `$17`, TS `$00`, CGWSEL `$80`, CGADSUB `$20`, BG12NBA `$00`,
BG1SC `$3C`, BG2SC `$38`. The decoded L2 (`$CC:6879` + `$CA:58DB`) equals
VRAM `$3C00` (BG1) on all 896 visible tile words at two camera positions;
the decoded L1 equals `$3800` (BG2), 896/896. Zero cross-matches. L2 holds
the torches, the side pillars, the ledges and the door frame
(`tmp/t101/r/b_bg1.png`).

## 3. Priority

Mode 1, front to back. BGMODE `$09` (BG3 high on top):
BG3.1, OBJ.3, BG1.1, BG2.1, OBJ.2, BG1.0, BG2.0, OBJ.1, OBJ.0, BG3.0.
BGMODE `$01` (`$106`): OBJ.3, BG1.1, BG2.1, OBJ.2, BG1.0, BG2.0, OBJ.1,
BG3.1, OBJ.0, BG3.0. Tile priority is bit 13 (`$2000`) of the metatile
word, never the cell. Ark and ordinary actors are OBJ priority 2
(`docs/depth-order.md`).

With the swap (profiles `$07`, `$04`, `$05`):
**L2 high > L1 high > OBJ2 > L2 low > L1 low > OBJ1**. So an L2 low pixel
covers L1 low but not L1 high, and is behind Ark; an L2 high pixel covers
everything except BG3 high and OBJ3.

Without the swap (sky `$12`): L1 high > L2 high > OBJ2 > L1 low > L2 low.
The sky is all low, so it is behind L1 and behind Ark.

## 4. Rendering rules for the app

1. Decode L2 with `SecondLayer::from_rom` (it already takes the right
   resources); the allowlist must admit the tower maps. Use L1's tiles and
   palette (and L1's animation).
2. Scroll: profiles with `+7/+8 = $11`: L2 at the camera, as L1. `$00`:
   fixed at (0,0) (the sky). Wrap at L2's own size (guess for `$113`:
   the ring masks `$085A/$085E,X` use each layer's own width).
3. Plain profile `$07` (towers 1, 2, 4 floors, `$125`): L2 is an opaque
   layer. Colour 0 is transparent. Draw back to front: L1 low, L2 low,
   OBJ2 sprites, L1 high, L2 high. In the app's model (one BG frame plus a
   `high` mask): composite L2 over L1 per pixel where L2 is opaque, except
   where L2 is low and L1 is high; set `high` from the winning pixel. Ark
   is then hidden by L2 high pixels and drawn over L2 low pixels.
   CGADSUB `$20` with fixed colour 0 changes nothing.
4. Additive profiles `$04`/`$05` (tower 3, `$121`): as the Crysta rays
   (`add_second_layer`). Exact: where L2 (BG1) is the front main pixel,
   add the front subscreen pixel (L1 or OBJ, or fixed colour 0), no half.
   An L2 low pixel under Ark is not drawn (Ark is in front, OBJ has no
   math); an L2 high pixel adds over Ark. The current "add over the
   sprites too" is right for high pixels only.
5. `$106`: L2 is subscreen BG1, added to L1, BG3, OBJ palettes 4-7 and
   the backdrop (the town's model, `add_second_layer`).
6. `$11B`, `$123`: L2 is subscreen BG2, **subtracted** (no half) from L1,
   OBJ palettes 4-7 and the backdrop; BG3 and OBJ palettes 0-3 are not
   changed. Its priority bits do not matter.
7. Sky maps: the existing `Sky` (`$100`) applies to `$107`, `$10E`,
   `$115`, `$11C` too, with the same HDMA darkening.
8. No L2 on screen: `$114 $11D-$120 $122 $127 $128 $12A $12B`.
9. Patches: `COP 46` with L ≥ 1 and flag patches with `second_layer`
   write L2 cells (`$7E:E000`); they change the picture only, never
   collision (`docs/block-patch.md`).

## Open

- Native frames for the additive (`$10F`) and subtractive (`$123`) maps:
  the rules come from the registers only.
- Map actors that write TM/CGADSUB/fixed colour with `COP 76` on tower
  maps (bosses, dark rooms) are not surveyed.
- `$125` is not in the underworld map list; its role is unknown.
