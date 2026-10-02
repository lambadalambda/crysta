# Use the rope's native collision tables

## Summary

Collision type 18, tower 4's rope, is Open in all directions (`room-core/src/room.rs`), a first cut. Natively it is Open in most of the sixteen directional tables and Solid in the rest.

## Dependencies

- [Play tower 4](tower-four.md)

## Requirements

- Give type 18 its per-direction handlers.

## Acceptance Criteria

- A test walks Ark into the rope from each direction and gets the native result.

## Notes

- Research: `docs/tower-four.md` §2.
