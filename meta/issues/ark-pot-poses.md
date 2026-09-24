# Show Ark's lift, carry and throw poses

## Summary

Lifting a pot plays no lift animation, and while carrying Ark keeps his ordinary walking frames with the pot drawn above him.

## Dependencies

- [Play the Crysta story from the wake-up scene to the world map](play-crysta-story.md)

## Requirements

- Ark's lift, held idle/walk and throw poses from his animation tables (`docs/pandora-pots.md` sprite table: table0 seq03-05, table1 seq09-0B, table3 seq0F-11).
- The app draws them from the pot component's phase.

## Acceptance Criteria

- Lifting, carrying and throwing show the native poses; tests pin the selected sequences.

## Notes

- Reported by the user while playing the slice (2026-09-23).
- Parent: [Play the Crysta story from the wake-up scene to the world map](play-crysta-story.md)

## Resolution

- `World::carry` gives the component's phase as a carry motion, facing and
  tick; the app draws `PandoraSprites::carry_pose`'s Ark and pot lists
  (lift and throw once, holding looping) and the flight list `$3C`.
- Tests pin the motion sequence of the native MISS segment (23 lift
  frames, 32 throw frames) and that every pose rasterizes.
