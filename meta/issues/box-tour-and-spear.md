# Play the tour inside the box and take the spear

## Summary

Inside the box a guide leads a forced tour through `$41..$44` (`$243`, `$244`), then Ark walks a corridor to the weapon door and takes the Crystal Spear (`$240..$242`, item `$81`).

## Dependencies

- [Open the box in the cellar](cellar-box-sequence.md)

## Requirements

- Admit maps `$41..$44` for the native renderer and runtime.
- Execute the tour controller and guide, and the spear pickup with its item write.

## Acceptance Criteria

- Tour and pickup follow the native route's inputs and flags; tests and gates pass.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice).
- Parent: [Play the Crysta story from the wake-up scene to the world map](play-crysta-story.md)
