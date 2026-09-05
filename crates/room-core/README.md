# room-core

A small `no_std` + `alloc` component with **zero runtime or dev dependencies**.
It implements the smallest profile from
[the movement qualification](../../docs/movement-qualification.md), not a complete
player controller or collision engine.

- `Room` owns an immutable row-major `Vec<u16>` of raw collision cells.
- `WalkingState` owns position, delayed/active cardinal input, cadence phase and
  a conservative used-directions mask. No emulator stream pointers are required.
- `step` is atomic: any `Unqualified` error leaves the entire component unchanged.
- `MovementOutput` distinguishes actual `dx/dy` from attempted stream deltas and
  reports solid-wall blocking. A rejected operation is **not** a blocked frame.
- Snapshots are fixed 16-byte versioned LE components. `decode_snapshot` checks
  representation/history invariants and player bounds against the supplied room.
  The parent authenticates room/data identity and simulation policy separately.

Only unflagged types 0/2/22 (open) and 12/14 (solid) are admitted. Old samples
conservatively reject unsupported materials; mixed new open/solid sample pairs
stop rather than implementing corner nudges. Direction reactivation is rejected,
not treated as ordinary walking or an emulated dash cooldown. Unknown cells may
exist elsewhere in the grid; `Room::new` validates shape, not all materials.

## Caller responsibilities

Admit an ordinary walking checkpoint, preserve history across frames, and stop
or hand off on unsupported modes. Do not reconstruct `WalkingState` each frame
to bypass the history guard. Submit input before each step and select exits
**after** the resolved step. At the qualified F doorway, completed frame 1681
at `(392,209)` belongs to walking; the next frame belongs to the parent's
transition logic. The component intentionally has no exit or map-switch code.

Raw collision bit `$8000` is preserved and rejected. Adapters may explicitly
mark reference-checkpoint guard cells; the component does not derive dynamic
collision changes from static graphics or silently clear those flags. Actors,
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
full-WRAM and extracted collision-grid SHA-256 values. It matches all **1,204**
strict-profile position/stream steps, including doorway handoff, and repeats each
step through a restored walking snapshot. No capture bytes are committed.

If the default fixture directory is absent the optional test skips and emits a
message (visible with `--nocapture` or `--show-output`); a partially present
directory is an error. To require a specific complete set:

```sh
ROOM_CORE_FIXTURES=/path/to/local/movement \
  cargo test -p room-core --test local_trajectories -- --nocapture
```

Only this native integration test invokes the host `shasum -a 256` or `sha256sum`
command (also checked against the standard `abc` digest). Missing hashing tooling
with fixtures present is an error, not an unauthenticated fallback. This avoids
adding even dev dependencies; it is not part of the native/Wasm library boundary.
Artifacts can be regenerated with `tools/movement-qualification/replay.sh` using
the independently authenticated private ROM/SRAM inputs.
