# The light room `$106` and the first resurrection

From tower 1's door to the underworld map `$03`. JP addresses unless
marked; EU differs only where shown. The evidence is source reads (map
loader `$86:86ED`, the scripts below), a Python render of `$106`'s
layers from both ROMs, and the EU longplay (PAL, captured at 60 fps,
27:44-30:01). No emulator run was made, so the frame counts are script
waits or video times, and not traces.

Screenshots are in `/Users/lainsoykaf/.claude/jobs/ef2e9592/tmp/light/`:
`01-room106-enter`, `02-room106-orb`, `03-room106-flash`, `04-ocean`,
`05-earth`, `06-spiral-continent`, `07-snow-mountain`,
`08-parchment-eurasia`, `09-souls-blue`, `10-souls-colour`,
`11-back-on-03` (all `.png`). The renders are `jp_106_layers.png` (BG1,
BG2, BG3, BG2+BG1) and `{jp,eu}_106_bg{1,2,3}.png`.

## 1. What `$106` loads

Script JP `$B3:866F`, EU `$B5:866F`. The command bytes are the same in
both ROMs. Only the packed pointers differ: the EU sources are 2 banks
higher, and the BG1 metatiles are also 2 bytes later.

| Bytes | JP / EU source | What |
|---|---|---|
| `80 00 20 01` | `$BB:D3BE` / `$BD:D3BE`, LZ `$4000` | BG1/BG2 4bpp tiles (the tower recipe) |
| `40 00 70 10` | `$CC:695B` / `$CE:695B`, raw | **`$70` colours to CGRAM `$10`** (palettes 1-7) |
| `20 00 40 00 01` | `$C8:666C` / `$CA:666E` | BG1 metatiles (palettes 2, 3) |
| `20 00 08 00 81` | `$BB:FFE5` / `$BD:FFE5` | attributes |
| `20 00 40 00 02` | `$CA:6739` / `$CC:6739` | BG2 metatiles (palettes 2, 7) |
| `10 01`, `10 02` | `$CB:6BA9`, `$CB:6725` (EU `$CD:`) | BG1, BG2 layers, 1x1 page (16x16 cells) |
| `04 … 01 03` | `$CB:218C` / `$CD:218C` | **opcode `$04`**: LZ screen to `$7E:5000`, then DMA to VRAM `$6800` (BG3 map). Handler `$86:8E9B`: mode 3 = `$6800`, 1/2 = the BG1/BG2 map base, bit 7 = VRAM `(b & $7C) << 8`. The size is `w*h*$800` from the header bytes 1 and 3 |
| `08 FC 2E 00` | | audio |
| `80 00 08 00` | `$CA:1FE5` / `$CC:1FE5` | BG3 2bpp tiles to VRAM `$7000` |

There is no `08 FF 10` jump, so the shared resources (`$98:819C`:
CGRAM `$00-$1F` and the HUD font) are **not** loaded. CGRAM `$00-$0F`
keeps the previous map's colours. The layers are as follows (they match
the video when composed):
- BG1 is the pedestal and the dome frame.
- BG2 is the light pillar (palette 7) and the dark side wedges.
- BG3 is the constellation net (BG3 palettes 4, 6, 7, which are CGRAM `$10-$1F`, from `$106`'s own palette).

The JP and EU renders are identical, except for the backdrop.

### Why our loader refuses it

`StaticBackground::from_rom` sends `$106` (in `0x0100..=0x0127`,
`crates/assets/src/maps/visual.rs:250`) through `TOWER_LOADS`.
`projected_recipe` asks for `WANTED_LOADS[1]` (palette `00 60 20`) and
fails at **`visual.rs:744-746`** (`ok_or(... "map does not load a
required background resource")`). The shared palette `00 20 00` is
missing as well. If those two were fixed, the opcode-`$04` load and
`10 02` would still meet `is_known_unconsumed` (`visual.rs:701`).
`10 02` is already listed there, but `04` is not, so the next error would
be "unqualified background resource" (`visual.rs:765`).

### Minimal change

A dedicated recipe for `$106` in `StaticBackground::from_rom`:
- graphics `00 20 01` (as the towers);
- palette `00 70 10`, `$E0` bytes, put into `palette[0x10..0x80]`;
- metatiles `…01` and `…81`, layer `01`.

For this recipe only, accept the opcode-`$04` load with mode byte 3 (the
BG3 screen) as known-unconsumed, and the palette `00 70 10` as consumed. For `palette[0x00..0x10]`, keep the tower shared palette
(`$98:81A5`, which every tower room loads before `$106`). The backdrop is
not settled: that palette gives colour 0 = `$28CD` (JP) / `$1184` (EU),
but the video shows black. Guess: the palette actor `FB $87:98BF` param
`$1E`, or the fade, writes it. Use black until a trace confirms it.
BG2 (the pillar, with colour math) and BG3 (the constellations) can come
later. BG1 alone already gives a correct room.

## 2. The sequence

| Step | Script / code | COP used (**missing** in `actors.rs`) | Video (EU) |
|---|---|---|---|
| Door `$90:93DB` (tower top `$105`) | `COP 14 06 01 00 66 78 00 A8 00`, i.e. `$106` mode 0 sel `$66` raw (120,168) | done | 27:44-27:49 |
| `$106` spawns (`$82:8AC6`; EU `$82:8AC4`) | `FD (8,9)` player; `FE $90:8AB9` (EU `8C52`); `01 (7,2) $90:8AD9` (EU `8C72`), descriptor `$82:F645` (EU `F5D2`); `FB $87:98BF` param `$1E` (EU `$87:987C`) | | |
| Controller `$90:8AB9` | native: `INC $0886` every 4 frames, `INC $0888` every 8 frames (frame counter `$0042`). These scroll the constellation layer | native writes to `$0886/$0888` and the read of `$0042` are **not allowed** in `native.rs` | |
| Orb/light `$90:8AD9` | `B1 08`, `DF $90:8B74` (player), `2A`, `86 08 00`, `8F`, `B0 02`, `87 02 00 68` (slow descent), `8F`, `85 07 00`, `8F`, **`AA $90:8BA2`** (child), `85 04 00`, `8F` | **`AA`** (`$80:A8EE`, spawn child) | 27:50-28:16, orb comes down the pillar, flash 28:13 |
| Child `$90:8BA2` (EU `8D3B`) | `$7F:0106,X=$8000`, `$7F:0104,X=$9A` (EU `$9C`), **`8A 1F`**, **`93`**, **`AE`** | **`8A`** (`$80:A2E3`), **`93`** (`$80:A3B5`), **`AE`** (`$80:A95B`) | the flash/orb effect |
| Player `$90:8B74` (EU `8D0D`) | `84 01 00 00`, `84 01 09 01`, `84 01 00 00`, `C1 8`, `84 02 00 00`, `C1 8`, `84 00 00 00` | **`84`** in general (only the map-`$21` frozen-return case is modelled) | Ark walks up onto the pedestal |
| End of `$90:8AD9` | native `$047C = 7` (pending map **`$0007`**). `$0482` (previous map) `$105/$10C/$113/$11A/$123` gives `$04CC = 0..4` (tower index). Then `80 00`, `8E` | native writes to `$047C`/`$04CC` and the `$0482` read are **missing** | 28:17 black |
| Map `$07` (load script `$98:827C`, audio only). Spawns `$83:8976`: 4 OBJ records `$86:E55E/E5AA/E5FA/E64A`, `FE $86:BB4A`. Bank `$86` code is the same in EU | controller `$86:BB4A`, all native PPU code, switch on `$04CC` | **`A1`** (`$80:A6F1`, spawn), `00`/`01`, `02`/`03`, `C1`, `BC`, `BD`, `30`, `1C`, `1F` | |
| (a) ocean dive | BGMODE 7. T1 data `$E3:6F94` gives Mode 7 map, `$D9:64F8` tiles, `$EB:0416` colours. `A1 $86:E249/E221`, `C1 $5A` | | 28:18-28:21 |
| (b) Earth | Mode 7, `$DE:0000`, colours `$EB:0616`. `A1 E2A0/E278`, `C1 $64`, music `30 0C` | | 28:21-28:23 |
| (c) spiral + continent | `$86:BE00..BF83`, continent picked by `$04CC`. Fade `00 $86:E46C`, `C1 $258` (600) | | 28:24-28:43 |
| (d) flyover | jump table `$86:BF98`: T1 `$86:C3DD`. **Mode 3** (not 7): a still 256-colour mountain picture (`$E9:622E` etc.) with snow on BG2/OBJ. Music `30 23`, `C1 $B4`, 32 palette steps x 8, `C1 $1E0`. It returns at `JMP $BFCD` | | 28:50-29:06 |
| (e) parchment map | `$86:BFCD..C2F8`: music `30 02`, mode 1, mosaic reveal, `INC $04CC`. Text `1C 92 8A C7` (EU `$92:DE10`): "On this day, Eurasia was resurrected." (T2-T5 JP `$92:C7B8/C7E3/C810/C83B`, EU `DE40/DE6D/DE9C/DEC9`). `1F` waits for the button. Fade `00 $86:E488`, `00 $86:C376`, then native **`$047C = $3C + tower`** | | 29:10-29:31 |
| Souls map `$3C` (T2-T5: `$3D-$40`) | spawns `$83:9400`: `FE $87:F6FA` (EU `$87:F64E`) param 0, 7 figures `$87:F7F9`/`F80C` (EU `F74D`/`F760`) | **`A3`** (figures), **`97`** (palette fade) | 29:32-29:46 |
| Souls controller | 3 palettes `$CC:2A6C` (blue), `C1 300`, `INC $04BC` (the figures take colour), `C1 60`, loop `$1F` x `97 …`. Then `0A $3C..$40` to `$87:F7A4` (EU `F6F8`) | `0A` done | |
| Exit `$87:F7A4` | `07 01 81` sets **flag `$101`**, then `14 03 00 01 10 D0 00 30 03`: **`$03` mode 1 sel `$10`, raw (208,816)** | done | 29:47 world map |

T2-T5 exits are the same with flags `$103/$105/$107/$109` and raw
(80,608), (480,160), (736,208), (880,688).

Video durations (EU, 50 Hz game): `$106` 26.9 s, ocean 3.1 s, Earth
1.9 s, spiral 19.5 s, stars plus mountain 23 s, parchment 21.9 s (it
waits for a button), souls 14.6 s. That is about 1:56 from the door to
`$03`.

## 3. Minimal playable implementation

Must be faithful:
- the door transfer;
- the `$106` art (BG1 at least);
- the tower index from the previous map;
- flag `$101` (and `$103..$109` per tower);
- the `$03` destination (mode 1 sel `$10`, raw (208,816) for T1);
- the parchment text from ROM (`$92:C78A` JP / `$92:DE10` EU, through `COP 1C`), with its button wait.

A first static approximation is acceptable:
- `$106`: BG1 only. Run Ark's walk-up and a fixed delay (about 1300 frames) instead of `COP AA/8A/93/AE` and the BG3 scroll.
- Map `$07`: one still parchment screen. Fade it, type the text, wait for the button. Leave out the ocean, Earth, spiral and flyover; they are bespoke native PPU code (Mode 7 and Mode 3) in `$86:BB4A..C585`.
- Map `$3C`: skip it, or show a fixed delay. Then do the `$87:F7A4` step: set `$101` and transfer.

This is consistent with `meta/issues/tower-one-top.md` ("first, static
resurrection screen; sets `$101` as natively").

The runtime work this needs:
- admit `$106` (the recipe above);
- an engine route for the pending map from native code (`$047C`), or a scene hook keyed on map `$07`;
- `$0482` (the previous map);
- general `COP 84` for Ark's walk.

## Open

- The source of CGRAM colour 0 in `$106`.
- The roles of the 4 OBJ actors on `$07`.
- The `COP 97`/`A3` operands.
- Native frame counts (no emulator trace was made).
