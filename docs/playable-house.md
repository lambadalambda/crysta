# Playable house boundary: semantic New Game

The portable house subset provides an explicit **New Game** operation derived
from the owned Japanese ROM, without supplied SRAM, a restored emulator
checkpoint, or original CPU execution in its simulation loop.

**This is a limited semantic start, not a complete port of the opening.** It
uses the default name and explicitly completes/omits intro presentation and
room-entry conversation waits. The northern room B resident’s progression
conversation is now supported, with original Japanese text and explicit choices. Ark now uses ROM-derived ordinary standing/walking sprites
over the static first background (hardware BG2), with **nine frozen fresh-setup
residents and F's table object across six rooms**. Source-setup occupancy remains
solid; NPC movement and other residents’ interactions are not simulated.
Inventory/stats, combat and audio are not implemented. Doorways use endpoint-qualified logical timing, not the native
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

The covered fresh area is **B,C,D,F,10,11**, with the source-qualified internal
doorways traversable both ways. To open C's wooden door, face Up against it at
`(136,352)`, release movement, then press **Space/Enter** or **Interact**. The
one-shot action pauses; choose Resume to continue. The final ROM metatiles and
collision patch persist across the house and reset with New Game/Checkpoint Reset.
This does not grant story event`$0026` or trigger NPC dialogue.

After the B conversation grants `$0026`, reload D to open the exterior gate.
Cellar C/E progression remains closed; attached E/20/21 scenes
and exceptional F/122 are not admitted. Unsupported exits remain scope errors,
not free warps. Partial furniture and passive flagged walls use source-qualified
collision responses. Pushing, attacks and general action hooks are unsupported.
See [the census](house-scene.md), [background profiles](house-backgrounds.md) and
[navigation contract](house-navigation.md) for the full graph and policies.

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

Profile9 slice snapshots are181 bytes. They retain the original house state,
then a64-byte event block, loaded-D gate history, active request/cursor and
progression capability. Profile8 snapshots reject rather than silently migrating.
Aggregate source/content identity binds the house/startup, conversation graph and
ordered source pages, exterior grid, camera and bounded sampling policy. Walking,
transition and dialogue ownership are mutually exclusive; every admitted action
is checked against a snapshot-restored continuation. See the current
[core snapshot layout](../crates/room-core/README.md).

## Talk and leave

From C’s open wooden door, continue north into B. Face Up toward the northern
resident at Ark `(120,128)`, release movement, then **Interact**. Room-entry text
is omitted and cannot grant progression. **Continue** acknowledges a real page;
retained choice tails are not extra Continue waits. The first acknowledgement
sets event`$0026` **before** the first explicit choice and its follow-up finish.
Choose either displayed option or **Cancel choice**; cancellation follows the
source’s second-result branch rather than aborting the conversation. Repeat talks
start immediately at the repeat choice and do not grant another event.

Walking cannot advance text or select a default response. Use Tab/Enter on the
choice buttons, and Resume after the conversation closes. New Game and Checkpoint
Reset can interrupt a page or choice and clear the grant. An already-loaded D gate
keeps its old state; reloading D after the grant removes its collision stamp. The
normal B→C→D journey necessarily reloads it.

Leave D south through `(120,720/721)`. The semantic17/load/17 handoff reaches mapA
at `(504,769)`. A separate ROM-derived1024×1280 background follows Ark using the
source1024×1024 camera bounds. Only the small landing’s collision-sample halo
`[29,47,36,53]` (half-open cell coordinates) is admitted: stepping beyond it reports
a scope error atomically, not an invented wall. No outdoor NPCs, exits or full-town
simulation are promised. The qualified Down32 / release, Right24 / release route
ends at `(538,815)`. See [conversation evidence](house-conversation.md),
[text boundaries](house-dialogue.md) and [exterior limits](house-exterior.md).

Focused host regression (owned ROM required to avoid the optional skip):

```sh
cargo test -p map-inspector source_conversation_branches_restore_and_leave_through_gate
```

It checks all nine first/repeat result combinations, the early flag boundary,
wrong-action no-ops, frozen movement, reset interruption and the exterior endpoint,
with each admitted action repeated from its restored snapshot.

## Actual browser verification

With the host running, use the `agent-browser` skill workflow, then:

```sh
agent-browser open http://127.0.0.1:8765/
agent-browser eval --stdin < tools/verify-house-browser.js
# Full six-room route; retain results so long runs cannot lose them to CLI timeout:
node <<'JS' | agent-browser eval --stdin
const fs=require('node:fs');
const route=fs.readFileSync('tools/house-navigation-qualification/core-route.jsonl','utf8').trim().split(/\r?\n/).map(JSON.parse);
console.log('globalThis.HOUSE_BROWSER_ROUTE='+JSON.stringify(route)+';globalThis.HOUSE_BROWSER_RESULT=null;');
console.log('('+fs.readFileSync('tools/verify-house-browser.js','utf8')+').then(result=>{globalThis.HOUSE_BROWSER_RESULT={ok:true,result}},error=>{globalThis.HOUSE_BROWSER_RESULT={ok:false,error:String(error)}}); "started";');
JS
# Once the page pauses at tick2244, read the retained result:
agent-browser eval 'globalThis.HOUSE_BROWSER_RESULT'
# Clear HOUSE_BROWSER_ROUTE and set HOUSE_BROWSER_HELPERS_ONLY=false before
# repeating the default511 route in a page used by the conversation harness.
```

The harness clicks the actual New Game button, observes the real host's fresh
state, resumes, then sends keyboard events through the page's bindings. A DOM
tick observer selects subsequent inputs; it does not send movement API requests
or substitute a fake host/controller. It checks every tick is sequential, nine
route checkpoints, the final live state, and absence of scope-error pauses. An
independent pixel compositor checks the actual canvas at all512 states (including
initial), using the decoded sprite pixels, signed bounds, camera and high-opaque
background mask. It does not call the page renderer to produce expectations.
The default route is511 real host steps with New Game origin retained. The
full itinerary has2,244 steps and15 pinned checkpoints across all six rooms;
it exercises the actual Interact button and explicit Resume after its reply.
Both modes validate the complete fixed source roster, exact room membership,
world-Y/tie order and canvas pixels; the full route additionally requires
nonvacuous visible evidence for every resident. Open-door pixels and foreground
replacement are composed independently from the transported source patches.

The talk-and-leave browser harness uses the same independent scene compositor,
actual controls and sequential tick checks, plus fixed source page boundaries,
flag-before-choice assertions and nonblank dialogue/choice canvas pixels:

```sh
{
  echo 'globalThis.HOUSE_BROWSER_HELPERS_ONLY=true;'
  cat tools/verify-house-browser.js
  echo '; globalThis.CONVERSATION_BROWSER_READY=true;'
  cat tools/verify-conversation-browser.js
} | agent-browser eval --stdin
# Returns immediately; inspect the retained result after the run:
agent-browser eval '({status:CONVERSATION_BROWSER_RUN.status,error:CONVERSATION_BROWSER_RUN.error,result:CONVERSATION_BROWSER_RUN.result})'
```

Raw ROM-derived captures, screenshots and verification logs stay ignored under
`local/`; only source, selected numeric metadata and hashes are committed.

## Ark rendering boundary

The native adapter compiles21 ordinary Ark ROM frames plus7 exact horizontal
mirrors and ten instance-keyed setup rasters once, exposing a fixed `/art.json` route. The state supplies the frame key; the
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
low-priority pixels, while transparent background pixels never cover him. All six
rooms share the same decoded full background sheet. Other hardware layers,
windows, sunlight/color math, shadows and equipment effects remain omitted;
this is not full-scene native screenshot equality.

See [ROM sprite evidence](ark-sprites.md) and
[ordinary animation qualification](ark-animation.md) for source ranges, native
register/tile/palette/composition witnesses and reproducible fresh-boot commands.

## Complete fresh house presentation

The source roster has B1,C4,D1,F0,10two,11one residents, plus F's table child.
Positions, selected first-list-record poses, palettes, actor mirrors and stable
source identities come from `HouseScenes`, not captured RAM. D deliberately
uses its source creation origin `(72,672)` and setup pose rather than a later
wandering capture. C/11 list timing is retained by the decoder but not scheduled.
The same frozen fresh presentation is used for the diagnostic checkpoint; it
is not a reconstruction of arbitrary saved NPC state.

The host supplies ordered `scene:[{id,key,position}]` entries and an immutable
`scene_ids` manifest. Unique actor membership is separate from raster identity.
World Y precedes the explicit source draw-list tie rank; Ark is last at equal Y.
The browser consumes this order without inventing entity simulation. All admitted
objects use OBJ2; opaque high BG2 pixels cover them, transparent pixels never do.
Shadow, secondary layers, transient labels, windows, sunlight and color math
remain omitted. In particular C's seated lower bodies and F's right-side final
layer/effect composition are not full-frame fidelity claims.

`/art.json` also provides two complete source-decoded wooden-door replacement
rasters and their per-pixel priority. The page prepares both closed/open sheet
variants once, replacing old foreground pixels as well as background pixels.
It selects only the core's `wooden_door_open` state; the page does not invent
door events or collision.

[House scene qualification](house-scene.md) records all ten actors, 28 bounded
list records and source/native checks. Parent fresh replays passed at ignored
`local/house-scene-qualification/art-rA75pW`,
`local/house-background-qualification/run-a76GGH` and
`local/house-navigation-qualification/replay-xF3xFu`. The first resident's prior
313-pixel raster remains byte-identical through the shared decoder.

## Complete-house acceptance

Two fresh actual-browser **2,244-step** runs produced identical complete results:
**2,245 full-canvas comparisons per run**, all six rooms,15 checkpoints, and
**1,626,662 visible resident/table pixels**. Every one of the ten instances had
nonvacuous visible evidence; no hidden-actor exemption was used. Final state was
room10 `(360,463)`, door open, paused, no error. The legacy511 route also passed
with512 full-canvas comparisons and ended F `(392,191)`, door closed.

The final suite passed287 workspace tests with required ordinary walking,
admission, material and fresh-house fixtures; strict all-target workspace
Clippy; native/Wasm core builds; Node UI and seven browser-verifier tests;
source-qualification negative controls; safety and tracker checks. Independent
component and final cross-component reviews found no remaining blockers.
The390px layout had no horizontal overflow and browser errors were empty.

Retained browser JSON is ignored under `local/map-research/whole-house-browser-`
`{green,repeat,legacy511}.json`; desktop/mobile screenshots are local only.
The long full-route CLI initially hit its socket read limit after the browser
continued to completion; accepted runs use the retained in-page result, not
an inferred success from the final coordinates.

## Earlier milestone verification

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
