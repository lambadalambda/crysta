# Finish the partial COP services

## Summary

Some COP services refuse or skip a rare case. No played script needs one yet.

## Dependencies

- [Make the Crysta slice fully playable](playable-crysta-slice.md)

## Requirements

- `COP 4B` bit 6, the subtraction (`scene.rs`, `count`).
- The pad test's bit 0, which also asks `$048A & $8000` (`$80:902D`).
- The blocked test's other-map case (`$0868` bit 7); a blocked wall-follower waits a frame instead.
- `COP 46`'s copies to the second layer (picture only): [Draw the towers' second layer](tower-second-layer.md).
- The movement resource base from a word and a bank (`FF`, `$80:A975`), and the other private bases (`COP B0 n`, n not 0 or 2: packed from `$7F:4000` in load order, `$80:FB5A`).
- The `+$04` bits that scripts set and the runtime accepts without effect.

## Acceptance Criteria

- Each case is modelled with a unit test, or shown to be unused in the ROM.

## Notes

- Markers: `actors.rs`, `scene.rs`.

## Progress

- `COP 4B` bit 6: BCD subtraction floored at 0, at `$0640 + (op & $BF)`; the word is read but not skipped (`$80:979D`). No script in the ROM sets bit 6.
- The pad test's bit 0: `$048A` is now the map's mode (the spawn list's header, `$86:957B`); bit 15 marks a tower floor.
- The blocked test probes as `$80:C092`..`C116` do: a cell beyond the box's edge, `ceil(size / 16)` cells, passing attributes 0, 1 and 22 only. Open: the world map's plane probe (`$80:C164`).
- `COP B0 FF`: a ROM resource, its stream pointers relative to its table (`$80:F313`). Open: `COP B0 n` for other n.
- `+$04` bit 12 (`$1000`) gates the whole actor loop (`$80:C871`: no script, no movement while clear); bits 8 and 9 are tested by the interaction (`$87:C789`) and the hit scans (`$85:D2C3`..). Not modelled yet.
