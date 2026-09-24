# Let Ark run

## Summary

Ark can run in the game; the slice only walks.

## Dependencies

- [Make the Crysta slice fully playable](playable-crysta-slice.md)

## Requirements

- Decode how running starts and stops, its speed and its animation, and
  model them in the movement.

## Acceptance Criteria

- Running starts, moves and stops as on a native recording.

## Resolution

- A double tap dashes (`room_core::run`), as measured on the native probe:
  3, 2, 2 pixels a frame, 8 frames of grace, a 16-frame brake with sound
  `$0D`, a walk after a wall; the app draws the dash and brake lists
  (2026-09-24). Not modelled: diagonal dashes, the dash attack and jump.
