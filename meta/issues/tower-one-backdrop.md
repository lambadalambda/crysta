# Draw tower 1's sky backdrop

## Summary

`$100`'s second layer is a fixed night sky and forest that does not scroll; it fills most of the screen during the intro pan. Its source and its darkened top lines are not researched. A "Tower 1" title also flies in during the pan.

## Dependencies

- [Enter the towers from the world map](enter-the-towers.md)

## Requirements

- Research and draw the backdrop layer and the title sprites.

## Acceptance Criteria

- The intro pan matches native frames.

## Notes

- The night sky is drawn: the map's second layer, fixed behind the first, darkened per line as `$97:B4BA`'s HDMA table does (checked by eye). The "Tower 1" title's fly-in is still open; our area title stands still.
