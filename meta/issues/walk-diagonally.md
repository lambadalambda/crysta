# Walk diagonally

## Summary

The runtime takes one cardinal direction a frame (`room_core::FrameInput`),
so Ark cannot walk diagonally, and the diagonal dash cannot start. The
hosts send one direction too.

## Dependencies

- [Make the whole underworld playable](underworld-playable.md)

## Requirements

- Diagonal input from the hosts to the world.
- Diagonal walking with its collision and facing, from native traces.
- Then the diagonal dash (`meta/issues/dash-diagonals-attack-jump.md`).

## Acceptance Criteria

- Diagonal walks and dashes match native frames.
