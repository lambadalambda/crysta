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
