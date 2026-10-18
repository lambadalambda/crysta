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

## Progress

- Done (native traces `loads/to0d` and the town exit, with the frame's
  brightness, 2026-10-18; the load frames of every map change on the
  native route): a door walks 17 frames; the old map stays for the load
  (5 frames between rooms, 4 between tower rooms, 26 to 77 for the big
  loads); the screen stays black until the dark's frames and 3 after the
  walk's last, with the world running; the arrival walk starts 14 before
  the fade in. `exit-while-carrying` replays to its end on the Japanese
  ROM. European: [Time the European loads and arrivals](european-loads.md).
