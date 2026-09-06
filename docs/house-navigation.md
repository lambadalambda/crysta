# Source-qualified fresh house navigation

## Scope and integration status

The dependency-free core now admits immutable **B,C,D,F,10,11** profiles and their
internal doorways. The legacy F/10 constructor and **511-step** demonstration
remain valid. This is `SemanticPreview`, not a native actor/event scheduler.

The source/native qualification below covers the bounded door mutation, new
exit endpoints and selected ordinary walking segments. With the sibling's
[background/collision source profiles](house-backgrounds.md), a **2,244-step
CPU-free itinerary** now visits all six rooms from `GameState::new_game`, opens
C→B by the action API, retains the patch, tests the closed exterior and traverses
the additional C↔10↔11 links. Every logical step restores and checks continuation.
No captured grid or checkpoint initializes `GameData`. The parent still owns
production adapter/host/UI integration; this tool is a qualification client.

D→A exterior and C→E cellar stay closed. F→122 remains unqualified. Attempting an
unsupported exit is an atomic error, not a silent wall or a free warp. Normal
fresh D gating additionally requires the source-created hidden occupant; omitting
it from collision data is not an acceptable way to keep the exit closed. D's
wandering resident/warning AI is not implemented; a frozen renderer must say so.

## Stable adapter API

```rust
use room_core::{Room, slice::{HouseRoom, GameData, NewGameData, DataIdentity}};

// Complete source-ID order, NOT camera order or unordered destination lookup.
let profiles: [HouseRoom; 6] = /* B,C,D,F,10,11 */;
let data = GameData::new_house(profiles, identity, new_game)?;
// Each HouseRoom { map_id: u16, collision: Room, exits: Vec<Exit> }.
```

- Supply complete ordered raw 12-byte ExitList records, including the unsupported
  outgoing boundaries. Core validates the pinned source records and list order.
- Supply closed, **source-compiled** 32×64-cell collision grids, including qualified
  static occupancy under fresh events `$0020,$00FB`. Source room loading,
  attributes, occupant identity/conditions and immutable data hashes belong to
  the compiler, not runtime captures. `NewGameData` retains the existing shared
  bootstrap-derived fresh F overlay and `(304,112)` anchor.
- The compiler must include D's hidden `$83:8CC8 → $88:A9AF` occupant at `(120,720)`
  while `$0026` is clear. Ordinary residents are not silently walk-through art.
- `Room::new_passive` admits raw bit15 as passive solid **only** under its existing
  action-free collision contract (`$0980 & $0050 == 0`). Do not mask away flags.
  Materials outside the qualified `Room` dispatch still produce `Unqualified`,
  not invented wall behavior. This work does not broadly change `WalkingState`
  or `FrameInput`.
- `DataIdentity` authenticates the normalized JP ROM and the complete ordered
  compiled recipe/policy, including occupancy, fresh overlay and door resources.
  Construction validates bounded structure, not the caller's SHA computation.
- `GameData::new([Room;2], [Vec<Exit>;2], identity, new_game)` remains the compatible
  F/10 wrapper. It does **not** enable the new interaction or other-room exits.

### One-shot action / renderer contract

`data.door_interaction()` is capability, not target availability. The parent may
expose it as `door_interaction:true` and route its serialized command5 to:

```rust
let output = state.interact(&data)?; // one successful atomic logical tick
let open = state.wooden_door_open();
```

Admission is deliberately narrower than native tile-class dispatch:

- map **C**, player anchor **`(136,352)`**, animation facing **Up**;
- closed door, non-fresh-F state, no transition;
- host submits one shot after releasing direction, never held/repeated, and not
  during a demo/transition. Core does not receive the physical held-key state.
- A residual delayed Up does **not** require manually draining hidden frames.
  Successful action cancels walking history, stands Up at the same anchor,
  selects the final patch and advances one tick. This is an explicit semantic
  control-handoff policy, **not a source-proved native dash-history reset**.
- Already open, wrong map/anchor/facing, or unsupported action target returns
  `SliceError::Interaction` without changing tick, history, animation or patch.
  Wrong data and tick overflow also reject atomically. No NPC fallback runs.

`wooden_door_open` is a retained **live shared-sheet mutation**, not a native event
or save flag. It is initially false and serialized. Rendering must select the
same final metatiles as collision, in every view of the shared sheet:

| Cell / world origin | Closed raw | Final raw | Low-nine visual selector |
|---|---|---|---|
| `(8,19)` / `(128,304)` | `$1CF2` | `$1CF6` | `$F2 → $F6` |
| `(8,20)` / `(128,320)` | `$1CF3` | `$00F7` | `$F3 → $F7` |

Core constructs a private immutable open-C clone; it never mutates supplied
`Room` data. The upper tile remains solid, the lower becomes open. The renderer
must replace **all four descriptor words per metatile**, not just collision or
one 8×8 constituent. Both final descriptors contain four `$0801` words in the
qualified common resource. Native C VRAM changes exactly word addresses
`$38D0,$38D1,$38F0,$38F1,$3910,$3911,$3930,$3931`.

The flag survives all six supported shared-sheet rooms. Full sheet unload/reload,
SRAM persistence and unsupported exterior/cellar phase changes remain unqualified;
no behavior across those boundaries is invented. B's entry dialogue is explicitly
omitted by policy. **Neither entry nor this door action grants `$0026`**; that
requires the later, unimplemented NPC conversation at `$88:8F08`.

## Source mutation, conditions and resources

Native registration `$84:8A1E` uses `COP2D $0080` (held A), targeting `$87:923F`.
`$80:906F..90AC` tests `(held[$0454] & mask) == mask`, not an A edge. There is a
separate `$0020` registration; it is not an additional A prerequisite.

The Up path tries eligible actor rectangles through `$87:C783` **before** tile
interaction. `$0DC2` must be zero. `$87:C7F1` checks vertical sample alignment;
with `$04F6 == 0` it recognizes low-nine `$F3` in raw `$1CF3` (nonzero `$04F6`
would require full raw `$00F3`). Class3 at `$87:C8E7` consumes held A and dispatches
through `$87:9444` to `$87:97CA`, taking the `$097C & $8000` control lock. The
native F3 branch itself has no map-ID or event test; the portable C target is a
bounded admission policy, not a claim that native code hardcodes C.

At the qualified anchor, the sample is `(136,328)`; `$87:9724` sets interaction
entity `(136,336)`. COP44's signed-byte offsets are multiplied by16 at `$80:BC2F`.
The four zero-offset writes are:

| COP44 site | Cell | Selector | Policy |
|---|---|---|---|
| `$87:97EF` | `(8,20)` | `$F5` | native intermediate, omitted |
| `$87:97FF` | `(8,19)` | `$F4` | native intermediate, omitted |
| `$87:9819` | `(8,20)` | `$F7` | final |
| `$87:9829` | `(8,19)` | `$F6` | final |

The operand `$20F6` is **not** a raw collision word: the writer receives `$F6`;
its high byte supplies a scheduling count after shifting right twice. COP44
`$80:949B` calls `$8D:8DF8`, which writes one live first-layer word:

```text
LE16[$7E:A000 + coordinate_byte_offset] =
    tile_id | ((attribute[$7F:0000 + tile_id] & $7F) << 9)
```

It reconstructs material bits, then queues the replacement's four visual words
from `$7E:2000 + tile_id*8`, with per-column clipping. Offscreen collision still
changes. The extracted body/wrapper/writer contain no event assignment or
persistent patch-record append. Source/observed replacement attributes are
`F6:$0E`, `F7:$00`. The final recovery does not prove a reset of `$096A/$096C`.

Already-qualified F common-sheet resources independently decode to the exact C
native attribute and metatile tables before and after interaction. This resource
comparison **does not** substitute for qualifying C's full background recipe:

| Resource | Headerless half-open range | Encoded SHA-256 |
|---|---|---|
| Metatile definitions | `$2ABA43..2AC506` | `79ff0b3911c4128622d615b17d4df2363d403b3099966f6c313ee5d5c20ddea5` |
| Attributes | `$30FF53..30FFFC` | `b676ce4175b5afedbbec2f0eb5214e2314b19030796ed99e7b85b6471a124f6a` |

Decoded SHA-256: definitions
`ad57ffc7e313e0c5436c8372542845da994bf965b88195d86b068ca2f0c3f4da`;
attributes `f4a836fc55450d600becc83b3a8b5836de0983ef2fa63f30e8deeea262d98ede`.
The JP ROM is `f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548`.
All handler/dispatch/placement source windows and hashes are pinned in
`tools/house-navigation-qualification/reference.json`.

## Ordered exits and endpoint policy

Selection preserves native **first coarse match, then its fine test**; a failed
fine test does not fall through to a later exit. Handoff coordinates below are
player anchors, not raw exit origins or actor-record spawns. Queue placement uses
the signed selector adjustment at `$8D:8985 + selector*4`, then player `(8,16)`.
The core uses immutable doorway specs, not a generic native VM.

| Source record / edge | Native handoff | Destination anchor | Settled endpoint |
|---|---|---|---|
| `$818E3C` F→10 | `(392,208)`; old209 retained | `(392,336)` | `(392,353)` |
| `$818E55` 10→F, prior qualification | `(392,336)` | `(392,208)` | `(392,191)` |
| `$818E61` 10→C | `(280,416)` | `(232,432)` | `(215,432)` |
| `$818DE5` C→B | `(136,336)` | `(120,208)` | `(120,191)` |
| `$818DC0` B→C | `(120,209)` | `(136,336)` | `(136,353)` |
| `$818DCD` C→D | `(120,464)` | `(120,608)` | `(120,625)` |
| `$818E16` D→11 | `(232,672)` | `(280,688)` | `(297,688)` |
| `$818E86` 11→D | `(280,688)` | `(232,688)` | `(215,688)` |
| `$818E0A` D→C | `(120,608)` | `(120,464)` | `(120,447)` |
| `$818DD9` C→10 | `(232,432)` | `(280,432)` | `(297,432)` |
| `$818E6D` 10→11 | `(360,480)` | `(360,592)` | `(360,609)` |
| `$818E7A` 11→10 | `(360,592)` | `(360,480)` | `(360,463)` |

All listed arrivals have source/native-qualified 17-pixel endpoints. Preview
uses **17 departure updates + one load/anchor update + 17 arrival updates**,
standing in the doorway direction throughout and discarding input. Completion
starts a fresh admission epoch. Native load stalls, repeated scheduler frames,
entry text and controller timing are deliberately not counted as walking frames.
In particular native B's Down input has two extra pre-walking scheduler frames;
the selected ordinary comparison starts at the actual zero/setup owner.

Down handoff admission permits the source entry boundary or its one-pixel
overshoot, accounting for 1/2 walking deltas; a one-cell fine rectangle still
rejects an overshoot. Horizontal admission uses the source fine Y interval at
the qualified X boundary. Only the coordinates in the table are new native
witnesses; other admitted coordinates are source-geometry semantic policy.
Unsupported A/E/122 records remain in ordered lists but have no doorway spec.

## Snapshot and atomicity

Profile **8**, schema1, **109 bytes**. The identity, tick, map, walking/fresh and
animation fields retain their previous offsets; profile7 snapshots reject.

- byte82: immutable doorway spec index, or255 for walking;
- bytes83..99: walking payload, zero while a doorway owns control;
- byte99: fresh F variant; bytes100..103: animation facing/mode/phase;
- byte103: doorway elapsed; bytes104..108: little-endian handoff X/Y;
- byte108: canonical door boolean. Elapsed/handoff bytes are zero while walking.

Restore checks source identity, dimensions/profile availability, source-ordered
exit/handoff, route/current-map/animation ownership and canonical encodings.
B and both directions of its doorway require the retained open patch, including
B→C arrival after the current map has changed. Clearing the transition marker
cannot turn its zero walking payload into an ordinary state. Walking/action
errors leave the entire state unchanged; unsupported actions never become NPC
interactions or collision walls.

## Reproduction and remaining integration gate

```sh
cargo test -p room-core
cargo clippy -p room-core --all-targets -- -D warnings
cargo check -p room-core --target wasm32-unknown-unknown
sh tools/house-navigation-qualification/replay.sh
cargo run -p map-inspector -- verify-house 'local/Tenchi Souzou (Japan).sfc' semantic-preview
```

The runner creates **four independent `Session::new` empty-SRAM boots**, using
`tools/new-game-qualification/bootstrap.rs`: two census itineraries plus two
finite additional C→10→11→10 itineraries. No warp, restore, RAM patch, SRAM or
`save_state` is used. Complete post-bootstrap frame traces and checkpoints stay
under ignored `local/`; only source metadata and hashes are tracked. Normal and
optimized Python checkers require twin equality and committed pins, validate
source queue anchors/endpoints, the exact two-cell/eight-VRAM-word door change,
unchanged global events and shared-sheet persistence.

The optional core fixture test compares **374 ordinary-owned frames in 11
segments** from B,C,D,10,11. Native grids are used only as authenticated collision
qualification fixtures, never fed to `GameData` or used as production initializers.
The full runner passes at `local/house-navigation-qualification/replay-KsNzts`;
the pre-instrumentation census also agrees with the supplied read-only sibling
capture. Core/checker changes followed red→green tests, including the review regression.
Native research instrumentation necessarily preceded its evidence assertions.
Core synthetic tests cover all internal exit selections, transitions/snapshot continuation, action
latency cancellation, atomic errors and invalid snapshot ownership. Independent
review caught and fixed a B→C-arrival erased-door-bit restore hole before commit.

The CPU-free runner compiles the sibling's `profiles.json` against decoded ROM
layers/attributes, checks base and per-profile grid hashes, supplies complete
source `ExitList` records, and reuses `crates/map-inspector/src/new_game.rs` for
the authenticated shared New Game source projection. Source stamps are applied
to compiler-owned vectors before constructing immutable rooms; they are not
runtime patches or native captures. `core-route.jsonl` is finite, uses host
commands0..5 (0 neutral,1 Left,2 Right,3 Up,4 Down,5 Interact), and pins independently
witnessed native settled/door endpoints while keeping logical timing explicit.
The route finishes at tick2244, map10 `(360,463)`, door open. Core data identity
also binds the source profile contract, policy, exits, fresh events and anchor.

**Parent integration remains separate:** production command5/state JSON/rendering
and the production source-profile compiler belong to the parent; background and
NPC assets remain in their respective owners' scopes. No tracker is changed or
issue archived by this core/qualification handoff.
