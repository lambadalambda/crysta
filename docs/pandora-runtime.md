# Bounded Pandora runtime

[Owned issue](../meta/issues/port-pandora-story-state.md). The live host remains
on profile9. This is an opt-in, CPU-free continuation of the **existing New Game
and GameState**, not a checkpoint-start game, inventory implementation or event VM.
The core remains dependency-free `no_std + alloc`.

## Compiler handoff

All public construction/output types are reexported by `room_core::slice`:

```rust,ignore
let text = PandoraText::new(requests)?;
let pandora = PandoraData::new(
    text,
    rooms,             // Vec<ProfileRoom>, CollisionKey::ALL order
    motions,           // Vec<MotionSpec>, missing entries fail closed
    contacts,          // [resident, first box contact, second box contact]
    source_pots,       // sorted Vec<pots::SourceObject>, maximum64
    cellar_up_lanes,   // explicit compiler qualification assertion
)?;
let data = old_house_data.with_pandora(pandora, aggregate_identity)?;
let game = GameState::new_game(&data, Policy::SemanticPreview);
```

`with_pandora` requires the existing B/exterior capability, unchanged ROM identity
and a new aggregate content identity. The compiler must authenticate/hash **all**
old house, new text, collision/occupancy/admission, pot, motion, contact and phase
inputs in canonical order. Core validates structure, not source provenance or
cryptographic hashes. No ROM pointers, captured-memory initializer, CPU execution,
filesystem or clock enter the runtime.

### Immutable text

`Invocation`, `RequestPages` and `PandoraText` validate the 33 source resources in
the assets compiler's first-use order, then map13 retry and refusal. Page keys are
opaque and globally unique across resources. Counts include non-acknowledged D4
choice contexts. The warning has **two** pages, not an invented AE50 wait.
`Invocation::ALL` preserves 34 direct sites, including four distinct D720
invocations. A resource identity is never executed as an address.

### Collision, contact and motion

`CollisionKey::ALL` selects fourteen immutable variants. Dimensions are source
layer dimensions: A64×80, map13 64×32, C/E/20 32×64, box21 16×32, tour41–44 32×32
cells. Every profile requires a qualified sample halo. C's four variants retain
the same pot source catalog and exact closed/damaged/temporary/opened door words.
The host owns source resident/table occupancy; an open synthetic test room is
**not** production admission.

`ContactSpec { kind, trigger: Anchor, result: Anchor }` admits exact callback
anchors. `Anchor` contains player coordinates and facing. The map13 resident uses
Interact; box warning/opening require collision-resolved **Down contact**, never
Interact. Box contact kinds are selected by the graph's authoritative local flags.

`MotionKey` is either a fixed `Travel` or a currently owned `Cue`:

- New ordinary routes: A→13, 13→A, A→D, C→E, E→20, 20→21. Existing house routes
  remain the old qualified transitions, including D→A and return to changed C.
- Forced loads: box21→21, 21→41, then 41→44→42→43→41.
- `Cue::Returned(Invocation)` identifies post-request presentation, not another
  page acknowledgement. Second-hit patching and cooperating color-worker
  boundaries have separate cue identities.

`MotionSpec { key, trigger, frames }` holds at most4096 logical samples; ordinary
exits require exact triggers. Ambiguous same-map/same-anchor travel pairs are
rejected. `MotionFrame { map_id, anchor, reload, scene }` carries compiler-qualified
samples. Load motions contain exactly one explicit reload marker, even for 21→21.
Map membership and pose bounds are checked. Missing cue motions reject the whole
action atomically; there are **no fallback timers, inferred stairs or teleports**.
The compiler must constrain its halo/collision to admitted movement and exits;
this is not a general town/navigation engine.

The last motion sample is an explicit **completion boundary**: its pose and reload
apply, then the graph continuation and next scene win in the same logical update.
Only preceding samples expose their supplied `scene`. A one-sample motion therefore
performs its endpoint/continuation atomically; it does not display a separate
terminal actor frame. This policy is tested and adds no unqualified trailing wait.
These logical samples are not a claim of native scheduler/video-frame fidelity.

## Host actions and presentation

Existing `step`, `interact`, `acknowledge` and `choose` remain the action seam.
`pot_action` is a distinct one-frame native A pulse—suitable for parent transport
command10—not an overload of resident Interact/B. Movement `step` advances delayed
A, lift, carrying, flight and recovery. Errors preserve the entire game and tick.
Directions during story ownership are discarded, never buffered into a later room.

```rust,ignore
let p = game.pandora_output(&data)?;
let scene_phase: Option<&str> = p.scene.key(); // exact assets33 phase names
let dialogue = game.dialogue(&data)?;        // visibility independent of ownership
let pot = game.pot_state();                  // phase/tick/facing/walking/flight()
let (hand, reservation) = game.pot_slots(&data)?; // FA=098A, FB=098F
let removed_cells = game.consumed_pots(&data)?;
```

`PandoraOutput` provides `scene`, typed `invocation`, pending `cue`, `owner`,
`motion: Option<(MotionKey, u16)>` (number of applied samples), room `locals` and
actual `door_counter`. No new map delegates to missing house art. Host must use
`scene.key()` rather than derive phase from flags. No interpolated actor positions
or inventory/equipment output are emitted. Art composition, record scheduling and
transport remain parent-owned.

Legacy `FrameOutput::phase` keeps its enum shape; its Dialogue variant is a
non-walking bucket under this new opt-in capability. Use `PandoraOutput::owner` to
distinguish visible dialogue, presentation, transitions and pot recovery. A pot
can continue flying/recovering while a story request is visible or its cue advances.

## Flags, graph and per-visit pots

There is exactly one authoritative `StoryFlags` block. The generic B conversation
runs directly on it and retains `$26` **after the first page, before the choice**.
The public EventFlags/EventSequence aliases remain unchanged. Because sole wide
storage cannot safely supply the old borrowed `&EventFlags`, `event_flags()` now
returns an **owned low64-byte inspection projection**; `story_flags()` borrows the
authority. This is an explicit Rust getter compatibility adjustment, not mirrored
mutable state. Old profile9 behavior and snapshot bytes remain unchanged.

Every reconstruction resets exactly locals0..31 plus the hit counter, including
same-map reload. Persistent highflags survive. Map13 cancellation/result2 runs two
refusal pages, sets local1, then admits retry. Only the fourth grant-page return
sets `$28`. Changed C sets `$27` before entry text returns; only result1 and the
second direct-answer return grant `$2E`. C cancel/result2 rejects atomically: no
silent direct continuation and no `$2F`.

The existing `PotState` owns movement/carry history and the consumed-cell ledger
for the entire C visit. Consumption and A attempts never increment hits. Only its
exact `door_hit` output advances the counter. The launch collision key remains
frozen through recovery, independently of first-hit/second-hit door patches and
`$292`; concurrent story presentation cannot invalidate the launch lane. Forced
empty-handed motion rebases the walker without losing consumed cells. Reconstruction
alone discards the ledger. `current_room()` returns the immutable story variant;
consumed-cell overlays are private to pot collision and exposed separately for art.

The warning requires two acknowledgements, its qualified return/delay cue, then a
second contact. Opening sets `$22` before reload. The mandatory tour owns control
across blank-text gaps and all forced loads. `$243` is granted on the final return;
`$244` only after the final four-page request returns. The endpoint is controllable
map41, **not** equipment acquisition, free inventory-room transitions or world return.

## Canonical snapshots

Disabled data retain the exact **181-byte version1/profile9** encoding. Opt-in data
use **300-byte version2/profile10**, bound to the same aggregate identity and tick:

| Byte range | Meaning |
|---|---|
| 0..181 | Common envelope/house fields; low64 flag bytes remain at109..173 |
| 181..245 | Upper64 authoritative story-flag bytes |
| 245..249 | Fixed graph node/invocation/page and per-visit hit counter |
| 249..252 | Motion index/next-sample cursor;255/0 means none |
| 252..292 | Existing POT1 encoding, or canonical zero absence |
| 292 | Frozen launch collision key;255 means none |
| 293..296 | Reserved zero |
| 296..300 | Frozen graph-owned anchor when no motion/legacy owner supplies it |

The walking component is erased when a dialogue, transition, graph or pot owns its
continuation. Motion-owned poses are reconstructed from immutable samples; graph
wait anchors and pot poses have canonical consistency checks. Restore rejects
invalid versions/identities, stage/flag/local/counter combinations, choice/page
cursors, erased motion ownership, inappropriate launch profiles and fewer consumed
source objects than recorded hits. The opened box collision profile is selected
at the actual reload sample, not delayed to motion completion. Canonical structural
restoration is **not proof of snapshot provenance** or authorization for a native
qualification restore.

## Verification and remaining acceptance

Text and aggregate API tests were introduced red then green. Synthetic tests cover
branches, atomic unsupported actions, malformed data/snapshots, source dimensions,
per-action restoration, reload/transition ownership, two actual lifts/carries/hits,
concurrent `$292`/recovery, preserved consumption and final controllable41. The
isolated miss test deliberately seeds a held launch; it is not a native carry-route
witness. Independent reviews caught and fixed ambiguous exit admission, box reload
profile timing, erased forced-motion ownership and counter/ledger inconsistency.

All five enabled private core fixture suites and the old host's authenticated
fresh-house route tests pass. Core strict all-target Clippy and no_std Wasm build
pass. These are **not** new native aggregate navigation/pacing or browser acceptance.
Source-qualified new motion/contact data, host compiler/transport/render integration
and final aggregate acceptance remain parent/navigation-owned. The capability must
remain disabled on the live host until those gates pass. The owned issue stays open.
