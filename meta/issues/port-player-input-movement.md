# Port input, player movement, and animation

## Summary

Reimplement Ark's basic input processing, movement state, direction, and animation timing.

## Dependencies

- [Define the deterministic portable core model](deterministic-core-model.md)
- [Build canonical RAM and hardware symbol maps](canonical-memory-symbols.md)
- [Export and compare reference frame state](reference-state-export.md)

## Requirements

- Preserve input edge/hold semantics and frame timing.
- Model fixed-point or subpixel position exactly where used.
- Cover idle, walking, running, and opening-area interactions needed by the slice.
- Compare named player state against reference replays.

## Acceptance Criteria

- Movement replay checkpoints match documented reference state.
- Unit tests cover wrap, sign, and boundary arithmetic.
- Animation frame sequences match classic timing for selected actions.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice)
- Advanced combat moves may remain in the actor/combat issue unless required by the opening slice.

## Completed bounded milestone

- [Render Ark standing and walking in the portable house](render-ark-house-sprite.md)

ROM-backed ordinary house Ark sprites and deterministic facing/walking animation are complete. This parent remains open for broader graphics/actions and classic scene behavior. See [playable scope](../../docs/playable-house.md).
