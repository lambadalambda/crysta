# Qualified fresh house backgrounds and collision profiles

## Admission and boundaries

`StaticBackground::from_rom` now admits **B, C, D, F, 10, 11** and the existing
cavern 128. E/20/21 remain rejected. Success means a **ROM-only first-background
recipe**, not access to every sector, an event VM, or whole-scene reproduction.
F/10/cavern payloads and existing APIs are unchanged. No sprite, core, host, UI,
tracker or asset-library-root changes are part of this work.

All six produce the **same complete 512×1024 sheet**, not six cropped assets:

| Plane | SHA-256 |
|---|---|
| Full indexed sheet, zero = transparent | `4adec38bf192483ec43e62feaeb9219cc1e0d02b4860bc3b682dd5cc11c8886d` |
| Opaque-pixel priority plane, transparent = 0 | `689d88232a9fc9a10d4550ffc8ff31a55ef0826b28046e8b14933cc902968174` |
| 128 natural BGR555 colors, little-endian | `f390bfcaf76ff322de892ca50514b387bce1c2c29b949376ea6e6d409dec2010` |
| Source-attributed 2048-cell grid, little-endian | `18b85caa2d23a51a941d296cc7b6c1eb8caa1c0c4efe6b88a2cff291900ab9db` |

There is **no admitted base palette or sheet variant**. The original natural
palette is retained, including ROM color 0 `$28CD`. Native backdrop color 0 is
`$1D6B`; all colors **1–127** match the ROM palette at every selected checkpoint.
Transparency is not an opaque backdrop or an occluder. The local export's RGB
hash uses its explicitly selected 48/80 checkerboard, not the older viewer's
checkerboard; index/priority/palette hashes are the stable interchange planes.

### Loading roots and resources

| Map | Root | Root behavior | Camera |
|---|---|---|---|
| B | `$988401` | FE `$0001`, fall through | `(0,0)` |
| C | `$988446` | FA `$0001`, END | `(0,256)` |
| D | `$98844D` | FA `$0001`, END | `(0,512)` |
| F | `$988496` | FA `$0001`, extra OBJ loads, END | `(256,0)` |
| 10 | `$9884AD` | FA `$0001`, END | `(256,256)` |
| 11 | `$9884B4` | FA `$0001`, END | `(256,512)` |

Each reaches common `$988405`: palette `$31E945..31EA05`, first layer
`$2FCBB3..2FCFE1`, graphics `$23B11D..23E222`, definitions `$2ABA43..2AC506`,
attributes `$30FF53..30FFFC`; shared palette `$328B78..328BB8` comes through
`$9881A5`. Ranges here are **headerless, half-open ROM offsets**. The extra F OBJ
loads do not overlap the first-background resources. Every root/control/table
byte is validated; only qualified packed resource-pointer fields vary. Existing
conditional audio alternatives remain checked without evaluating their events.

Secondary layer `$31E263` and its definitions are still omitted. Sunlight,
windows, color math, fades, dialogue, actors and shadows are not folded into the
base sheet. D's temporary scene-label glyphs are **not map art**.

## Source cameras and actual hardware assignment

The scene-list loader `$86955C..86959B`, called at `$8686F3`, chooses bank `$83`
after zero bank-$82` entries. The two-byte prefixes at `$838B7C`, `$838BF0`,
`$838CA1`, `$838D1E`, `$838D69`, `$838DCF` are all `00 06`. Prefix byte 1 supplies
display selector **6**, independently of the loading-script resource recipe.
`$868C77..8C86` reads `$96BB70 = $BC1D`; the nine-byte display profile at
`$96BC1D..BC26` is `17 12 82 21 64 80 09 11 11`.

- Profile byte +4, `$64`, selects `$0866 = 256` at `$868CDE..8CE6`.
- Byte +5, `$80`, selects the swapped branch `$868D3F`: first resource layer
  becomes **hardware BG2**, ring buffer VRAM word **`$3800`**.
- Byte +6 is the actual **BGMODE `$09`** write at `$868D6F`: mode 1,
  BG3-priority flag, 8×8 tiles. Main-screen mask `$17` enables BG1/BG2/OBJ.

New independent `Session::new` writer probes stop before `$868D6B`, before
`$868D6F`, and immediately after at `$868D72`. All six have X=`$B9`, DB=`$81`,
8-bit accumulator mode; the latter two stops have low A=`$09`. The write happens
at completed-frame counters F **2346**, 10 **6984**, C **7281**, B **7728**,
D **8089**, 11 **8303** (authoritative counters/pins are in `reference.json`).
These are loading-writer witnesses, not readback of arbitrary later raster state.
Settled shadows are `$046D=$3C`, `$046E=$38`, `$0468=$17` throughout.

Camera producer `$869371..8693F7`, called at `$86874F`, reads two bytes at
`$96BE30 + 2*map_id`. Records B/C/D/F/10/11 are respectively `10 10`, `10 11`,
`10 12`, `11 10`, `11 11`, `11 12`. For record `(a,b)`:

```text
left   = (a & 15) * 256; top    = (b & 15) * 256
right  = left + (a >> 4) * 256
bottom = top  + (b >> 4) * 256
```

The producer initializes `$080E/$0812`. Ordinary clamp `$8790A1..9105` uses
`clamp(player_x-128,left,right-256)` and
`clamp(player_y-112,top,bottom-$0866)`. These one-page bounds collapse to the
fixed cameras above; native bounds and `$081E/$0822` match them.

With this **verified** BG2/mode-09 assignment, the vendored ares mode table gives
BG2 low rank 4, ordinary OBJ2 rank 6, BG2 high rank 7. Thus **opaque high BG >
OBJ2 > low BG** is valid for the admitted first-background/ordinary-actor subset.
Tile-word `$2000` is priority; map-cell `$8000` is collision state. Color zero
contributes neither color nor high-priority occlusion. This does not qualify
secondary-layer, special shadow, OBJ3 label or window composition.

## Immutable collision contract for host compilation

Machine-readable **source-derived** contracts and grid hashes are in
[`profiles.json`](../tools/house-background-qualification/profiles.json). They are
not copied native grids, and are not a new production runtime API.

1. Decode each admitted map with `StaticBackground::from_rom`.
2. Use `layer().attributed_cells(resources()[3].decoded().try_into()?)`:
   `tile = source_word & $01FF; word = tile | ((attributes[tile]&$7F)<<9)`.
   This replaces source high bits. **The house table** has no attribute bit 6,
   so its base has no bit15; the generic formula need not always clear bit15.
3. OR `$8000` at the following source-configured/frozen-occupancy cells.
4. Separately select navigation's qualified wooden-door patch when applicable.
5. Construct an immutable **passive** collision room; content identity must include
   door profile, F history, source-resident freeze and passive policy.

Indices below are decimal, row-major **`row*32+column`** over the full sheet.

| Room/profile | Fixed source stamps | Frozen source-resident stamps | History residue |
|---|---|---|---|
| B, baseclosed | 203, 267 | 199 | — |
| C, baseclosed | — | 708, 710, 739, 805 | — |
| D, `$0026` clear | **1415** | **1316** | — |
| F, first load after intro | 317 | — | **504** |
| F, ordinary return/reload | 317 | — | — |
| 10 | 731, 732 | 826, 827 | — |
| 11, pre-interaction | 1334 | 1275 | — |

All assume global events **`$0020,$00FB` set**, other fresh globals including
`$0026` clear. Those two globals **alone do not identify F history**. Local blue-door
approach event `$0001` is not proof of a cellar opening. These are ordinary
initial/pre-interaction profiles, not every phase of each room's scripts.

### Why these are source stamps rather than RAM fixtures

COP3B `$809301 → $80BE8E` sets bit15; COP3C `$809314 → $80BF0E` clears it.
`$8D8C7E` maps pixels into the first-layer grid; `$8D8CE1` advances a column.
Descriptor geometry is loaded at `$86BAEF..86BB48` through `$80ED75`, and appears
at `$7F0028/2A/2C/2E + entity_slot` as `(x_offset,width,y_offset,height)`:

```text
nx = width >> 4; ny = height >> 4
if nx + ny < 3: stamp (entity.x - 8, entity.y - 16)
else: stamp nx * ny points from (entity.x + x_offset, entity.y + y_offset), step 16
```

It is **not generic bounding-box intersection** or reference-counted occupancy.
The checker confirms linked source-origin actors' actual geometry: `(-8,16,-16,16)`
except room 10's hidden selector-1 actor, `(-8,32,-16,16)`. Unlinked nonzero slots
are ignored. Source identities, not runtime slots, identify profiles.

| Stamp owner | Source record | COP3B site |
|---|---|---|
| B hidden pair | `$838BA0`, `$838BA7` | `$88968D` |
| B resident | `$838B96` | `$888E5A` |
| C residents | `$838C0A`, `$838C14`, `$838C1E`, `$838C28` | `$88A33C`, `$88A502`, `$889A85`, `$88A1BF` |
| D hidden exterior gate | `$838CC8` at `(120,720)` | `$88A9B8` |
| D resident setup | `$838CB4` at `(72,672)` | `$88A842` |
| F table parent | `$838D4F` at `(472,160)` | `$88D612` |
| 10 hidden interaction | `$838DA6` at `(440,368)` | `$889A4B` |
| 10 residents | `$838D7C`, `$838D86` | `$8898B6`, `$88999E` |
| 11 hidden interaction / resident | `$838DEC`, `$838DE2` | `$88C893`, `$88A9C8` |

F's visual child created at `$88D618` is **not an extra occupancy stamp**. The
opening resident checks `$0020` clear at `$889757`, sets it at `$889791`, stamps
its final `(392,256)` footprint at `$8897C1`, then unlinks via COPA7 `$8897C5 →
$80A876 → $80BD57` without clearing that footprint. Cell 504 is therefore a
**departed-actor residue** on the first load, absent after reload. Returned
controller `$888ABA` instead requires `$0020` set and contributes no extra
resident stamp on this branch. Fresh and returned native full grids verify both.

### D must remain closed; freeze the correct resident origin

`$88A9B4 COP48 $8026` retains the hidden actor while global `$0026` is **clear**;
`$88A9B8 COP3B` stamps **1415** (raw `$0592 → $8592`). This fixed gate is independent
of the visible wanderer. Its bit must never be excluded by a resident mask.

The explicitly frozen D resident uses **source setup `(72,672)`, cell 1316**
(raw `$0002 → $8002`). `settledD` frame 8175 matches this stamp. Later `D-again`
frame 8693 has `(72,688)` and cell **1348**, not 1316. That is a real dynamic
collision delta; the source script uses COP26 at `$88A868` and restamps at
`$88A870`. Do **not** place the source-origin sprite over the later captured
occupancy. For this pass use immutable **1316+1415**, explicitly without NPC AI.
Later native trajectories through the wanderer's footprint are not promised;
ordinary room routes and the hidden exterior gate remain separate contracts.

Both categories occupy the **same native bit15**. If a diagnostic masks dynamic
resident occupancy, mask only bit15 at specifically justified cells (e.g. 1316
and 1348 for that later D comparison), preserve every low 15-bit word, and keep
1415 outside the mask. This checker instead reports all full-word deltas without
masking any cells.

### Navigation-owned C/B door boundary

C's first `settledC` grid is **baseclosed** and matches all **2048 full words**
after its four source-resident stamps. After opening the wooden door, the shared
sheet retains differences at **616** (`$1CF2 → $1CF6`) and **648** (`$1CF3 → $00F7`)
in B, returned C, D and 11. These are **low-word tile/material mutations**, not
occupancy or an excuse to discard high bits. Returned C differs in eight sampled
8×8 tile words / 512 index-priority pixels.

**Handoff:** navigation owns the source interaction/mutation and must provide the
opened patch/profile to the host. This work reports these two native deltas but
does not research, implement or bless a patch copied from them. Until that
source-qualified patch is composed, the baseclosed B/D/11 profiles are not exact
whole-grid reproductions of the post-door itinerary (their own visible sectors
still match). The table above and `profiles.json` intentionally remain baseclosed.

Passive movement requires **`$0980 & $0050 == 0`**, not `$0980 == 0`. New-edge
flagged cells dispatch as solid; old-edge unsupported slope rejection still
precedes this override. See [house-materials.md](house-materials.md). No event
VM, action hooks, wooden-door actions, interactions or wandering AI are invented.

## Native tile/palette/priority and animation qualification

All selected captures from two independent census processes match each other;
the additional main-checkout replay also gives the same report. Every checkpoint
compares all 512 definitions, all 128 palette entries with the named backdrop
exception, all 2048 collision words (retaining deltas), and **672 full hardware
BG2 tile words** in interior offsets X `2..30`, Y `2..26` of its sector.

| Checkpoint | Frame | High / low words | Equal base index+priority pixels / 43008 |
|---|---:|---:|---:|
| F boot | 6800 | 122 / 550 | 43000 |
| 10 | 7050 | 126 / 546 | 42998 |
| C baseclosed | 7372 | 136 / 536 | 43008 |
| B settled | 7848 | 156 / 516 | 43008 |
| D settled | 8175 | 224 / 448 | 43008 |
| 11 settled | 8389 | 132 / 540 | 42940 |
| 11 passive wait | 8569 | 132 / 540 | 42936 |
| F returned | 7250, separate return itinerary | 122 / 550 | 42998 |

The unequal base pixels are **source-qualified animation**, not a base variant:

- F/10 compact records `$838D31/$838D9A` select graphics animation `$0F`.
  Table `$9BE5ED..E62E` writes tiles **37–40**, VRAM word `$0250`, 128 bytes.
- 11 records `$838DFD/$838DF3` select `$10/$11`. Tables `$9BEBAE..EBCF` and
  `$9BED4F..ED58` write tiles **41–44** / **57–60**, VRAM words `$0290/$0390`.
- `$8798EB → $8D93D8/$8D940D/$8D9470` resolves the table, reads its eight-byte
  frame records, and queues DMA. COP95 `$80A404` continues; `$86A678..A6C4`
  performs the hardware transfer. Repetitions advance the source by transfer
  length, retaining the destination. The checker bounds these three selectors;
  it is **not an animation timer or compact-script interpreter**.
- Selected full 128-byte native spans match source-table payloads exactly.
  Substituting only those source frames reconstructs **all 768 native graphics
  tiles**, with no arbitrary changed-tile exclusion. The frames remain in VRAM
  through later scene changes; no claim is made that later scenes created those
  animation services. Base presentation may deliberately omit their timing.
- 11's separate palette-animation selector `$07` does not change the admitted
  first-background colors 1–127 at these checkpoints. Secondary palette effects
  remain outside this profile.

F's native attribute scratch entry 0 is 22 instead of ROM 0 on both selected F
checkpoints; other entries agree, and the source-compiled whole grid still matches.
The exporter keeps the source attribute table. Native memory is evidence, never
production input. This is not full-framebuffer equality: natural colors are not
renderer gamma, and excluded scene layers can visibly change the final frame.

## Reproduction, pins and review

Japanese normalized ROM SHA-256:
`f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548`.
`reference.json` pins source consumers, scene records, camera/profile tables,
animated payload/table extents, vendored priority consumers, input scripts,
full exports, writer stops and native surfaces. No raw artifact is tracked.

```sh
cargo test -p assets --lib maps::visual::tests
cargo test -p assets --test visual_maps --test local_visual_maps --test local_house_backgrounds
python3 -B tools/house-background-qualification/test_checks.py
python3 -O -B tools/house-background-qualification/test_checks.py
sh tools/house-background-qualification/replay.sh "$ROM" "$CENSUS_REPLAY" "$RETURNED_F_REPLAY"
```

The driver uses the shared empty-SRAM bootstrap and the exact finite
`tools/house-scene-qualification/route.jsonl`. Each writer trace starts within one
held-input segment and must stop before its next input edge. There is no forced
warp, memory patch, restored state, SRAM seed or `save_state`. Reused returned-F
surfaces come from `tools/player-sprite-qualification/probe.rs`'s independently
qualified empty-SRAM return itinerary, not the SRAM-based older graphics test.

Local evidence:
- New exports/writer traces: `local/house-background-qualification/run-IjSuen`;
  full driver rerun `local/house-background-qualification/run-sC5Jhh` matched all
  committed pins in normal and optimized Python.
- Initial census: `/Users/lainsoykaf/repos/ilar-task-house-npc/local/house-scene-qualification/replay-gVc1Gx` (read-only).
- Repeated main census: `/Users/lainsoykaf/repos/terranigma/local/house-scene-qualification/replay-r6NVSO` (read-only).
- Returned F: `/Users/lainsoykaf/repos/terranigma/local/player-sprite-qualification/replay-IzVj7x` (read-only).

Synthetic tests followed red→green (new admissions and checker helpers).
Independent source review approved the minimal assets change before commit;
a separate tooling/documentation review also approved the qualification contract.
Both reviews were read-only source reviews; executed tests/replays are listed
above, not attributed to the reviewers. The background subissue's tracker remains
parent-owned; opened-door source composition is the explicit integration handoff,
not an implicitly completed event implementation.
