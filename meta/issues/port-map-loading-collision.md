# Port map loading, transitions, and collision

## Summary

Load decoded map content and reproduce collision and transition behavior for the opening areas.

## Dependencies

- [Define the deterministic portable core model](deterministic-core-model.md)
- [Decode map, metadata, and collision formats](decode-map-collision-formats.md)

## Requirements

- Instantiate maps from the local asset pack.
- Implement tile or region collision semantics used in selected rooms.
- Implement camera and map-transition state needed by the slice.
- Keep renderer concerns out of collision logic.

## Acceptance Criteria

- Recorded paths through selected Crysta rooms match reference position and map transitions.
- Collision tests cover walls, passable tiles, interaction boundaries, and transition edges.
- Malformed asset data fails with context.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice)
- Broader world-map behavior can be added in later chapter work.
