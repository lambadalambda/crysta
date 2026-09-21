# Walk between Crysta maps through the real exit geometry

## Summary

Free roam needs the player to leave a map by walking into its exits and arrive
somewhere correct in the destination. The exit geometry is decoded and the
door-entry mechanism is measured; what is missing is a runtime that applies
them.

## Dependencies

- [Promote the Crysta room builder into a library](crysta-room-library.md)
- [Decode the door-entry trigger for one-cell exits](decode-door-entry-trigger.md)

## What is already known

- `assets::maps::exits::ExitList` gives each map's exit rectangles and their
  destinations.
- The scan at `$8D:8797` tests the probe written at `$87:91B3` as
  `(x-8) & (mask_x|$0F)` and `(y-16) & (mask_y|$0F)`, so an exit is matched
  against a wrapped probe rather than the player's raw position.
- Some exits are doorways that require facing and an interaction rather than
  walking through, measured live. Most town entrances sit on cells the player
  cannot stand on, which is why walking alone reaches only part of the slice
  and doorway interaction is what connects it.

## Requirements

- Transition on the probe geometry the game uses, not on the player's bounding
  box overlapping a rectangle.
- Distinguish walk-through exits from doorways that need an interaction, and
  drive both from the decoded data rather than a hand-written table.
- Place the player on arrival at a cell they can stand on, derived from the
  destination's own data.
- An exit whose destination is outside the slice is refused as movement — the
  player does not pass — rather than transitioning to an unloaded map.

## Acceptance Criteria

- Every bound doorway in the slice can be entered and leaves the player
  somewhere standable.
- A round trip returns the player to a cell adjacent to the exit they used, not
  to an arbitrary arrival point.
- The whole slice is connected: a walk plus doorway interactions reaches all 24
  maps from the opening house.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice)
- Parent: [Play the Crysta slice free-roam in a native app](free-roam-crysta-app.md)
- `local_crysta_rooms.rs` already proves connectivity offline with
  `reachable_maps`; this is that logic made live.
