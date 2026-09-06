# Pandora first backgrounds

Owned scope: source compilation and existing-capture qualification for widened A,
13, E/20/21 and the controller-initialized 41–44 tour. No renderer rewrite.

## Parent compiler contract

`assets::maps::visual::pandora::{PandoraBackground, SourceCamera}`:

```rust,ignore
let compiled = PandoraBackground::from_rom(authenticated_japanese_image, map_id)?;
let background = compiled.background(); // &StaticBackground: resources/grid/pixels
let base = compiled.attributed_grid();  // &[MapCell], full source-derived sheet
let camera = compiled.camera();         // &SourceCamera, source transport
let origin = camera.settled_origin(player_xy);
let policies = compiled.policies();
```

The existing `StaticBackground::from_rom` nine-map allowlist (A–D, F–11, 128)
is deliberately unchanged. The Pandora compiler is a separate, explicit opt-in.
The full attributed sheet is **not** an admitted movement halo. It grants no
material/movement coverage and includes no resident/object stamps, door/pot
patches or dynamic phase. The parent owns route admission and source-derived
phase/occupancy changes; captured grids are never production initializers.

Maps41–44 compile a **controller-initialized tour profile**, not independent map
loads: 41 supplies definitions, controller `$89D24E` supplies `$E15DD5` graphics
and ordered bank-first COP5A palette transfers; 42–44 inherit them. Callers must
not use this compilation to infer standalone reachability or a standard recipe.

## Camera transport

The compiler retains the two-byte `$96BE30+2*map` record, source scene/display
addresses and display bytes, bounds, `$0866=256`, hardware background and VRAM
ring word base `$3800`. Source A/13 assign first layer to BG2; E/20/21/41–44
assign it to BG1. All use source BGMODE `$09`.

Ordinary settled origin is `clamp(player_x-128,left,right-256)` and
`clamp(player_y-112,top,bottom-256)`. The last 256 is **not** viewport height224.
Map21 scrolls vertically; 13, E/20 and each tour sector have fixed bounds.
Transition interpolation, scene reloads and forced camera motion are not this
pure settled clamp. Early `enter-13` captures still have the preceding camera;
qualification must use settled states, not weaken the source contract for them.

## Source resources and loading history

All extents below are normalized/headerless, half-open. Source packet bytes and
natural indexed planes are retained locally; `reference.json` commits only
metadata/hashes. The compiler validates instruction shapes/control flow and
map/subscript tables, while allowing resource operands to relocate. It bounds
all returned resources to their source bank and exact decoded extent.

| Map | Root | First-layer extent | Full cells | Camera record / bounds |
|---|---|---|---|---|
| A | `$988350` | `$29BAF5..29C8A1` | 64×80 | `40 40` / `(0,0,1024,1024)` |
| 13 | `$9884D7` → sub4 `$9884BF` → sub3 `$98841E` | `$2FB5A9..2FBA31` | 64×32 | `11 10` / `(256,0,512,256)` |
| E | `$988454` → sub1F `$988461` | `$2FCBB3..2FCFE1` | 32×64 | `10 13` / `(0,768,256,1024)` |
| 20 | `$9885E9` → sub1F | same | 32×64 | `11 13` / `(256,768,512,1024)` |
| 21 | `$9885F5` → sub1F | `$3281D1..328261` | 16×32 | `10 20` / `(0,0,256,512)` |
| 41 | `$98831C` plus controller | `$30AAC5..30ADEF` | 32×32 | `10 10` / `(0,0,256,256)` |
| 42 | `$988338`, layer only | same | 32×32 | `10 11` / `(0,256,256,512)` |
| 43 | `$988340`, layer only | same | 32×32 | `11 10` / `(256,0,512,256)` |
| 44 | `$988348`, layer only | same | 32×32 | `11 11` / `(256,256,512,512)` |

13 uses the existing house graphics `$23B11D..23E222` (decoded `$6000`),
definitions `$2ABA43..2AC506` (`$1000`), attributes `$30FF53..30FFFC` (`$200`),
main palette `$31E945..31EA05`, and shared palette `$328B78..328BB8`.
E/20/21 use those graphics/definitions/attributes/shared colors but **different
main colors** `$31EA05..31EAC5`. Merely sharing the house layer does not imply
the ordinary house palette. A keeps its independently qualified exterior loads.

### Controller tour, not a standard recipe

`initialization()` returns `Map21ThenController41Tour` for all four tour IDs.
The controller's `$89D253..D26C` instructions set `$66/$68` from `$E15DD5`,
write VRAM word base0, pass A=0 and call `$8684E1`. The packet extent is
`$215DD5..21777E`, decoded **`$3000` / 384 tiles**, not 768. Map41's definitions
are `$2DB000..2DB7E4` (`$1000`) and attributes `$3287EF..328856` (`$200`).
Unused definitions can refer to inherited VRAM, but **every cell of the full
returned sheet** uses only supplied controller tiles; the compiler rejects a
relocated sheet/definition that violates this. No captured VRAM padding is used.

The seven COP5A instructions at `$89D26C..D29D` use **bank-first** pointer bytes:
`BANK, ADDR_LO, ADDR_HI`. They write 112 colors at16 from `$B1DEFA`, then eight
colors each at24/40/56/72/88/104 from `$AFE45B/$AFE47B/$AFE49B/$AFE4BB/
$AFE49B/$AFE4FB`, in that order. The repeated source is retained. The smaller
transfers override the base transfer. The continuation calls `$86925C`.
Colors0–15 are **not** supplied by COP5A: compilation retains map21's validated
shared palette source for those colors, rather than inventing black colors.
Color0's later runtime backdrop remains an effect, not a natural palette match.

`background().resources()` keeps graphics, base palette, definitions,
attributes and shared palette at indices0–4. Tour entries5–10 are the six
ordered overrides. The separately retained `layer()` includes its original
container. Resource order is storage order, **not chronological transfer order**:
shared colors first, base controller palette next, then the six overrides.

## Qualification and explicit omissions

The dedicated checker reads **only WRAM, VRAM and CGRAM**, never `.pixels`.
For each of 17 selected settled states in **both** existing journeys it checks:

- All 512 source definition records equal native `$7E2000..2FFF`.
- The full source grid independently equals
  `tile | ((attributes[tile] & 127) << 9)`. Full native-grid differences are
  counted and hashed without suppressing bit15, tile changes or other attributes.
- Source camera bounds, both native camera pairs, clamp extent and first-layer
  hardware assignment. Source table/scene/controller fields are independently
  cross-checked against the delivered typed `source.json`.
- **Every intersecting ring tileword and all 57,344 viewport pixels** per state
  (256×224, clipped at partial tiles). Aligned cameras sample896 words; fine Y
  samples928 and fine X/Y957. Whole tilewords include flips, palette selectors
  and priority; source export sampling is checked independently against the
  SNES planar decoder.
- Tileword expectations come from the **source base grid**, never native WRAM.
  The suite separately requires zero visible low-tile phase changes, even if a
  changed tile ID has identical definitions. Full-grid deltas outside this
  qualification remain reported rather than admitted. Native memory never feeds
  a compiler or parent initializer.

All 17 selected background reports agree between the original and parent
journeys. Both high and low priority are covered across the set (not every
individual viewport has high-priority tiles).

| Selected states | Full-grid deltas (source base → native) | Natural-base pixel differences |
|---|---|---|
| A west/north/gap/gap-up/door-align rests | 13/13/12/14/13, bit15 only | 833 ring-alias pixels / 0 / 0 / 0 / 0 |
| 13 landed/west-rest | 2 each, bit15 only | 17 / 10 |
| E/20 left-rest | 5 each, changed tiles; 4 also change attributes | 0 each |
| 21 entry-closed/contact-rest/opening-wait | 0 / 1 bit15 / 0 | 0 each |
| 41 tutorial020, 44 tutorial034, 42 tutorial041, 43 tutorial045, final stable41 | 0 / 0 / 1 bit15 / 0 / 0 | 84 / 84 / 104 / 104 / 84 |

The five E/20 differences are the opened C door and three lifted pots in the
**C sector of the shared sheet**, outside the sampled cellar sector. They
correspond to the delivered door/carry progression, not a new cellar base.
Do not reset the shared sheet's route phase merely because map ID changes to
E/20. Conversely, do not grant full-grid collision admission from these results.
Actor/object occupancy, dynamic door/pot patches, wider movement/material halos
and their timing remain parent/source-phase-owner work. Attribute0 differs at
13/21 after load; those differences are also retained in the reference report.

### Fine-scroll ring edge, not a hidden border exclusion

At town-west camera `(236,703)`, the visible footprint intersects33 tile columns
but the hardware ring has32. Its 33rd column aliases the leading resident column
at the same VRAM addresses. The checker compares **that edge too**, mapping
`resident_x = first_tile_x + (tile_x-first_tile_x)%32`, and separately retains
its mismatch with a natural unwrapped source crop: **28 words / 833 pixels**.
All 57,344 resident-ring/source-animation pixel comparisons agree. This is not
natural-crop equality at that edge and not a reason to change the source sheet.
No other selected state has a horizontal alias difference. A nonuniform synthetic
fine-X/Y fixture tests partial edges, alias counts and shared-ring-slot mutation.

### Source-selected animation, not arbitrary changed-tile masks

The checker reconstructs all supplied graphics bytes using bounded source-table
payloads before comparing pixels. It never exempts whichever tiles happen to
change. Scene records `$838A73/$838A78` select A graphics animations0/1 at
`$9B807E/$9B84DF` (tiles9–12 and496–511). Record `$838EED` selects13's animation
F at `$9BE5ED` (tiles37–40). Record `$839559` selects tour animation3C at
`$9CE240` (tile341). Eight-byte records specify repetition/source/destination/
size/delay; native spans must exactly match source-selected payloads. The tour
animation persists into42–44. Production compilation returns natural base art,
not the observed phase or an animation scheduler.

Natural palette differences are reported separately: runtime backdrop0 in all
profiles, plus A's animated colors in96–119. **No whole-frame RGB equivalence**
is claimed. Secondary layers, sprites/text, windows, color math, brightness,
animation timing and the historical cellar-band RGB discrepancy are omitted
layers/effects—not reasons to alter source art or suppress mismatches.

## Reproduction and acceptance status

```sh
sh tools/pandora-background-qualification/compare.sh "$ROM" "$SOURCE_JOURNEY" "$PARENT_JOURNEY"
cargo test -p assets
cargo clippy -p assets --all-targets -- -D warnings
```

The wrapper builds a local source exporter, runs normal/optimized Python tests
and strict comparisons. It never boots an emulator or captures a new frame.
`--record` is an audit aid for initial metadata review, not an acceptance bypass;
it still requires source/parent nonpixel reports to agree and all semantic
comparisons to pass. No source pixels, palette payloads or captured grids are
committed. ROM-backed outputs remain ignored under `local/`.

The parent reports that the original exact checker failure was isolated to
intermittently zeroed regions in a handful of `.pixels` files: an unsynchronized
async video-publication race in `vendor/ares/shims.cpp` (source diagnosis
`5b7b88b`, parent `42fe162`). Frame logs/native state/nonpixel captures and
provenance agree in a further fresh same-binary replay, including final control
pixels. **Do not use `.pixels` as uniformly trustworthy RGB evidence yet.**
Observer repair and renewed strict route replay are parent-owned and pending;
this background comparison does not claim accepted parent route replay.

TDD covered missing compiler/camera/comparator APIs, relocated synthetic source
resources, negative control/table/extent mutations and palette overwrite order.
Independent compiler review found no blocker and prompted stronger nonzero
inherited-color and distinct-layer tests. Independent source-pixel review prompted full-viewport coverage, explicit ring
alias reporting, a visible-phase rejection gate and independent camera-table
linkage. Input-level mutation tests cover real ring tile/palette/priority/flip
changes, every graphics plane, a coordinated grid/ring change, camera metadata
and native pairs, missing/one-sided captures, and invalid animation payloads or
destinations. Normal and optimized Python pass. The reviewer found no remaining
blocking source-pixel issue; the requested nonuniform fine-scroll regression was
also added and passes.

The public camera is a transport: `settled_origin` expects its unmodified
source-derived bounds (documented panic precondition for manually manufactured
invalid bounds).

## Opt-in preview sheet transport

The host's opt-in art profile compiles four additional bitmaps directly in memory:
`town13` (1024×512), `cellars` (512×1024), `box` (256×512), and `tour` (512×512).
E/20 and 41–44 reuse their respective sheet **only after complete rendered bitmap
byte equality**, not an assumed common map/grid. E/20 do not reuse `/map.bmp`:
their source palette differs from the ordinary house. Per-map masks preserve
opaque-high first-BG coverage, and camera metadata retains the source bounds,
256-pixel vertical clamp extent, hardware BG identity and mode.

The additive `pandora_backgrounds` artifact remains separate from the live
house manifest pending runtime/frontend admission. Exact loopback-only bodyless
GET routes `/town13.bmp`, `/cellars.bmp`, `/box.bmp`, `/tour.bmp` serve only those
in-memory capabilities; the current house-only profile returns **404**. No file
lookup, query selector, alternate origin, mutation endpoint or CPU execution is
added. Host visual policy remains natural palette/checkerboard transparency,
not native whole RGB; phase patches and transition camera pans are not inferred.

TDD covers every emitted BGR pixel, high mask and camera for all eight maps,
shared-sheet equality, the separate cellar palette, old bundle preservation and
exact-route rejection. Parent host tests and strict workspace Clippy pass; logs
`local/map-research/pandora-background-host-{green,clippy}.txt`. The previously
qualified source/native background comparisons remain the evidence boundary.
