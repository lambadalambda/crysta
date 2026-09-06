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

## Verification boundary

Synthetic source/mutation tests, the complete assets suite and authenticated
Japanese house/cavern/exterior regressions pass. Independent compiler review
found no blocker; nonzero inherited-color and distinct relocated-layer tests
were strengthened after review. The fullviewport source-pixel comparator has
also undergone independent review; its reproducible tooling is the next topical
commit. No captured RGB or accepted parent route replay is claimed.
