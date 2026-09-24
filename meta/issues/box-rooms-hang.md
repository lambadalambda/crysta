# Let Ark walk in the box's rooms after an arch

## Summary

Entering a room of the box through an arch left the pad locked: the arch
locks it (`COP 2A $F0FF`) before `COP 14`, and the runtime carried the mask
across the transfer.

## Dependencies

- [Play the Crysta story from the wake-up scene to the world map](play-crysta-story.md)

## Acceptance Criteria

- After each arch, Ark walks in `$42`, `$43` and `$44`; natively `$045E` is 0
  in `$42` after the weapon door.

## Notes

- Reported by the user while playing the slice (2026-09-23).
- Fixed: every load clears the mask; scripts lock it again where they hold
  the player (the guide in the reloaded `$21`, `$88:AEC5`).
- Parent: [Play the Crysta story from the wake-up scene to the world map](play-crysta-story.md)
