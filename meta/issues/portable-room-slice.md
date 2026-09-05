# Implement a reference-qualified portable room slice

## Summary

Reproduce bounded player movement, collision and the qualified room transition
in a minimal deterministic portable simulation over decoded assets.

## Dependencies

- [Qualify an opening room transition](qualify-opening-room-transition.md)
- [Define the deterministic portable core model](deterministic-core-model.md)

## Requirements

- Trace measured player movement and collision decisions in the selected rooms;
  do not promote community collision labels into verified rules.
- Introduce only the core state/input/data/output boundaries required by the
  slice, with explicit integer behavior and no device, clock, filesystem or
  original-CPU dependency.
- Use synthetic red-green tests plus authenticated reference replay fixtures.
- Keep unqualified actions, slopes and event behavior explicit rather than
  silently treating them as supported.

## Acceptance Criteria

- Deterministic portable replays match selected reference positions, blocked
  movement and before/after room-transition state.
- Synthetic tests cover movement bounds, collisions and repeatable snapshots.
- A minimal local frontend or inspector demonstrates walking and leaving the
  selected room without original CPU execution in the simulation loop.
- Native and WebAssembly builds validate the portable boundary when the target
  toolchain is available; missing tooling or evidence is reported explicitly.

## Notes

- Parents: [player movement](port-player-input-movement.md),
  [map loading and collision](port-map-loading-collision.md).
- This is a bounded room slice, not the full opening or an early general engine.
- [Experimental movement qualification](../../docs/movement-qualification.md)
  measures fresh-bootstrap directional latency/cadence and all four actual-player
  flat collision paths. Nine complete movement segments match 1,771 position
  steps; flagged cells, type16 and dash diagnostics are explicitly not promoted
  to support. No production implementation in that work item; raw traces and
  per-frame reference fixtures remain ignored under `local/movement/`.
- A smaller fail-closed floor/full-wall profile now matches 1,204 steps,
  including all 80 walking steps to the parent's doorway handoff at completed
  1681 `(392,209)`. It rejects mixed open/solid pairs and repeated directional
  activations; corner nudges and dash cooldown reconstruction are not required.
  Completed 1682 is explicitly rejected as transition-controlled, not walking.

## Implemented bounded profile and verification

- Added dependency-free `no_std` + `alloc` room-core, immutable grids/data,
  deterministic input/output, atomic errors and versioned source-bound snapshots.
- 1,204 authenticated walking steps match reference cadence, positions, blocking
  and restored replays. Unknown/flagged cells, mixed pairs, boundary arithmetic
  and conservative direction-reactivation admission fail explicitly.
- The exact selected doorway handoff and decoded queue/spawn path are used by
  an **opt-in endpoint-qualified SemanticPreview policy**. Its 17/load/17 logical
  pacing does not reproduce native video-frame scheduling or loader stalls.
  This is the completed minimal preview profile, not classic transition fidelity.
- Native loopback browser host demonstrates walking and leaving room F for 10
  without original CPU execution. Static BG1 plus a bounds marker; no sprites,
  actors, audio, combat or events beyond the single semantic doorway.
- Synthetic/core/HTTP/UI tests, authenticated replay/ROM-grid equality, repeated
  CPU-free CLI results, desktop/mobile browser QA, native and Wasm builds,
  workspace gates and independent reviews passed.
- See [portable room documentation](../../docs/portable-room.md). General
  controller cadence, dash, corner/event behavior and direct Wasm browser hosting
  remain outside this completed slice; the parent issues remain open.
