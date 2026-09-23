# Run scene dialogue, choices and flags from scripts

## Summary

Talking today merges every callback's text and applies all flag writes before the first page. Run interaction scripts instead: pages, choices and flag writes at their true point, with the player's control owned by the scene.

## Dependencies

- [Play the Crysta story from the wake-up scene to the world map](play-crysta-story.md)

## Requirements

- A scene executor steps a script over frames: text services show pages and wait for acknowledgement, choice services show the catalog and branch on the answer (`$22` jump tables), flag writes happen where the script makes them.
- Control ownership: while a scene owns control, player input only advances or answers it.
- Unknown services stop the scene visibly and are logged, not skipped.

## Acceptance Criteria

- The Elder in B sets `$26` after the first request and before the choice; either answer continues, as natively.
- The weaver in `$13` sets `$28` only for answer 1; answer 2 or cancel refuses and re-prompts.
- Pure tests per service, owned-ROM tests for both conversations; gates pass.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice).
- Parent: [Play the Crysta story from the wake-up scene to the world map](play-crysta-story.md)
