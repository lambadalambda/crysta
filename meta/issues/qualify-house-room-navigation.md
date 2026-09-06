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

## Completion

- Six-room immutable core and authenticated host compiler now cover the full source-ordered internal graph, preserve the prior511-step route, and retain the wooden-door patch in profile8 snapshots.
- The atomic one-shot door action has source-derived collision and complete metatile visual replacements; no entry dialogue or event0026 grant is invented. Exterior/cellar gates remain closed.
- Parent independently repeated four fresh native runs at ignored `local/house-navigation-qualification/replay-xF3xFu`, including374 ordinary-owned walking comparisons. The2,244-step CPU-free core and host routes check all six rooms,15 checkpoints, every-step restored continuation and reset.
- Core/host/visual independent reviews, synthetic atomicity/source-mutation tests, required authenticated legacy route fixtures, strict Clippy and Wasm build pass. See `docs/house-navigation.md`; whole-house browser acceptance remains tracked by the umbrella issue.
