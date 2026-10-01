# Mode-`$0004` resource descriptors (tower 1 statues and guardian)

JP addresses unless marked. The bank-`$80` loader code has the same
addresses and bytes in EU. Only data pointers move (table below).

## Where the descriptor is read

Spawn record reader `$80:F4EA` (`$01` records): record bytes 4-6 are the
script header, bytes 7-9 the descriptor pointer (`$66/$68`). The script
header byte 0 goes to `$7F:0008,X`. This is the initial pose list. If the
descriptor bank is 0 or below `$90`, `$80:F5D0` calls `$80:FA4E`:

```
$80:FA4E  Y=0; JSR FA65 (packet, mode); if carry (reuse) done
          JSR FB5A (second packet) ; JSR FBE4 (palette)
          JSR FCD2 (graphics)      ; JSR FE8F (relocate packet)
```

A descriptor pointer of `00 00 00` (`$66` = 0) takes `$80:FAF9`. The actor
reuses the previous actor's packet (`$BF`), second table (`$48`), idle class
(`$46`) and `$4A`. Palette, graphics and relocation are skipped.

## Field layout (`$82:F65A`, EU `$82:F5E7`, 21 bytes)

`00 00 f7 | 04 00 | 85 11 fb | 40 0c 5a cc 04 04 06 | 00 00 40 1d 2d c0`

The next 3 bytes (`d4 36 fb`) start the next descriptor (`$82:F66F`). They
are not part of this one.

| Bytes | Value | Reader | Meaning |
|---|---|---|---|
| 0-2 | `$F7:0000` | `FA87` | Pose packet. `$86:83BE` decompresses it to `$7E:7000+$BD`. `$FFFF` = keep the previous one. |
| 3 | `$04` | `FAA4` | Low nibble: idle class -> `$7F:2018`. Bits 4-6: `$4000+(b&$70)<<8` -> `$7F:0022/0026`. A value of `$6000` (bits = `$20`) means "no second packet". Bit 7: `JSR FB20` (frames stream through WRAM, `$0DF0/$0DF2`). |
| 4 | `$00` | `FACF` | Non-zero: index into `$8D:BDFA` -> `$7F:1022` (then `JSR F94C`). |
| 5-7 | `$FB:1185` | `FB7D` | Only when bits 4-6 of d[3] are not `$20`. Second packet, decompressed to `$7F:4000+$C1` (480 bytes). Its word list is relocated by the base. `$7F:0022/0026` point to it. This is house.rs's "movement program". It is not needed for raster. |
| 8 | `$40` | `FBE4` | Palette flags. Bit 7: the pointer is from the table `$80:FC72+(b&$3F)*3` and the field is 4 bytes (house modes `$80/$81/$90`). Bit 6: the source OBJ palette is d[12]/2 (else it is taken from the first frame's first component). Bits 0-5: subtracted from d[12] when the source is addressed. |
| 9-11 | `$CC:5A0C` | `FBFC` | Palette pointer. EU `$CE:5A0C`. |
| 12 | `$04` | `FC02` | Source slot in 16-byte units (`&$0E`). Source = ptr + (d12&$0E - d8&$3F)*16 = `$CC:5A4C`. |
| 13 | `$04` | `FC22` | Length in 16-byte (8-colour) units: 64 bytes = 2 OBJ palettes. |
| 14 | `$06` | `FC28` | Destination in 8-colour units: CGRAM `128 + 8*d14` = colour 176 (OBJ palettes 3 and 4). Copied by MVN into the buffer `$7F:0700+d14*16`. |
| 15 | `$00` | `FCDC` | Source offset in 64-byte units inside the decompressed sheet (`$7E:5000+(b&$7F)*64`). Bit 7: decompress to `$BB:0000` instead. |
| 16 | `$00` | `FCFB` | VRAM destination: word `$4000 + $4C + d16*32`. `$4C` = `$1000` if the actor's `$0008` bit `$100` (record byte 0 bit 0) is set. |
| 17 | `$40` | `FD10` | Bits 0-6: transfer size, `(b&$7F)*64` bytes per row band. Bit 7: the graphics pointer is 1 byte, an offset into `$80:FDA4` (house `$C0`/`$C3`/`$F0` forms). |
| 18-20 | `$C0:2D1D` | `FD1F` | Graphics packet. EU `$C2:2D1D`. Cached by `$86:9145` against `$0433`. It decompresses to `$7E:5000` (8192 bytes). |

### Graphics upload (`$80:FD4A`-`FD8A`, `FE0E`)

The source is a 16-tile-wide sheet. Each step makes two DMAs on channel 0
(`$2118`, mode 1): `$06` bytes from `$00` to VRAM `$02`, and `$06` bytes from
`$00+$200` to VRAM `$02+$100`. These are the top and bottom 8-px rows of one
16-px row. The step length is clipped to the next `$400` source boundary.
Then `$00` moves to the next `$400` block and `$02` to the next `$200` words.
The total is `2*(d17&$7F)*64` bytes.

For the tower: `$2000` bytes. All 256 tiles go 1:1 to VRAM words
`$5000-$5FFF`, because the records have byte 0 = `$01` (second OBJ name
table, OBSEL = `$02`). This was verified in the emulator, JP and EU: VRAM
`$5000` equals the decompressed `$C0:2D1D`.

### Packet relocation (`$80:FE8F`)

The relocation walks every frame from the first frame anchor (the last word
of the list table, here `$E6`) up to `$FFFF`. It rewrites each component word
as `word - C9 + CB`, where:
- `C9 = (d15&$7F)*2 | (d12&$0E)<<8`
- `CB = d16*2 | (d14&$0E)<<8`

Thus tile' = tile - 2*d15 + 2*d16, and OBJ palette' = palette - d12/2 + d14/2.
For the tower: the tile is unchanged and the palette moves by +1 (2->3, 3->4).
The OAM builder then adds `$100` for the second name table.

## Scripts and COPs

Records `$82:88CB` (both ROMs), map `$100`:

| Actor | Record | Header (initial list) | Script | Effect | Final origin |
|---|---|---|---|---|---|
| Guardian | `01 0f 37 00`, desc `$82:F65A` | `$90:8BF0` = `01 00 51 00 20` -> list **1** | `$90:8BF5` (EU `$90:8D8E`): `COP B3 8,8`; `COP 08 $8196,..`; `COP BC`; `COP D6 $40,$8C97` (talk when Ark is within 65 px) | +8,+8 | (256,888) |
| Left statue | `01 0a 3e 00`, desc 0 (reuse) | `$90:8F09` = `00 00 51 00 00` -> list **0** | `$90:8F0E` (EU `$90:90EA`): `COP B1 -16`; `COP BC` | x-16 | (152,992) |
| Right statue | `01 17 3e 00`, desc 0 (reuse) | `$90:8F15` -> list **0** | `$90:8F1A` (EU `$90:90F6`): `COP B7`; `COP B1 +16`; `COP BC` | H-flip, then x-16 | (360,992), mirrored |

COP handlers (table `$80:83B2`, both ROMs):
- `B1` = `$80:A9B9`: x += operand. The operand is negated if the actor is H-flipped (`$0008` bit 14).
- `B3` = `$80:A9EA`: the same for x, then y += second operand.
- `B7` = `$80:AA42`: set H-flip (`$0008 |= $4000`).
- `BC` = `$80:AAA5`: save the script pointer and yield (park).

Record x is cell*16+8 and record y is cell*16. The emulator actor `$1080`
is at (152,992).

## Poses (packet `$F7:0000`, EU `$F9:0000`, 4378 bytes)

The list table has 9 lists. The 10th word (`$E6`) is the start of the frames.
Lists 0 and 1 each have one record `00 03 anchor` (duration 0, static).

- List 0 (statue): frame `$FE`. Anchor (32/32, 112/64). 43 large (16x16)
  components, tiles `$07-$EE`, palette 2 (-> 3). The figure is 64x176: a
  knight with a sword on a pedestal.
- List 1 (guardian): frame `$23C`. Anchor (40/40, 96/8). 28 large
  components and one small one. Tile `$26` (chains, palettes 2/3) and
  `$16`, `$2C-$8E` (plaque, palette 3). The palettes become 3/4.

The component offsets are **unsigned** bytes. The y value `$80`-`$A0` is
below the anchor, not above. The statue rows 144 and 160 (y 1024-1055) are
below the map bottom (the camera clamps at 800), so natively they are never
seen. The emulator OAM (JP frame of `tower/jp/end`, the same in EU) matches
exactly: 34 visible components each for the statues (OAM 9-42 and 43-77,
the right one with H-flip and the alternate x column), and 29 for the
guardian (OAM 80-108). All are tiles `$1xx`, palettes 3/4, priority 2, large.

CGRAM 176-207 equals ROM `$CC:5A4C` (64 bytes), except colour 14 of each
palette (ROM `$7C1F`, game `$08DF`/`$7FFF`). No pixel in the sheet uses
colour 14.

## EU data addresses

| | JP | EU |
|---|---|---|
| Descriptor | `$82:F65A` | `$82:F5E7` |
| Pose packet | `$F7:0000` | `$F9:0000` |
| Second packet | `$FB:1185` | `$FD:1185` |
| Palette | `$CC:5A0C` (+`$40`) | `$CE:5A0C` (+`$40`) |
| Graphics | `$C0:2D1D` | `$C2:2D1D` |
| Guardian header/script | `$90:8BF0`/`8BF5` | `$90:8D89`/`8D8E` |
| Statue headers | `$90:8F09`, `$90:8F15` | `$90:90E5`, `$90:90F1` |
