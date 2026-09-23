# Animate door and stair transitions

## Summary

Doors and stairs move Ark to the next map at once: no walk out, fade or stair walk.

## Requirements

- Model the departure and arrival (the graph port's 17-frame door policy, the selector-14/13 stair motions) and the fades.
- The app draws the fade.

## Acceptance Criteria

- Doors and stairs play their departure and arrival as natively; tests pin the frames.

## Notes

- Reported by the user while playing the slice (2026-09-23).
- Parent: [Play the Crysta story from the wake-up scene to the world map](play-crysta-story.md)
