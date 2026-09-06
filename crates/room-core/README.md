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

## Current house profile v8

`slice::HouseRoom` and `GameData::new_house` admit source-compiled B,C,D,F,10,11
profiles. `GameState::interact` is a one-shot atomic final wooden-door action,
not a general interaction hook. The retained `wooden_door_open()` state selects
the matching collision/visual patch; no NPC conversation or event0026 is granted.
Slice snapshots are now **109 bytes, profile8**; walking snapshots remain16-byte
v3. Legacy F/10 constructors, both208/209 handoffs, directional animation and the
511-step route remain supported. See the complete
[house navigation contract and reproduction](../../docs/house-navigation.md),
including the 2,244-step source-compiled six-room replay and closed boundaries.
