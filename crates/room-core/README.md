# room-core

A small `no_std` + `alloc` component with **zero runtime or dev dependencies**.
It implements the smallest profile from
[the movement qualification](../../docs/movement-qualification.md), not a complete
player controller or collision engine.

- `Room` owns an immutable row-major `Vec<u16>` of raw collision cells.
- `WalkingState` owns position, delayed/active cardinal input, cadence phase and
  last activation direction and onset countdown. No emulator stream pointers are required.
- `step` is atomic: any `Unqualified` error leaves the entire component unchanged.
- `MovementOutput` distinguishes actual `dx/dy` from attempted stream deltas and
  reports solid-wall blocking. A rejected operation is **not** a blocked frame.
- Snapshots are fixed 16-byte versioned LE components. `decode_snapshot` checks
  representation/history invariants and player bounds against the supplied room.
  The parent authenticates room/data identity and simulation policy separately.

Unflagged types0/2/22 (open),12/14 (solid), and16 (partial) are admitted. Old samples
conservatively reject unsupported materials. Mixed open/solid edges preserve the
reference perpendicular corner nudge while retaining main-axis snap/rollback.
Ordinary direction reactivation is supported by the measured 11-tick onset
window; accelerated triggers fail atomically. Dash execution is not implemented. Unknown cells may
exist elsewhere in the grid; `Room::new` validates shape, not all materials.

## Caller responsibilities

Admit an ordinary walking checkpoint, preserve history across frames, and stop
or hand off on unsupported modes. Do not reconstruct `WalkingState` each frame
to bypass the history guard. Submit input before each step and select exits
**after** the resolved step. At the qualified F doorway, completed frame 1681
at `(392,209)` belongs to walking; the next frame belongs to the parent's
transition logic. The component intentionally has no exit or map-switch code.

Raw collision bit `$8000` is preserved. `Room::new` rejects flagged samples;
`Room::new_passive` explicitly opts into their class3 geometry under the
action-free `$0980 & $0050 == 0` contract. Neither constructor derives dynamic
events or implements collision action hooks. Actors,
interaction effects, arbitrary idle histories, camera, graphics, clocks, devices,
original CPU execution and top-level game/snapshot identity are outside this API.

## Verification

```sh
cargo test -p room-core
cargo clippy -p room-core --all-targets -- -D warnings
cargo build -p room-core --lib
cargo build -p room-core --lib --target wasm32-unknown-unknown
```

Synthetic tests are ROM-free. The optional `local_trajectories` integration test
uses existing ignored `local/movement` artifacts, authenticated by pinned CSV,
full-WRAM and extracted collision-grid SHA-256 values. It matches all **1,971**
qualified-profile position/stream steps, including doorway handoff, and repeats each
step through a restored walking snapshot. No capture bytes are committed.

If the default fixture directory is absent the optional test skips and emits a
message (visible with `--nocapture` or `--show-output`); a partially present
directory is an error. To require a specific complete set:

```sh
ROOM_CORE_FIXTURES=/path/to/local/movement \
  cargo test -p room-core --test local_trajectories -- --nocapture
```

The native integration tests invoke the host `shasum -a 256` or `sha256sum`
command (also checked against the standard `abc` digest). Missing hashing tooling
with fixtures present is an error, not an unauthenticated fallback. This avoids
adding even dev dependencies; it is not part of the native/Wasm library boundary.
Artifacts can be regenerated with `tools/movement-qualification/replay.sh` using
the independently authenticated private ROM/SRAM inputs.

## Semantic slice integration

`slice::{GameData, NewGameData, GameState, FrameOutput, Policy}` adds source-bound snapshots,
ordered exit selection and an explicitly opted-in `SemanticPreview` doorway.
Its 17/load/17 logical-update policy preserves qualified endpoints, not native
video-frame timing. It does not execute COP services or a CPU. See
[the complete preview boundary](../../docs/portable-room.md); the native local
host and browser frontend live in `map-inspector`, never in this crate.

## Profile v6

The original flat-only profile has been extended to decoded open/solid corner
responses. Walking-component snapshot version 3 and slice profile version 6
reject old semantics: walking remains16-byte v3; slice is100-byte profile6.
The original twelve local
trajectories now include positive/negative perpendicular nudges observed through
fresh input-only boots. See [house movement progress](../../docs/house-movement.md).
P16 and passive flags now have [separate qualification](../../docs/house-materials.md);
dash execution and interaction-hook effects remain unsupported.

[Admission qualification](../../docs/input-admission.md) adds 42 authenticated
fixture pairs with 3,994 ordinary transitions and 19 atomic trigger rejections,
including a 209-step revisit route and per-step snapshot restoration.

`GameState::new_game` consumes the source-compiled `NewGameData`, not a saved
checkpoint. Fresh bedroom overlays and reloaded bedroom overlays are distinct;
profile6 snapshots were100 bytes and preserved that selection until first map load.
The fresh handoff208 and saved handoff209 retain their separate departure
endpoints. [House verification](../../docs/playable-house.md) covers441 fresh
walking steps plus70 semantic updates through both rooms and repeated inputs.

## Historical house profile v8

`slice::HouseRoom` and `GameData::new_house` admit source-compiled B,C,D,F,10,11
profiles. `GameState::interact` is a one-shot atomic final wooden-door action,
not a general interaction hook. The retained `wooden_door_open()` state selects
the matching collision/visual patch; no NPC conversation or event0026 is granted.
Slice snapshots were **109 bytes, profile8**; walking snapshots remain16-byte
v3. Legacy F/10 constructors, both208/209 handoffs, directional animation and the
511-step route remain supported. See the complete
[house navigation contract and reproduction](../../docs/house-navigation.md),
including the 2,244-step source-compiled six-room replay and closed boundaries.

## Current profile v9: B conversation and bounded exterior

`conversation::{ConversationPages, ConversationSpec}` takes thirteen opaque,
non-aliased page keys in the exact source layout: one first acknowledged page,
two retained choice contexts, two three-page first follow-ups and two two-page
repeat follow-ups. It compiles six fixed source-identified requests; it does not
accept arbitrary event graphs or decode font/script bytes. Page key zero is valid.
An adapter may use `(request << 4) | page_index` as a **logical ID**, not pointer
arithmetic. The caller must authenticate the ordered keys and actual text resources.

```rust,ignore
let pages = ConversationPages {
    first: key(0x888ff0, 0),
    first_choice: key(0x888ff0, 1),
    repeat_choice: key(0x889156, 0),
    first_option1: [0, 1, 2].map(|i| key(0x8890d9, i)),
    first_option2: [0, 1, 2].map(|i| key(0x88905a, i)),
    repeat_option1: [0, 1].map(|i| key(0x88918c, i)),
    repeat_option2: [0, 1].map(|i| key(0x8891d6, i)),
};
let data = GameData::new_house(house_rooms, house_identity, startup)?
    .with_progression(ConversationSpec::new(pages)?, exterior_room, aggregate_identity)?;
```

The aggregate content identity must cover all existing house/startup inputs,
ordered request/page keys, decoded text, exterior grid and admission/semantic
policies. The extension requires unchanged ROM identity and a six-room base.
As with existing data constructors, authentication is a **caller trust boundary**;
reusing an identity for different data is not supported. No native captures,
original CPU, font runtime, wall clock, filesystem, RNG or dependency enters core.

### Control and effects

- `interact` preserves the existing C wooden-door action. With progression data,
  B at **exactly `(120,128)`, facing Up**, open wooden door, no existing owner,
  admits resident `$838B96` / callback `$888EDE`. This is an explicitly bounded
  target policy, not a guessed radial distance or full native target search.
- Initial source flags are exactly `$0020,$00FB`. First `$888FF0` starts at its
  sole acknowledged page. `acknowledge` completes that request, sets `$0026`,
  and enters **choice0 in the same atomic tick**, displaying its retained tail.
  No extra acknowledgement is invented for the tail.
- `choose(selection)` uses **0=cancel, 1/2=options**. First branches are
  `[$88905A,$8890D9,$88905A]`; repeat branches are
  `[$8891D6,$88918C,$8891D6]`. Repeat `$889156` immediately opens choice1;
  there is no acknowledged pre-choice page or further event write.
- `dialogue(&data)` returns `Option<DialogueOutput { request, cursor, wait }>`.
  Wait is `DialogueWait::Page(key)` or `Choice { catalog, key }`; catalog **0 is
  valid**. Cursor is the event-operation offset, not the native text cursor.
  `FrameOutput` keeps existing fields; `Phase::Dialogue` identifies its owner.
- `interact`, `acknowledge`, `choose`, and `step` are each one atomic logical
  tick on success. Wrong targets, owner/action mismatch, invalid results, wrong
  data and overflow leave all state unchanged. Acknowledging a choice or
  choosing a page rejects; no default option is silently selected.
- During dialogue, `step` discards directions and keeps canonical standing-Up
  animation plus empty walking history. A successful action never advances
  ordinary movement. Completion leaves the same history reset; a subsequent
  movement command starts a new admission epoch. This is semantic policy, not
  native pose/scheduler fidelity. `new_game` resets flags, owners and profiles.
- Entry greeting `$888FDA` is not part of this progression graph and never
  grants `$0026`. Parent presentation may retain its separate bounded policy.

### D reload and A admission

The constructor derives a private open-D clone by changing **only cell1415**
from `$8592` to `$0592`; it rejects a base D without that exact closed gate.
The state selects this clone only when a transition loads D with `$0026` set.
`current_room(&data)` returns the actually loaded variant. A hypothetical flag
change in already-loaded D does not remove its gate; there is no public arbitrary
flag setter. The visible frozen resident and all other occupancy remain intact.

The D→A route is enabled only by progression data and the loaded-open D profile.
It accepts source handoff **`(120,720)` or `(120,721)`**, then uses 17 departure
updates, one load update at **`(504,752)`**, and 17 arrival updates to
**`(504,769)`**. Input is discarded while transition owns control. This is the
same semantic pacing as existing house doorways, not native loader frame counts.

A's full collision sheet is **64×80 cells**. Only the 42-cell half-open sample
halo **`[29,47,36,53]`** is admitted: the constructor requires unflagged open
material0/22 in that halo and installs `Room::with_sample_halo` there. Every
**actual old/new edge sample, including perpendicular neighbor samples**, rejects
outside admission before material dispatch with `SampleOutsideAdmission`.
The boundary is not a solid wall; the whole attempted tick remains unchanged.
Other source cells/occupancy are retained but not admitted. Map bounds and
camera bounds are independent; core does not implement a camera.

No A exits or NPCs are supplied. In particular, source A→D at Ark `(504,752)`
is beyond ordinary northern sample admission, so walking cannot silently enter
an unqualified return doorway. This profile admits landing and Down/Right
walking, **not the town**. Parent owns exterior art/camera and frozen actor policy.

### Snapshot layout and verification

Slice snapshots are **181 bytes, profile9**; walking component remains16-byte v3.
Old slice profiles are rejected. Offsets through108 retain their prior meaning.

| Half-open byte range | Field |
|---|---|
| 109..173 | Source-compatible 64-byte event flags |
| 173..174 | Loaded-open D profile bit |
| 174..178 | Active source request, u32 LE; zero means no dialogue |
| 178..180 | Canonical event cursor, u16 LE; zero when inactive |
| 180..181 | Progression-data capability bit |

Walking bytes83..99 are all zero while **either** transition or dialogue owns
control; erasing a dialogue marker cannot turn its payload into valid walking.
Restore validates request membership, real wait cursors (not effect/completed
positions), reached flags, owner/map/facing/history coherence, capability and
aggregate identity. A retained D arrival additionally proves the just-selected
load profile must match its flags; an arbitrary old loaded-D state is not used
to weaken this retained-transition check. Snapshots validate canonical state,
not cryptographic input-history provenance: replacing one entire valid state
with another valid state is not corruption detection's remit.

Tests cover all nine first/repeat result combinations; no early/deferred grant;
page/choice action separation; atomic data/target/overflow failures; input locks;
all 512 single-bit pre-grant flag corruptions; erased owner and invalid request/
cursor/order mutations; reset; D load versus already-loaded lifetime; every
retained pre/post-grant D arrival; exterior transition and all halo boundaries.
A reviewer-found load-bit restore bug was reproduced red at arrival elapsed18,
then fixed and tested through elapsed34.

Additionally, a CPU-free local adapter (based on the existing
`tools/house-navigation-qualification/core_probe.rs`) compiled **real ROM assets**
with the delivered text/exterior decoders and checked restored snapshots at every
tick: legacy511 ended F `(392,191)`; the new1701-step route completed first option1,
repeat cancel, D reload, A landing `(504,769)`, Down-settled `(504,815)` and
Right-settled `(538,815)`. This used no additional oracle boot. Generated adapter,
route and logs remain ignored under `local/core-progression/`; parent host
integration supplies the permanent source compiler and UI reproduction path.
