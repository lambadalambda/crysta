# Play Shadowkeeper's full fight

## Summary

`$123`'s Shadowkeeper runs its native scripts (`docs/tower-five.md`); its drawing is partial.

## Dependencies

- [Weave the cape and play tower 5](tower-five.md)

## Requirements

- Run the native intro: the darkness, the torches and the camera's pan.
- Run the native body script, or model its claws, tail, shots and phases.
- Show the "Defeated Shadowkeeper!!" text.
- Draw vertically flipped actors (`+$08` bit `$8000`, `COP B4`/`B5`/`B9`): the runtime moves them flipped but does not export the flip to the drawing.

## Acceptance Criteria

- The fight matches the longplay's frames from the intro to the explosion, Japanese and European.
- `world/shadowkeeper.rs` is gone or holds no stand-in.

## Notes

- Research: `docs/tower-five.md` §1, §5.5.

## Subissues

- [Run `$123`'s intro natively](shadowkeeper-intro.md)
- [Run Shadowkeeper's scripts natively](shadowkeeper-scripts.md)
