# First house exterior: map A

Bounded source/assets handoff for
[qualify-house-exterior-profile](../meta/issues/qualify-house-exterior-profile.md).
Conversation, flag `$0026`, hidden gate removal, D departure and arrival timing
belong to the conversation/navigation owner. This profile neither executes their
events nor admits the rest of Crysta. All asset data comes from the authenticated
Japanese ROM; native surfaces are comparison evidence only.

## Destination and source asset API

D's first exit record `$818DFE` directly targets **map `$000A`**. Its raw anchor
is `(496,752)`; the parent's source initialization yields **`(504,752)`**, and
its completed arrival is **`(504,769)`**. An early map-ID write while coordinates
still belong to D is not an exterior arrival witness.

```rust
use assets::maps::visual::StaticBackground;
let bg = StaticBackground::from_rom(authenticated_japanese_rom, 0x000a)?;
// width=64 cells, height=80 cells: 1024x1280 map-relative pixels.
let attributes: &[u8; 512] = bg.resources()[3].decoded().try_into()?;
let raw_collision = bg.layer().attributed_cells(attributes);
let pixel = bg.pixel(map_x, map_y)?; // IndexedPixel: transparent or index+priority
let natural_palette = bg.palette();
```

This is the existing CPU-free assets API with **one additional allowlisted
recipe**, not a new scene/event interpreter. The caller authenticates the ROM.
`layer()`, `resources()`, `tiles()`, `metatiles()`, `palette()` and `pixel()` retain
the same semantics. All returned resource bytes and source ranges are retained.
The synthetic tests relocate the resource pointers, decode a full 64×80 sheet,
and reject changed controls, table entries, destinations, invalid pointers,
truncation, wrong payload sizes and unsupported cell/definition bits.

**Host per-room background contract:** select an asset by the current room's
background key, not a single global indoor `map.bmp`. Indoor rooms may share their
512×1024 sheet; A requires a separate **1024×1280** sheet and palette. Supply
width/height with each index/priority plane or bitmap; never infer them from
camera bounds. Use map-relative coordinates and the current source camera. A
minimal descriptor is `{map, width_px, height_px, indices, priorities, palette}`,
with transparent index zero contributing neither color nor occlusion. Bind the
source profile, dimensions and omission policy into compiled content identity.
This is an integration contract, not a host implementation in this change.

The machine-readable source contract is `reference.json`'s `contract` member;
`check.py::source_contract(rom)` reproduces it. Exported planes remain ignored.
The `export` member pins their hashes and ROM extents, not production RAM data.

### Loading recipe

Map table `$8695BA` selects `$988350`. At root, `FE $0059` and `FE $006C` skip
opaque words; `F8` at `$988382` falls through because this is not a call. After
secondary definitions and `FE $0006`, execution falls through to the already
audited conditional audio tail `$988390`, whose alternatives share subscript
`$0010`. No FD predicate is evaluated by the visual decoder. Only packed source
pointer fields vary in the checked instruction windows.

| First-background resource | Headerless half-open source extent | Decoded size |
|---|---|---:|
| 4bpp graphics | `$1DC1E6..1DFDE7` | 24576 bytes / 768 tiles |
| Palette colors 32–127 | `$31E885..31E945` | 192 bytes |
| Definitions | `$2BB9FA..2BC334` | 4096 bytes / 512 four-word records |
| Attributes | `$31E3F8..31E4BD` | 512 bytes |
| Shared palette colors 0–31 | `$328B78..328BB8` | 64 bytes |
| First layer | `$29BAF5..29C8A1` | 64×80 cells / 10240 bytes, plus dimension prefix |

The secondary layer/definitions and shared OBJ graphics are omitted; their
instruction shapes and pointer starts are checked, not their full payload
extents. Natural first-layer art is deliberately not a full-frame compositor.

## Camera and independently verified first hardware background

A's scene lookup uses the zero bank-$82 entry and `$838014 → $8389A7`, whose
prefix is **`00 08`**, not indoor `00 06`. `$868C77..8C86` indexes
`$96BB64 + 2*(selector & $3F)`: selector 8 reads **`$96BB74 → $96BC2F`**.
The nine-byte profile there is:

```text
16 01 82 33 64 C0 09 ED 13
```

- +4 `$64`: `$0866=256` (bit 6), `$080C=1` (bit 7 clear).
- +5 `$C0`: bit 7 swaps assignment: **first logical layer is hardware BG2**;
  ring tilemap **VRAM word `$3800`**. Secondary layer becomes BG1 at `$3C00`.
  Hardware tilemaps are 32×32 **8×8 tiles**, not the source sheet dimensions.
- +6 `$09`: actual BGMODE write at `$868D6F`, mode 1 with BG3 priority.
- +5 bit 6 selects alternate secondary scrolling. `$868D7B..8D95` and `$868DC7`
  turn +7/+8 `$ED/$13` into `$086C/$086E = $02FF/$0201`. Exact scrolling cadence
  is not implemented here.

Source camera record `$96BE44 = 40 40` is decoded by `$869371..93F7`:
`left=(a&15)*256`, `right=left+(a>>4)*256`, and similarly for Y. Thus bounds are
**`[0,0,1024,1024]`**, despite the taller 1280-pixel source sheet. Under ordinary
follow (`$0868 & $0083 == 0`), `$8790A1..9105` follows entity `$0DEC`:

```text
camera_x = clamp(player_x - 128, 0, 768)
camera_y = clamp(player_y - 112, 0, 768)  // bottom - $0866, NOT bottom - 224
```

Both native camera pairs `$080E/$0812` and `$081E/$0822`, all four bounds, clamp
height, hardware shadows and secondary-scroll parameters match at the three
settled checkpoints. Native BG1SC/BG2SC shadows are `$3C/$38`. More importantly,
all **672 sampled full source tile words** match the BG2 ring at each checkpoint;
**zero** match the same-coordinate BG1 ring. This checks assignment independently
of the indoor assumption. It is source-consumer plus settled-ring evidence, not
a newly instrumented loading-writer trace.

### Priority and explicit visual omissions

For BG2/mode09, vendored ares ranks BG2 low=4, ordinary OBJ2=6, BG2 high=7.
Therefore **opaque high first BG > ordinary OBJ2 > low first BG** is valid for
this subset. Priority comes from definition bit `$2000`, never cell collision
bit `$8000`. Transparent color zero does not occlude.

A differs substantially from indoor final composition:

- TM `$16`: BG2/BG3/OBJ on main. TS `$01`: **BG1 only on subscreen**, not a
  normal foreground occluder. TMW/TSW use the same masks.
- CGWSEL `$82` selects subscreen color and main-color clipping inside the color
  window result. CGADSUB `$33` selects **saturating addition**, no halving,
  for BG1/BG2/OBJ/backdrop; OBJ palettes 0–3 are not math-eligible. Fixed color,
  window geometry and per-pixel effects are not reproduced by a blanket tint.
- The native app adds the secondary layer, the crystal clouds
  (`assets::maps::visual::SecondLayer`), onto the view with the animated
  tiles and colors; it scrolls with the camera and drifts one pixel left and
  down every three frames (`$0810`/`$0814` at the town checkpoints: 80
  pixels over 241 frames), added over the sprites as in the game. Its phase
  after the load is not matched.
- **Omitted:** subscreen/fixed-color/
  window/color-math composition; BG3 output; animation timing; transient text;
  shadows and unqualified actor output. Do not claim native-complete RGB.
- Source compact records `$838A73/$838A78` select graphics animation **0/1**.
  Native base-tile differences are **9–12 and 496–511**. Source tables start at
  `$9B807E/$9B84DF`; their 128-byte transfers cover these destinations. This is
  an inventory, not a timer/frame scheduler implementation. Palette-animation
  services `$838A7D/$838A82` select **$23/$06**; native colors **96–119** differ,
  as does runtime backdrop color 0. All differences are retained in the report.

The sampled interior viewport has **43008/43008 equal base index+priority
pixels**, and **zero opaque pixels using changed palette entries**, at all three
checkpoints. Animated graphics and palette omissions therefore do not invalidate
that bounded base-art comparison. This says nothing about the omitted additive
subscreen output in the final framebuffer or the unsampled edge strips.

## Collision and movement-material boundary

Build the source grid with `StaticLayer::attributed_cells`: each cell becomes
`cell | ((attributes[cell] & 127) << 9)`. Width is **64**, not indoor 32.
All 512 source attributes and all 512 definitions match native. The checker
independently rebuilds all **5120** source collision words and compares the full
native grid without a blanket flag mask. There are exactly **14 bit-15-only
occupancy differences** at each selected checkpoint; **no low-word mutation**.
Every difference is listed, including moving remote residents. No native stamp
is imported into compiled source data.

The bounded source-cell **sample halo** is half-open **`[29,47,36,53]`**:
7 columns × 6 rows, map pixels `[464,752,576,848]`. It contains only open types
**0/22**, and all 42 full words match native at all selected checkpoints. The
witnessed route remains inside it. Checkpoint player envelope is inclusive
**X 488..552, Y 768..832**; it is a conservative source-policy envelope, **not**
a claim that every possible trajectory in that envelope was replayed.

**Parent admission requirement:** every old/new collision sample must remain
inside the qualified halo, with ordinary action-free control and
`$0980 & $0050 == 0`. Reject attempts outside the qualified profile rather than
inventing invisible walls or treating the whole exported map as a passive room.
The portable movement qualification, arrival cadence and live admission policy
remain parent-owned. No new material is needed for the retained Down/Right path.
If the parent needs a wider route, it needs additional bounded qualification.

Whole-map stored-type counts: `0:3084, 2:1, 6:72, 7:59, 8:44, 12:1010,
14:432, 22:189, 25:229`. Existing open classes 0/2/22 and solid classes 12/14 do
**not** admit outdoor **6/7/8/25**. No type16 occurs here. Keep unsupported old-edge
6/7 rejection; do not broaden room-core for this assets task. In particular the
house-front door at column31, rows45/46 remains source **`$1CF2/$1CF3`** (solid14).
Do not apply the unrelated indoor opened-door patch to A.

## Bounded actor/overlay inventory, not a Crysta resident decoder

Fresh flags are exactly **`$0020,$0026,$00FB`**. Scene FA tests at `$8389A9`,
`$8389AE`, `$8389B3` concern `$01AC,$0199,$0196`; all are clear, selecting the
fallthrough list. Alternate-story rosters are not combined with it.

Ordinary source origins (not claims about settled AI position):

| Source record | Origin | Behavior header |
|---|---|---|
| `$8389BF`, `$8389C9` | `(536,304)`, `(552,304)` | `$8889FE`, `$888910` |
| `$8389D3`, `$8389DD` | `(808,384)`, `(792,320)` | `$888331` |
| `$8389E7`, `$8389F1` | `(168,288)`, `(776,768)` | `$888677`, `$888827` |
| `$8389FB`, `$838A05` | `(840,512)`, `(344,272)` | `$8885CC`, `$888788` |
| `$838A0F` | **`(680,816)`**, nearest southern resident | `$8886EB` |
| `$838A19`, `$838A23` | `(456,416)`, `(632,464)` | `$8880AD` |
| `$838A2D`, `$838A37` | `(568,416)`, `(904,672)` | `$888103`, `$8881EA` |
| `$838A41` | `(8,16)`, conditional register-effect controller | `$888038` |

Zero descriptors reuse preceding art, not invisibility. These origins lie
outside the selected landing/walking viewports. Native OAM census finds no
separate visible outdoor resident in those checkpoints; that is **not** a promise
about later movement, wide compositions or AI. No resident art/AI decoder is
added. The two nearby ordinary scripts have COP3B sites `$8886F3/$88882C`, but
this work does not compile their remote composition footprints.

Other selected records:

- `$8389B8`: FD Ark source origin `(504,880)`, **not arrival position**.
- `$838A4B/$838A52`: hidden FD controllers at `(904,752)/(72,640)`; conditional
  COP47 before COP3B. Their fresh native cells are not stamped.
- `$838A59/$838A60/$838A67`: hidden FD controllers at `(120,912)/(24,496)/
  `(872,480)`, immediate COP3B at `$88823F`. Their remote cells 3591/1921/1910
  appear among native occupancy deltas. These are not visible residents.
- `$838A6E → $8884EF`: compact animation/event controller, not a world-origin
  resident; includes alternate dialogue/control behavior. Not executed here.
- `$838A73..A86`: the four animation services inventoried above.
- `$838A87`: FF temporary text service, **not permanent map art**. At landing,
  OAM slots0..7 are eight OBJ3 16×16 pieces at Y48, X80..164. They are absent at
  both later walking checkpoints. This output must be omitted explicitly until
  the text owner supplies it; no glyph bytes are extracted into production here.
- Ark and its separate shadow remain player-presentation concerns. Native OAM
  visible-piece totals are **18 / 10 / 7**, respectively; the landing's eight
  OBJ3 pieces account for its additional screen-space group. OAM inventory is
  bounded to the witnessed OBSEL `$02`, 8/16 size mode, not a new renderer.

## Evidence and reproduction

Japanese normalized SHA-256:
`f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548`.
`reference.json` pins source windows/exports, the parent's finite route and probe
provenance, all six native surfaces per checkpoint, every grid/palette delta,
OAM metadata and source/native comparisons. No ROM, extracted art, capture or
save-state file is tracked.

| Checkpoint | Native frame | Position | Camera | High / low sampled words |
|---|---:|---|---|---|
| `landed-A` | 11853 | `(504,769)` | `(376,657)` | 63 / 609 |
| `exterior-walk-down-settled` | 11945 | `(504,815)` | `(376,703)` | 8 / 664 |
| `exterior-walk-settled` | 12059 | `(538,815)` | `(410,703)` | 8 / 664 |

Use the conversation owner's fresh journey (do not independently navigate):

```sh
cargo test -p assets --lib maps::visual::tests
cargo clippy -p assets --all-targets -- -D warnings
sh tools/house-exterior-qualification/replay.sh "$ROM" "$CONVERSATION_CAPTURE_ROOT"
# Optional source-only verification against a prior local export:
python3 -B tools/house-exterior-qualification/check.py "$ROM" "$SOURCE_EXPORT"
```

To reproduce native inputs after integrating the parent's tools:

```sh
sh tools/house-conversation-qualification/build.sh
local/house-conversation-qualification/probe/target/release/house-conversation-probe \
  "$ROM" local/exterior-fresh < tools/house-conversation-qualification/route.jsonl \
  > local/exterior-fresh.jsonl
sh tools/house-exterior-qualification/replay.sh "$ROM" local/exterior-fresh
```

The parent probe creates a fresh `Session::new` with empty SRAM, bootstraps New
Game and consumes only the retained real-button itinerary. **It calls
`oracle::save_state` to synchronize before capture**; state files are never loaded
or used by this checker. This is not a no-save-state-call witness. No warp, SRAM
seed, restore or WRAM patch is used. Native capture/probe provenance belongs to
the conversation qualification, not this assets implementation.

Local checks passed against the supplied sibling
`/Users/lainsoykaf/repos/ilar-task-house-conversation/local/house-conversation-qualification/journey`
at `local/house-exterior-qualification/run-XShRHq`. A same-itinerary fresh repeat
was already run at `local/house-exterior-qualification/repeated` before the
parent requested reuse-only; all selected pins match. The parent's independent
`/Users/lainsoykaf/repos/terranigma/local/house-conversation-qualification/replay-OYXAH7/journey`
also matches every selected pin in normal and optimized Python. No further boot
is needed. Both normal and optimized Python run the synthetic tests and native
checker. Assets and checker helpers followed red→green. Separate read-only
reviews approved the assets extension and the tooling/documentation contract
before their signed commits; reviewers inspected source, not executed tests.

Two reproducibility limits remain bounded: `source_windows` and
`capture_provenance` are audit metadata, not separately enforced by the checker
(which enforces whole-ROM, route, exported-file and native-report identities).
Capture freshness/probe provenance comes from the parent qualification. The
small local build manifest uses an ignored lockfile and is output-pinned, not a
fully dependency-locked build environment.

**Handoff/blockers:** first-background source, dimensions, camera, bounded base
collision and landing pixels are qualified. Parent must supply admission only
inside the sample halo and within the witnessed passive context. Full-map material/
actor admission and final exterior color-math/overlay fidelity remain explicitly
unsupported. No core, host, UI or tracker files are changed; issue closure remains
parent-owned.
