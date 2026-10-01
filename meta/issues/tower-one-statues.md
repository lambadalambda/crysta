# Draw tower 1's statues and plaque

## Summary

Tower 1's guardian plaque and two statues share the mode-`$0004` descriptor
`$82:F65A`, which the sprite decoder refuses; magenta placeholders show
instead (`docs/mode4-descriptors.md`).

## Dependencies

- [Enter the towers from the world map](enter-the-towers.md)

## Requirements

- Decode mode-`$0004` descriptors: graphics in the second OBJ name table,
  the two-slot palette, frames with unsigned part offsets; place the actors
  with `COP B1`/`B3`.

## Acceptance Criteria

- The statues and the plaque match the native frame on both ROMs.
