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
    contacts,          // [resident, first box contact/recoil witness]
    opening_gate,      // BoxOpeningGate { raw_bounds: [120,368,152,400] }
    source_pots,       // sorted Vec<pots::SourceObject>, maximum64
    cellar_up_lanes,   // explicit compiler qualification assertion
)?;
let data = old_house_data.with_pandora(pandora, aggregate_identity)?;
let game = GameState::new_game(&data, Policy::SemanticPreview);
```

`with_pandora` requires the existing B/exterior capability, unchanged ROM identity
and a new aggregate content identity. The compiler must authenticate/hash **all**
old house, new text, raw collision/occupancy/material policies, pot, motion, contact and phase
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

### Raw material policy

`room_core::{MaterialRule, MaterialAlias, MaterialPolicyError}` supplies the finite
classification seam. Install after the profile halo, without modifying source words:

```rust,ignore
let room = raw_room.with_material_policy(vec![MaterialRule {
    bounds: [6, 53, 7, 54], // half-open cell scope, E stair only
    direction: Some(Direction::Up),
    alias: MaterialAlias::StairOpen29,
}])?;
```

- `TownSolid25`: type25 as solid, scoped to authenticated Town bounds; direction
  may be `None` (all directions) or one cardinal.
- `ClosedDoorPartial5`: type5 as partial only Up at `(11,21)`.
- `StairOpen29`: type29 as open only Up at `(11,21)`, `(6,53)`, or `(22,53)`.
  The compiler binds these to C/E/20 respectively, never a global floor alias.

Room validates finite scopes, grid/halo extent and same-type/direction overlap.
`material_policy()` exposes immutable rules for hashing; policies survive clones
and genuine tile patches. Rules need not match the current raw word, since closed
and open door phases share a policy. The collision solver uses its **delayed resolve
direction**, not newly submitted input. Stored old-edge slopes6/7 reject before
bit15; the existing passive assertion then classifies bit15 as solid before aliases.
Other unsupported types and samples outside the halo still fail atomically.
Default rooms have no aliases. The host authenticates map membership, source table
semantics, policy and full raw grid in the aggregate content identity.

Standalone pot lane admission retains its existing exact-word contract but uses
this same classifier internally rather than normalizing `0B81`/`3ACB` in a private
grid. Actual consumed-pot/door patch events remain distinct from classification.

### Collision, contact and motion

`CollisionKey::ALL` selects fourteen immutable variants. Dimensions are source
layer dimensions: A64×80, map13 64×32, C/E/20 32×64, box21 16×32, tour41–44 32×32
cells. Every profile requires a qualified sample halo. C's four variants retain
the same pot source catalog and exact closed/damaged/temporary/opened door words.
The host owns source resident/table occupancy; an open synthetic test room is
**not** production admission.

`ContactSpec { kind, trigger: Anchor, result: Anchor }` admits exact callback
anchors. `Anchor` contains player coordinates and facing. The map13 resident uses
Interact. The first box callback retains an exact compiler-qualified witness:
raw `(136,370)` Down, with the admitted recoil endpoint `(136,359)`. The source
first-contact rectangle is X123..149/Y370..400, but core does not broaden walking,
poses or recoil admission to that whole rectangle.

Opening is **not a second callback or Down edge**. `BoxOpeningGate` supplies the
inclusive raw-coordinate rectangle (source X120..152/Y368..400). Core polls it with
local1 AND local2, independently of facing and neutral/held input. Raw Y359 remains
outside. A successful predicate takes masked control in `Cue::BoxAcquireControl`
without granting22. Only its compiler-qualified completion certifies successful
COPDF `$888EA6` (`$097C & $0810 == 0`); it then grants22 and starts `BoxReload`.
The compiler must not emit that completion merely because proximity passed or a
guessed timer elapsed. Missing readiness motion data fail atomically while masked,
without grant/reload. COPC1's32 actor-delay units are not assumed to be32 video frames.
See main's corrected `docs/pandora-navigation.md` source contract (e98a7cc).

`MotionKey` is either a fixed `Travel` or a currently owned `Cue`:

- New ordinary routes: A→13, 13→A, A→D, C→E, E→20, 20→21. Existing house routes
  remain the old qualified transitions, including D→A and return to changed C.
- Forced loads: box21→21, 21→41, then 41→44→42→43→41.
- `Cue::Returned(Invocation)` identifies post-request presentation, not another
  page acknowledgement. Second-hit patching and cooperating color-worker
  boundaries have separate cue identities.

`MotionSpec { key, trigger, frames }` holds at most4096 logical samples; ordinary
exits require exact triggers. Ambiguous same-map/same-anchor travel pairs are
rejected. `MotionFrame { map_id, pose, reload, scene }` carries compiler-qualified
samples. Load motions contain exactly one explicit reload marker, even for 21→21.
Map membership and pose bounds are checked. Missing cue motions reject the whole
action atomically; there are **no fallback timers, inferred stairs or teleports**.
The compiler must constrain its halo/collision to admitted movement and exits;
this is not a general town/navigation engine. Exact `MotionSpec` anchors do not
represent the navigation compiler's complete ordered coarse/fine exit tables.
Ordered-exit/Town admission is being corrected separately; the raw classification
seam below resolves the previous Up-only type29 limitation.

The last motion sample is an explicit **completion boundary**: its pose and reload
apply, then the graph continuation and next scene win in the same logical update.
Only preceding samples expose their supplied `scene`. A one-sample motion therefore
performs its endpoint/continuation atomically; it does not display a separate
terminal actor frame. This policy is tested and adds no unqualified trailing wait.
These logical samples are not a claim of native scheduler/video-frame fidelity.

`MotionFrame.pose` is `MotionPose::Absolute(Anchor)` or
`MotionPose::Preserve { facing: Option<Direction> }`. Preserve is allowed only for
non-reloading cue samples; it retains the active player coordinates, and `None`
also retains facing. This is Ark's pose, never a moving resident/guide's position.
For `BoxAcquireControl`, the compiler can supply Preserve with stationary Down on
successful completion without teleporting from the rectangular, any-facing gate.
The source-qualified completed-recoil result clears `$097C`; only the admitted
ordinary grounded walking/neutral and warning subset preserves that readiness
certificate. The compiler authenticates this closed-subset assertion; core does
not infer readiness from elapsed time, proximity, or the installed Down facing.
Missing readiness data still fail closed. This API/schema change requires a new
aggregate content identity; profile11 snapshots are intentionally not accepted.

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
actual `door_counter`, plus `sheet: SharedSheetOutput { resident, cellar, consumed }`.
`cellar` is `CellarDoorPatch::{Closed, Damaged, Open}`; `consumed` is a source-catalog
`u64` bitset. `wooden_door_open()` remains the sole wooden-door boolean.
No new map delegates to missing house art. Host must use
`scene.key()` rather than derive phase from flags. No interpolated actor positions
or inventory/equipment output are emitted. Art composition, record scheduling and
transport remain parent-owned.

Legacy `FrameOutput::phase` keeps its enum shape; its Dialogue variant is a
non-walking bucket under this new opt-in capability. Use `PandoraOutput::owner` to
distinguish visible dialogue, presentation, transitions and pot recovery. A pot
can continue flying/recovering while a story request is visible or its cue advances.

## Flags, graph and resident-sheet pots

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
while active. When absent, the finite resident-sheet state parks that ledger;
there is never a second mutable ledger authority. Consumption and A attempts never
increment hits. Only exact `door_hit` output advances the per-load counter. The
launch collision key remains frozen through recovery, independently of door patches
and `$292`; concurrent story presentation cannot invalidate the launch lane.
Forced empty-handed motion rebases the walker without losing consumed cells.

AFCBB3 tile/attribute patches survive B/C/D/E/20 loads (also F/10/11, whose common
source load operand was verified against the ROM). Removed pots and damaged/open
cellar and wooden doors survive; locals, counter, action state and scene occupancy
do not. A/13/21 replace the sheet without an off-screen mutation cache. A→D rebuilds
the wooden door closed even with persistent `$26`; idle empty-handed pot ownership
permits reopening it without losing the sheet ledger. All four supplied C profiles
must contain source-closed wooden cells `(8,19)=$1CF2`, `(8,20)=$1CF3`.

`current_room()` remains the borrowed immutable base view. Use
`effective_room(&data) -> Result<Room, SliceError>` for owned scene geometry with
retained patches; Pandora movement uses this effective view. Patching preserves the
**current scene's** occupancy bit rather than copying departed actors. In particular,
C's departed `(7,31)` actor stamp is not retained in E/20. `consumed_pots()` remains
available without an active pot controller. Visit baselines distinguish old sheet
mutations from new hits after counter reset.

After sheet replacement, persistent `$292` alone does not authorize reapplying open
cellar tiles. A→D with `$292` restores on a new closed sheet, but subsequent D→C
reconstruction fails atomically until a source-qualified load effect is supplied.

The warning requires two acknowledgements, its qualified return/delay cue, then a
facing-independent proximity poll and successful COPDF handoff. Opening sets `$22` before reload. The mandatory tour owns control
across blank-text gaps and all forced loads. `$243` is granted on the final return;
`$244` only after the final four-page request returns. The endpoint is controllable
map41, **not** equipment acquisition, free inventory-room transitions or world return.

## Canonical snapshots

Disabled data retain the exact **181-byte version1/profile9** encoding. Opt-in data
use **320-byte version4/profile12**, bound to the same aggregate identity and tick:

| Byte range | Meaning |
|---|---|
| 0..181 | Common envelope/house fields; low64 flag bytes remain at109..173 |
| 181..245 | Upper64 authoritative story-flag bytes |
| 245..249 | Fixed graph node/invocation/page and per-visit hit counter |
| 249..252 | Motion index/next-sample cursor;255/0 means none |
| 252..292 | Existing POT1 encoding, or canonical zero absence |
| 292 | Frozen launch collision key;255 means none |
| 293 | Active-motion witness (0/1), checked against motion ownership |
| 294..296 | Reserved zero |
| 296..300 | Frozen graph/motion-owned X/Y (LE u16); zero with pot/legacy ownership |
| 300 | Resident-sheet boolean |
| 301 | Cellar patch: Closed0 / Damaged1 / Open2 |
| 302 | Visit-baseline cellar patch |
| 303 | Reserved zero |
| 304..312 | Parked consumed ledger, LE u64; zero while PotState owns it |
| 312..320 | Visit consumed baseline, LE u64 |

The walking component is erased when a dialogue, transition, graph or pot owns its
continuation. Motion-owned poses use the frozen owner anchor and are checked against every
known absolute-position/facing constraint in the applied immutable prefix. Graph
wait anchors and pot poses have canonical consistency checks. Restore rejects
invalid versions/identities, stage/flag/local/counter combinations, choice/page
cursors, erased motion ownership, inappropriate launch profiles and fewer consumed
source objects than recorded hits. Sheet identity, parked/active ledger exclusivity,
visit baselines and patch/counter consistency are validated separately; persistent
flags do not reconstruct discarded mutations. The opened box collision profile is selected
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

## Opt-in native-A transport

The loopback step protocol preserves commands0–9 and adds only canonical ASCII
`10` for `GameState::pot_action`; this is separate from B/Interact and dialogue
acknowledgement. The request body ceiling is now two bytes, but other two-byte
forms (`00`, `01`, signed/whitespace forms, `11`) remain invalid. Host/Origin,
exact route, total request bound and absolute deadline checks are unchanged.

State advertises `pot_action` from the immutable Pandora capability. The frontend
shows **Lift / throw (Z)** only when enabled, rejects held-direction/dialogue/
transition use, and submits one paced A pulse without repeat or autoplay. As
with manual interaction, it pauses; **Resume** supplies neutral ticks to advance
the delayed action/recovery. Z is not WASD A and never acknowledges a page.
The current house-only host advertises false and does not enable pot gameplay.
