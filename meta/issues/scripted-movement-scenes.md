# Run scripted movement, entry scenes and map transfers

## Summary

Scenes move Ark and residents, run on map entry (C's changed entry, the town controller) and transfer maps (COP14). None of this is executed.

## Dependencies

- [Apply flag-gated doors, blockers and tile patches](flag-gated-geometry.md)

## Requirements

- Execute forced player and NPC movement, facing, waits and entry controllers.
- Execute map transfers with their arrival positions and ownership.

## Acceptance Criteria

- C's entry scene sets `$27` and its choice `$2E` as natively, with the same page sequence.
- Movement matches native positions at the route's labels; gates pass.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice).
- Parent: [Play the Crysta story from the wake-up scene to the world map](play-crysta-story.md)

## Progress: room C's entry scene

- The static length derivation now counts `$80:BC2F`'s stream read, which
  fixes `COP 13` (3 bytes, was 1) and eight other services' lengths.
- New services: `COP 13` (place), `49` (map delete), `85` + `8F` (repeated
  pose), `4B` (counters), `BD` (yield); legs at all nine common-stream
  speeds on the common movement base; `COP 0F` compares the facing; map
  loads clear locals and counters and keep items. `BA`/`D8` are stepped over.
- The friends' scene plays from the ROM with real presses and matches the
  native journey: placement, `$27`, about 41 frames to the first page, six
  pages, a 64-frame walk, choice, two pages, `$2E`, the walk back, unlock.
- Remaining here: the town controller scene (`$3C`), which comes later on
  the path. `COP 14` transfers run since the cellar box (a same-map
  reload). Also the refusal branch (`$2F`)
  needs text command `$CB`.
- Independent review approved (24-map trace comparison: no new freezes;
  `$1F`'s `$83:921B` now idles in a `BD` loop with a live callback, and
  `$20`/`$0E` residents are deleted by `COP 49` on entry, as specified).
  Follow-ups applied: tests for `COP 0F`'s facing match and for the
  map-load reset through a real door (locals cleared, globals and items
  kept); `Globals::default` sizes the counters; `COP 13` mirrors only for 2.
