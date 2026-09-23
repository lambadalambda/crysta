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
