# Finish the partial COP services

## Summary

Some COP services refuse or skip a rare case. No played script needs one yet.

## Dependencies

- [Make the Crysta slice fully playable](playable-crysta-slice.md)

## Requirements

- `COP 4B` bit 6, the subtraction (`scene.rs`, `count`).
- The pad test's bit 0, which also asks `$048A & $8000` (`$80:902D`).
- The blocked test's other-map case (`$0868` bit 7); a blocked wall-follower waits a frame instead.
- `COP 46`'s copies to the second layer (picture only).
- The movement resource base from a word and a bank (`FF`, `$80:A975`).
- The `+$04` bits that scripts set and the runtime accepts without effect.

## Acceptance Criteria

- Each case is modelled with a unit test, or shown to be unused in the ROM.

## Notes

- Markers: `actors.rs`, `scene.rs`.
