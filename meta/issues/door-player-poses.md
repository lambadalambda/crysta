# Show the player's poses at a wooden door

## Summary

The wooden doors (`world/door.rs`, `$87:97CA`) open, but the player's poses (`COP CB`) and the `$7F:1020` writes are not modelled. `$87:C7F1`'s other branch, which wants the raw word `$00F3` while `$04F6` is nonzero, is not modelled either.

## Dependencies

- [Make the Crysta slice fully playable](playable-crysta-slice.md)

## Requirements

- Draw the poses and keep the `$7F:1020` writes.
- Trace what sets `$04F6` and model the branch.

## Acceptance Criteria

- Opening a door matches native frames.
