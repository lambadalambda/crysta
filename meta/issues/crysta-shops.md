# Open the shops in Crysta

## Summary

Maps `$1D` and `$1E` are shops. Their shopkeepers run, but the shop itself
(the item list, prices, buying, money) is not modelled, so there is nothing
to buy.

## Dependencies

- [Make the Crysta slice fully playable](playable-crysta-slice.md)
- [Port menus, inventory, configuration, and saves](menus-inventory-save.md)

## Requirements

- Decode the shop stock from source: the spawner at `$92:CC8A` searches the
  9-byte records at `$96:C6DC` (map, flag, inventory pointer, cell) for the
  current map (`$047E`); the inventory pointer names an item and price list.
- Open the shop menu when the player talks to the shopkeeper, with buying
  that takes money (`$07ED`) and adds the item.

## Acceptance Criteria

- Talking to each shopkeeper in `$1D` and `$1E` opens a shop with the native
  items and prices, and a purchase changes money and inventory as natively.

## Sub-issues

1. [Decode the shop stock](shop-data.md)
2. [Decode the shop texts' indexed controls](shop-text.md)
3. [Keep money, Prime Blue and the inventory](money-and-inventory.md)
4. [Run the shop's talk callback](shop-state-machine.md)
5. [Draw the shop display](shop-display.md)

## Notes

- Found by the town sweep (2026-09-24). Research: [shops](../../docs/shops.md).
  `$1E` is the item shop, `$1D` the Prime Blue shop; money is `$0694`, not
  `$07ED` (Prime Blue).
- Needs a menu layer that the slice does not have yet.
