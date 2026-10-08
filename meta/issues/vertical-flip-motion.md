# Flip movement streams on the vertical axis

## Summary

Movement streams (`actors/motion.rs`) negate velocities on a flipped X axis only. Vertical flips (`+$08` bit `$8000`) are not modelled.

## Dependencies

- [Make the Crysta slice fully playable](playable-crysta-slice.md)

## Requirements

- Negate Y velocities for vertically flipped actors.

## Acceptance Criteria

- A test with a vertically flipped actor moves as natively.

## Progress

- Done: `COP B4`/`B5`/`B9` set, clear and toggle `vflip`; the streams read both flips each frame (`$80:F297`, `F2F3`); children inherit it. Drawing a flipped actor is in [shadowkeeper-full-fight](shadowkeeper-full-fight.md).
