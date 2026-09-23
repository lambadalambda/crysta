# Apply flag-gated doors, blockers and tile patches

## Summary

Progress in Crysta is gated by hidden occupancy actors (D's gate needs `$26`), metatile and collision patches (the blue door, the cellar stairs) and COP3B stamps. None of these exist natively, so the player can walk anywhere.

## Dependencies

- [Run scene dialogue, choices and flags from scripts](script-dialogue-choices.md)

## Requirements

- Hidden bodies block by occupancy even without art.
- Loading-script and script-driven tile patches apply to background and collision.
- A flag write in the current map refreshes roster and patches as the game does.

## Acceptance Criteria

- Before `$26` the D gate blocks; after it, it opens; the blue door and stairs follow `$292`.
- The slice's reachability without progression drops to what the game allows; tests and gates pass.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice).
- Parent: [Play the Crysta story from the wake-up scene to the world map](play-crysta-story.md)
