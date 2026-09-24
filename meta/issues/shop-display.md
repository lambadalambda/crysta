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

## Progress

- Drawn from ROM art (`assets::shop_display`, `crysta_app::shop`): the
  name, icon, count row, coin, price, bag and money, and Ark's hold pose
  after a purchase (2026-09-24). A frame of `$1E` matches the native
  capture but for the text window's style and the room's light beams.
  Open: a native frame of the Prime Blue shop `$1D` (its two Prime Blue
  sprites, `$92:D52C`/`$92:D53F`), and the weapon and armour markers
  (`$85:E699`, `$85:E6F7`) no Crysta shop shows.
