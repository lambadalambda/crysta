# Draw the vortex and the Chapter 2 title card

## Summary

The end of Chapter 1 (`world/chapter.rs`) shows 600 dark frames (a guessed length) in place of the vortex `$201`, and the title card without its picture. The European title text is refused by the page geometry and not shown.

## Dependencies

- [Finish the underworld](underworld-end.md)

## Requirements

- Run or draw the vortex (`$90:8436`, Mode 7) with its native length.
- Draw the card's picture.
- Show the European title text (check the 8-pixel advance).

## Acceptance Criteria

- The vortex and the card match native frames, Japanese and European.

## Notes

- Research: `docs/underworld-end.md` §2.
