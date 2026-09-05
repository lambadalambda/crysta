# Qualify Ark's standing and walking animation

## Summary

Establish the facing, pose-selection and animation-timing semantics needed to display Ark while using the existing portable house walking core.

## Dependencies

- [Port input, player movement, and animation](port-player-input-movement.md)
- [Qualify repeatable house movement and collision](qualify-repeatable-house-movement.md)

## Requirements

- Capture fresh input-only standing, cardinal walking, turn, block and release sequences; separate animation ownership from movement cadence and native scheduler stalls.
- Identify source-backed frame IDs, timing and facing fields, including new-game initial facing.
- Implement a small pure deterministic state component where qualified; preserve atomic unsupported-action rejection and snapshot determinism.
- Explicitly separate semantic doorway rendering policy from native transition animation timing.

## Acceptance Criteria

- Reproducible reference sequences and source witnesses define required ordinary animation behavior.
- Synthetic red-green tests and authenticated per-step comparisons cover selected holds, turns, blocked walking and releases.
- Integration contract exposes renderable frame/facing state without introducing devices, clocks, ROM access or dependencies into the core.

## Notes

- Subissue of [render Ark in the portable house](render-ark-house-sprite.md).
- Bounded ordinary selection is implemented and independently reviewed; see [animation contract and source/native witnesses](../../docs/ark-animation.md). Six walking records last nine ticks each; turns reset, blocked holds keep animating, delayed release selects standing in the retained facing.
- `tools/player-animation-qualification/` reproduces six input-only plans twice from fresh frame 6800, with 827 real walking/animation per-step comparisons per replay set. Five separate ROM-free red/green tests cover the pure component and canonical restore parts. Raw captures remain private.
- Fresh initial facing is Down, but native frame 6800 is an idle-fidget pose. Ordinary Down standing and indefinite neutral/doorway rendering are explicit semantic policies, not claims of native idle/transition timing equality.
- Parent still owns exporting the new module, whole-state atomic rejection, snapshot/mode coherence and host/UI integration. Existing lib/state/snapshot/walking tests and tracker indexes are intentionally untouched by this bounded task.
