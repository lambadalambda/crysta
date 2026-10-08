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
