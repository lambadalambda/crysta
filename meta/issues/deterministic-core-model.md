# Define the deterministic portable core model

## Summary

Create the platform-independent simulation API, state ownership, fixed-width numeric policy, and command outputs.

## Dependencies

- [Bootstrap the Rust workspace and quality gates](bootstrap-rust-workspace.md)
- [Define the ROM revision and version-support model](define-version-support-model.md)
- [Create initial reference replay scenarios](initial-reference-scenarios.md)

## Requirements

- Define immutable/versioned `GameData`, mutable `GameState`, deterministic `FrameInput`, and `FrameOutput` boundaries.
- Include source revision and asset-schema compatibility in snapshots without serializing immutable game content.
- Define how deterministic host-service responses enter `FrameInput` or explicit lifecycle APIs.
- Version snapshots and deterministic RNG state.
- Keep rendering, audio devices, wall clocks, and filesystems outside the core.
- Specify classic and enhanced feature boundaries.

## Acceptance Criteria

- A synthetic replay produces the same serialized state hash across repeated runs.
- The core builds for native and `wasm32-unknown-unknown` targets.
- Architecture tests prevent platform dependencies from entering the core.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice)
- Prefer simple data-oriented structures that mirror verified behavior before considering a generalized engine or ECS.

## Bounded groundwork

`room-core::slice` now supplies source/data-bound snapshots, deterministic
GameData/GameState/input/output, explicit SemanticPreview policy and a no-std
native/Wasm boundary for the small [room preview](../../docs/portable-room.md).
Snapshot RNG-policy version 0 means no RNG is used in that subset. A general
reference RNG/state model, broader commands and future host services are not
implemented; this parent issue remains open.
