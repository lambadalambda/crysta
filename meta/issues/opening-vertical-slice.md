# Complete the Crysta and Pandora vertical slice

## Summary

Integrate the portable systems into a playable opening that progresses from a new game to the first tower transition.

## Dependencies

- [Port input, player movement, and animation](port-player-input-movement.md)
- [Port map loading, transitions, and collision](port-map-loading-collision.md)
- [Port the actor system and combat primitives](port-actors-combat.md)
- [Implement the portable event runtime](portable-event-runtime.md)
- [Build the reproducible local asset pack](local-asset-pack.md)

## Requirements

- Support required rooms, interactions, dialogue events, flags, player actions, and transitions.
- Drive the slice entirely through the portable core and extracted assets.
- Add a deterministic end-to-end replay.
- Document known fidelity gaps explicitly.

## Acceptance Criteria

- The replay reaches the first tower transition without reference CPU execution.
- Covered semantic state matches reference checkpoints or approved documented exclusions.
- The slice runs under a minimal native test frontend and a headless test.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice)
- Visual and audio completeness are not required until M5, but semantic commands must be inspectable.

- [Native route evidence is complete](qualify-tower-approach-route.md): one fresh
  input-only boot reaches first-tower interior map101, with two-axis control and
  stability at frame65608 `(112,607)`. This is not portable implementation.
  [The bounded integration scope](../../tools/tower-approach-qualification/TOWER.md#bounded-portable-follow-up)
  covers source assets, frozen-state actors/requests, world-map motion and tower
  presentation/arrival. Item81 is acquired but unequipped; combat is not a hard
  prerequisite for this entrance-only slice. Actor/presentation work remains
  required, and broader combat stays in its separate dependency.
