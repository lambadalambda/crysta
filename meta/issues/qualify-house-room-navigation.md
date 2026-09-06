# Qualify navigation through all fresh house rooms

## Summary

Extend the playable house from F/10 to fresh rooms B,C,D,F,10,11, retaining the source-gated exterior/cellar boundaries.

## Dependencies

- [Qualify the fresh house room and actor roster](qualify-house-scene-roster.md)
- [Qualify complete house background profiles](qualify-house-background-profiles.md)

## Requirements

- Qualify source exit selection, handoffs, destination anchors and settled endpoints from fresh input-only routes. Keep explicit semantic transition timing separate from native scheduling.
- Generalize the dependency-free core only as needed for these six immutable room profiles and transitions; preserve atomic rejection, animation ownership and snapshots.
- Qualify the C/B wooden-door interaction and its required collision/visual changes rather than silently opening it. Entry-dialogue omission may remain explicit semantic policy; it must not grant event0026.
- Keep D/A exterior and C/E cellar access gated. Do not implement unrelated NPC conversations, wandering or later-story events.

## Acceptance Criteria

- A CPU-free reproducible route visits the six admitted rooms and restores snapshots along the way.
- Source/native route endpoints and ordinary walking/collision segments agree; the existing511-step route remains valid.
- Synthetic interaction/transition/state tests, fresh qualification and independent review pass.

## Notes

- Subissue of [complete fresh house setup](complete-house-scene-setup.md).
