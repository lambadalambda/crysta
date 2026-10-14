# Draw the world map's horizon band

## Summary

On `$03` the record `$83:8909` (`$84:E3E6`) holds 40 OBJ fixed on the screen at (128,53). Their grey (palette 5) replaces the fog colour that lines 52-67 subtract from the plane, which shapes the dark band at the horizon. The app subtracts the fog colour everywhere (`docs/world-map-mode7.md`).

## Dependencies

- [Make the whole underworld playable](underworld-playable.md)

## Requirements

- Decode the band's pieces (`$B1:BE49`, second name table) and palette 5.
- On lines 52-67, subtract the band's colour where it is opaque.

## Acceptance Criteria

- The app's `$03` horizon matches `local/mode7/jp/arrival-59762/native.png` on lines 52-67.

## Progress

- Done: the band (`art::horizon_band`, `Mode4Art`'s short descriptor form) replaces the fog on lines 52-67; `local_mode7` matches all six native frames on both ROMs with the frame's CGRAM.
- Open: natively OBJ palette 5 moves a step darker a few frames after the arrival (59762 vs 59768); the app keeps the descriptor's palette. Its writer is not traced (possibly the world map's palette cycle, `$87:98BF`).
