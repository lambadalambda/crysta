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
