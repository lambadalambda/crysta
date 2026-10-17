# Draw colour addition

## Summary

The display models colour subtraction (darkening) only; colour addition is not modelled (`display.rs`).

## Dependencies

- [Implement the classic renderer](classic-renderer.md)

## Requirements

- Model addition with its layers and fixed colour.

## Acceptance Criteria

- A scene that adds colour matches native frames.

## Notes

- `$11D`'s guardian holds Ark with colour math and the ring window
  (`COP 64`, `World::ring`, modelled with native tables) and lightens the
  backdrop; the hosts draw neither yet.
- The light room's palette cycle (`COP 8A $1F`, `World::palette_cycle`) is
  recorded, not drawn.
