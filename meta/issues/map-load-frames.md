# Spend the map loads' own frames

## Summary

An exit's load takes no frames (`world/transition.rs`). Natively a load takes 3 to 5 frames, more between the house and the town, which only lengthens the dark.

## Dependencies

- [Make the Crysta slice fully playable](playable-crysta-slice.md)

## Requirements

- Measure the frames of each kind of load and spend them in the dark.

## Acceptance Criteria

- The frames from leaving to arriving match the native route.

## Notes

- The pot fixture `exit-while-carrying` (`local/pots/`) measures one: at
  the stairs' end Ark stands 3 frames, then 2 load frames, then the next
  map (rows 40 to 45). Its replay stops at the load until this is done.
