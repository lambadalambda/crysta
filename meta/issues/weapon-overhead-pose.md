# Hold the weapon over Ark's head when he gets it

## Summary

When Ark gets the weapon, he does not hold it over his head as natively.

## Dependencies

- [Fix the slice bugs the user reported on 2026-10-01](slice-bug-report.md)

## Requirements

- Research the get-item presentation (Ark's pose, the item sprite over him, timing) and draw it.

## Acceptance Criteria

- The weapon's presentation matches native frames.

## Notes

- `COP 60` starts a presentation: the lift for 22 frames, its stand to frame 422, the icon 40/44/42 pixels above Ark, the pad locked meanwhile (`local_story.rs` `ark_takes_the_crystal_spear_and_returns_to_the_box_room`, `docs/ark-poses.md`).
