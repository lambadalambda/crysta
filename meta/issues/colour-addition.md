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

- In the underworld the runtime reaches, only `$11D`'s guardian adds
  colour (a logged run of `$11D`, `$123`, `$127`, `$12A`: the rest
  subtract).
- The light room's palette cycle (`COP 8A $1F`, `World::palette_cycle`) is
  recorded, not drawn.

## Progress

- `$11D`'s light is drawn: inside the ring (`World::ring`) the sub
  screen's BG1 is added to the backdrop (`Display::backdrop_addition`,
  `frame::lighten_in_ring`). Against the native frame at `$0474` = 44 the
  ring's 2848 pixels are equal or one 5-bit step darker (the backdrop's
  every-other-frame phase, the `$42` lag). Open: addition in general, with
  its layers, halving and the sub screen.
