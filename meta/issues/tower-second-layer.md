# Draw the towers' second layer

## Summary

The app draws a map's second layer (BG2) only in Crysta (`$0A`-`$21`) and on `$100` (`SecondLayer::from_rom`). On the other tower maps BG2 is not drawn, and the second-layer copies of `COP 46` (layer select 1 to `$7F`, `docs/block-patch.md`) and of the load's flag patches change nothing on screen: the doors of `$101`, `$102`, `$109`, `$10A`, `$10B`, `$111` and `$118` open only on the first layer.

## Dependencies

- [Make the whole underworld playable](underworld-playable.md)

## Requirements

- Decode and draw BG2 on every tower map, with its priority against BG1 and the sprites.
- Keep the second layer's patches in the runtime (`COP 46` with layer 1 to `$7F`, the load's `second_layer` flag patches) and draw them.

## Acceptance Criteria

- A tower door that a script opens matches native frames before and after, on both layers.

## Progress

- Research: `docs/tower-second-layer.md`. The second layer is the front layer on towers 1, 2 and 4 (the layers swapped, profile `$07`): torches, pillars, ledges, door frames.
- `assets::maps::visual::profile` reads each map's display profile and classifies the layer: front, added, sky, subtracted, hidden.
- The app draws the front layer over the first by priority, the towers' skies and added light, and the second layer's patches (`COP 46` layers 1 to `$7F`, flag patches), which the runtime now keeps.
- The subtracted layer of `$11B` and `$123` darkens the background and OBJ palettes 4-7 (rasters mark them, `MATH_ALPHA`); Ark (palettes 0-3) and the HUD stay.
- Open: the added light's low pixels under a sprite (`docs/tower-second-layer.md` §4.4); native frames of `$10F`, `$11B` and `$123` (the add and subtract rules come from the registers).
