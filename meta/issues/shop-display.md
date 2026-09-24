# Draw the shop display

## Summary

Draw what the shop shows while browsing: Ark holding the item up, and the
name and price window of the display actor `$92:D190`.

## Dependencies

- [Run the shop's talk callback](shop-state-machine.md)

## Acceptance Criteria

- The held item and the window match a native frame of each shop.

## Notes

- The window's layout is not decoded yet; a frame from the reference
  emulator is needed.
