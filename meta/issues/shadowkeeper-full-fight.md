# Play Shadowkeeper's full fight

## Summary

`$123`'s Shadowkeeper runs its native scripts (`docs/tower-five.md`); its drawing is partial.

## Dependencies

- [Weave the cape and play tower 5](tower-five.md)

## Requirements

- Run the native intro: the darkness, the torches and the camera's pan.
- Run the native body script, or model its claws, tail, shots and phases.
- Show the "Defeated Shadowkeeper!!" text.
- Draw vertically flipped actors (`+$08` bit `$8000`, `COP B4`/`B5`/`B9`).
- Draw the parts in their OBJ palettes: `COP 5A` loads and `COP BB` fields (the shots are green natively, dark here), and in `+$06`'s depth order.

## Acceptance Criteria

- The fight matches the longplay's frames from the intro to the explosion, Japanese and European.
- `world/shadowkeeper.rs` is gone or holds no stand-in.

## Notes

- Research: `docs/tower-five.md` §1, §5.5.

## Subissues

- [Run `$123`'s intro natively](shadowkeeper-intro.md)
- [Run Shadowkeeper's scripts natively](shadowkeeper-scripts.md)

## Progress

- The scripts run (`shadowkeeper-scripts`, archived); the stand-in is gone. The text shows; vertically flipped actors are drawn; a mode-4 body draws its other lists (the claws, the wisps, the shots).
- Open: the darkness and window (`shadowkeeper-intro`), the OBJ palettes the parts take from `COP 5A`/`COP BB`, the depth order of `+$06` bits 11-14.
