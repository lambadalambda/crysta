# Playable house boundary: semantic New Game

The portable house subset provides an explicit **New Game** operation derived
from the owned Japanese ROM, without supplied SRAM, a restored emulator
checkpoint, or original CPU execution in its simulation loop.

**This is a limited semantic start, not a complete port of the opening.** It
uses the default name and explicitly completes/omits intro presentation and
conversation waits. Ark now uses ROM-derived ordinary standing/walking sprites
over the static first background (hardware BG2), with one frozen ordinary-pose
resident in room10. NPC behavior/collision, other residents, dialogue,
inventory/stats, combat and audio are not implemented. Doorways use endpoint-qualified logical timing, not the native
loader's video-frame schedule. The frontend still uses a native Rust host;
the same device-free core builds for Wasm but browser Wasm glue is not yet wired.

## Start and walk

```sh
cargo run -p map-inspector -- serve-room 'local/Tenchi Souzou (Japan).sfc' 8765
```

Open **http://127.0.0.1:8765/**, choose **New Game**, then **Resume**. Use one
Arrow/WASD direction at a time, or hold a touch cardinal button. Release before
turning; Escape, blur and hidden tabs pause and release controls. Ordinary
release/repress and reversals are supported. Very rapid same-direction double
taps select an unimplemented dash and report an error; New Game starts cleanly.

The covered area is **bedroom F and adjoining house room10**, with their shared
doorway traversable both ways. Other exits remain explicit scope errors; this
is not all house interiors, outdoor Crysta, or the Pandora opening. Partial
furniture and passive flagged walls use source-qualified collision responses.
Pushing, attacks and collision action hooks are outside the cardinal-only policy.

**Checkpoint Reset** and **Checkpoint doorway demo** deliberately retain the
older saved-position diagnostic start `(472,176)`. They are not New Game and
are not fresh-start acceptance evidence.

## Source-driven initialization

The adapter authenticates the annotated ranges in
`tools/new-game-qualification/sources.json` against the ROM. It projects:

1. New Game reset semantics clear the selected64-byte event block; the pinned
   default table does not reseed it.
2. Name-entry setup sets event`$00FB`. Explicit semantic intro completion sets
   `$0020`, preserving that default. These are decoded COP07 bit assignments,
   not captured flag bytes or a general event interpreter.
3. Prologue COP14 requests mapF, mode0, selector0, queue`(296,96)`.
4. The map's FD player record/header qualifies queue override: `(8,16)` yields
   **`(304,112)`**. The record's ordinary default`(312,112)` is not selected.

Only the projected startup data needed by this house subset is compiled. The
unused actor/scheduler/inventory subsystems are not secretly initialized from
WRAM. Full reference evidence and explicit omissions are in
[new-game-bootstrap.md](new-game-bootstrap.md).

Fresh bedroom collision data includes frozen runtime flag additions at317 and504.
After map loading, the returned-bedroom profile has only317. The core keeps
these immutable profiles distinct and discards the fresh-room selector at the
first map switch. Map10 has additions731,732,826,827. Ongoing event/NPC writers
are not simulated; the passive policy assumes `$0980 & $0050 == 0`.

## Deterministic end-to-end route

```sh
# Two real empty-SRAM boots, including native control/loader evidence:
sh tools/new-game-qualification/route-replay.sh 'local/Tenchi Souzou (Japan).sfc'
# Source initializer, full fresh bootstrap, negative controls and native writers:
sh tools/new-game-qualification/replay.sh 'local/Tenchi Souzou (Japan).sfc'
# CPU-free source/asset compiler and complete portable route:
cargo run -p map-inspector -- verify-house 'local/Tenchi Souzou (Japan).sfc' semantic-preview
cargo test -p map-inspector --test local_house
# Require authenticated per-step fresh-route comparisons rather than optional skip:
HOUSE_ROUTE_FIXTURES="$PWD/local/new-game-qualification/fresh-house-route" \
  cargo test -p map-inspector authenticated_fresh_route -- --nocapture
```

| Logical tick | Map | Anchor | Next owner |
| ---: | --- | --- | --- |
| 0 | F | 304,112 | walking, fresh New Game |
| 167 | F | 392,208 | outbound departure |
| 202 | 10 | 392,353 | walking |
| 266 | 10 | 392,336 | return departure |
| 301 | F | 392,191 | walking, reloaded bedroom |
| 361 / 411 / 461 / 511 | F | 420,191 / 392,191 / 420,191 / 392,191 | repeated walking/release |

The route contains **441 reference-qualified ordinary steps** across three
segments, plus **70 semantic doorway updates**. The checker preserves core
state across both transitions; it does not teleport between reference segment
starts. It compares positions, attempted streams, ROM-compiled grids and
per-step snapshot-restored continuations. Both raw reference boots and both
fresh CPU-free verifier processes must agree.

The fresh outbound handoff is208; the prior saved route lands at209. Both are
qualified separately:17 departure updates end225 or226, respectively, followed
by spawn336 and settled353. The reference6968 sample already belongs to the
transition; it must not be admitted as another walking frame.

Profile7 slice snapshots are103 bytes. Byte99 identifies the fresh
bedroom overlay; bytes100–102 retain animation facing, walking ownership and phase; invalid room/transition combinations reject. Transition-owned
snapshots contain no walking component, preventing marker erasure from creating
walking ownership. Walking encoding remains16-byte v3. Source/content identity
includes the startup projection and all three room profiles. Animation ownership,
direction and cadence must be coherent with walking or the doorway policy.

## Actual browser verification

With the host running, use the `agent-browser` skill workflow, then:

```sh
agent-browser open http://127.0.0.1:8765/
agent-browser eval --stdin < tools/verify-house-browser.js
```

The harness clicks the actual New Game button, observes the real host's fresh
state, resumes, then sends keyboard events through the page's bindings. A DOM
tick observer selects subsequent inputs; it does not send movement API requests
or substitute a fake host/controller. It checks every tick is sequential, nine
route checkpoints, the final live state, and absence of scope-error pauses. An
independent pixel compositor checks the actual canvas at all512 states (including
initial), using the decoded sprite pixels, signed bounds, camera and high-opaque
background mask. It does not call the page renderer to produce expectations.
The result is511 real host steps with New Game origin retained.

Raw ROM-derived captures, screenshots and verification logs stay ignored under
`local/`; only source, selected numeric metadata and hashes are committed.

## Ark rendering boundary

The native adapter compiles21 ordinary Ark ROM frames plus7 exact horizontal
mirrors and one resident raster once, exposing a fixed `/art.json` route. The state supplies the frame key; the
browser never derives animation from elapsed wall time or position differences.
Frames retain source anchors and transparency. Mirrored components have native
alternate offsets, so they are composed before browser rasterization rather than
mirroring a cropped normal image. Missing art is a visible paused error, not a
fallback marker. The old marker exists only in the renderer helper's diagnostic
no-actor branch; the live page never uses it.

Each ordinary walking record lasts9 ticks; six records make a54-tick cycle.
The input-delayed active direction drives animation, including blocked holds.
Turns reset the cycle; delayed release stands in the retained direction. New
Game chooses ordinary Down standing. Indefinite idle fidgets and native doorway
animation are not implemented: handoff through arrival uses ordinary standing
in the doorway direction. This does not reproduce the reference's special
idle gesture at fresh frame6800.

The selected OBJ components all use priority2. In the qualified house mode1,
opaque high-priority pixels of the first background cover Ark; Ark covers its
low-priority pixels, while transparent background pixels never cover him. Both
rooms share the same decoded full background sheet. Other hardware layers,
windows, sunlight/color math, shadows and equipment effects remain omitted;
this is not full-scene native screenshot equality.

See [ROM sprite evidence](ark-sprites.md) and
[ordinary animation qualification](ark-animation.md) for source ranges, native
register/tile/palette/composition witnesses and reproducible fresh-boot commands.

## First house resident

Room10 now displays the resident at **(424,416)**, frozen in the source-derived
ordinary Right-facing pose. The spawn comes from ROM record`$83:8D7C`, not a RAM
snapshot. Its16×33 raster uses the shared component decoder with palette base208
(Ark uses128), native signed anchor`(-8,-33)` and OBJ priority2. It is present
only while map16 is loaded, including the explicit semantic doorway phase;
returning to map15 removes it. The neighboring resident is not rendered yet.

The host supplies an ordered `scene:[{key,position}]` alongside the player state.
For the qualified ordinary pair, painter order uses worldY before sprite-anchor
subtraction; equalY puts the NPC first and Ark last. The browser consumes that
order without inventing entity simulation. Ordering on either side and the tie
are covered synthetically; the current route stays above the NPC and does not
claim a native overlap capture. Opaque high background pixels still cover both.

**This is presentation only.** There is no NPC interaction, collision, AI,
conversation or event-condition runtime. No NPC state enters the player snapshot.
Initial/final snapshot hashes are unchanged from the Ark-only milestone. The
frozen resident is the selected fresh ordinary-house policy, also used by the
diagnostic checkpoint preview—not a reconstruction of arbitrary saved NPC state.

[House NPC qualification](house-npc.md) records the complete ROM resource chain,
source ordering and fresh hardware/image evidence. A two-boot replay in the
integrated checkout (`local/house-npc-qualification/replay-QNlHv5`) passed all
four settled samples, including313/313 opaque reference pixels each,256 graphics
tiles and16 palette words. The shared compositor's large-component column15
wrap remains unqualified, but neither selected large component uses that case.

## Verified result

The earlier marker baseline passed two actual-browser511-step runs with
byte-identical checkpoint/final JSON, ending F392,191 with no errors. The UI also starts cleanly at304,112 at390px
width with no horizontal overflow; browser error log is empty. Desktop/mobile
screenshots and JSON stay under `local/map-research/playable-house-*` and
`new-game-browser-{green,repeat}.json`.

Final gates passed: required authenticated fixtures, full workspace tests,
strict workspace Clippy, native and Wasm core builds, formatting, Node UI tests,
repository safety and tracker checks. Independent core, source-compiler, UI and
integration reviews found no blockers. Fresh bootstrap repeated twice with both
negative controls rejected and22 source-writer stops authenticated; the separate
fresh house route also repeated twice with661 identical captured rows.

The sprite milestone repeated the full workspace tests with required walking,
admission, material and fresh-house fixtures, strict all-target Clippy, the Wasm
core build, Node controller/renderer tests and safety/tracker checks. Two new
actual-browser511-step runs produced identical results with **512 full-canvas
pixel checks per run** and21 distinct rendered keys. The full28-raster transport
is additionally checked against ROM composition by Rust tests. Browser errors
were empty;390px width had no horizontal overflow.

Fresh source qualification was rerun independently in the integrated checkout:
animation `replay-59Ly6n` (827 per-step comparisons per set, twice) and sprites
`replay-IzVj7x` (two boots, selected hardware/image evidence and28 exports).
Logs/screenshots stay under ignored `local/map-research/ark-*`. Independent
core, assets, native transport, frontend and browser-verifier reviews passed.

The resident integration passed two identical511-step browser routes with512
full-canvas comparisons each. Each run includes99 room10 states and30,987
visible NPC-pixel comparisons (313 per state), checking source placement,
membership and painter order. Browser errors were empty,390px layout had no
overflow, and both initial/final player snapshot hashes match the Ark-only runs.
All29 transport rasters are tested, including the pinned NPC RGBA export hash.
Full workspace tests with required movement/house fixtures, strict Clippy,
Wasm build, normal/optimized qualification checks and independent review pass.
Private logs/screenshots are under `local/map-research/house-npc-*`.
