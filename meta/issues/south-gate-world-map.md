# Leave through the south gate onto the world map

## Summary

The south exit `$818DB3` leads to map `$03` under a separate world-map controller, arriving at (536,544).

## Dependencies

- [Play the frozen return and the Elder's mission](frozen-return-mission.md)

## Requirements

- Admit map `$03` with its background, collision and world-map player movement from source.
- Establish whether the exit is gated before the mission, and model that.

## Acceptance Criteria

- The full replay from new game arrives on the world map as natively at frame 59760; Ark can walk there.
- Tests and gates pass.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice).
- Parent: [Play the Crysta story from the wake-up scene to the world map](play-crysta-story.md)
