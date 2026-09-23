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

## Resolution

- `World::update` walks Ark out through an exit (doors 16 px, stairs the
  measured 63-frame climbs and descents), loads, places him at the native
  arrival start (raw + `$8D:8985` adjustment + (8,16)) and walks him in;
  16-frame fades out and in (`World::brightness`, fade type 0). The app dims
  the view.
- Story tests pin C to D (16 frames out, (120,608) to (120,625), the fade
  levels) and every stair spawn and rest on the native route.
- Gaps: script transfers (`COP 14`) still load at once, without their
  modes' fades (the box's mosaic); sideways doors reuse the vertical door's
  pattern; the loads' own dark frames are skipped; the stepping API keeps
  its instant legacy placement for route discovery.
