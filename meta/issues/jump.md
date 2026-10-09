# Jump with B

## Summary

Ark's jump (B) is not modelled: `Presses` has no jump, and Ark is never in the air. Research: `docs/jump.md`.

## Dependencies

- [Make the whole underworld playable](underworld-playable.md)

## Requirements

- The ground jump from standing or walking: crouch, the height stream, the poses, the steering, the sounds, what it crosses, the landing's ground test (`docs/jump.md`).
- The dash jump, the carry jump and the rope jump: [jump-variants](jump-variants.md).

## Acceptance Criteria

- B jumps Ark from standing or walking as natively, on both ROMs: the frames, heights, poses, sounds and moves of `docs/jump.md`, and he clears a 2-cell pit.

## Notes

- The first draft said the jump starts at `$84:9252` and clears `+$04` bit `$10`; both were wrong (the thrust, and "not attacking").

## Progress

- Done (`world/jump.rs`, `b_jumps_ark_over_a_two_cell_pit`): the frames, heights, poses, steering, sounds and the pit of the native trace `p_24` match on both ROMs. Hits pass under him while h <= -16; a hit ends the jump.
- Left to [jump-variants](jump-variants.md): the dash, carry and rope jumps, the shadow (`$84:A917`), the jump attack. B pressed again in the air re-jumps at the landing even when released (`Presses` has no held B).
