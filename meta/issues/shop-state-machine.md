# Run the shop's talk callback

## Summary

Port the shop loop `$92:CD70`: choosing the item and count, the price, the
confirm choice, the refusals, the sounds and the purchase.

## Dependencies

- [Decode the shop stock](shop-data.md)
- [Decode the shop texts' indexed controls](shop-text.md)
- [Keep money, Prime Blue and the inventory](money-and-inventory.md)

## Acceptance Criteria

- Talking to either shopkeeper greets, browses the stock with the native
  keys and sounds, and refuses or sells as `$92:D120` decides.

## Progress

- The loop runs in `crysta_runtime::shop` (2026-09-24); a story test browses,
  is refused, buys and leaves in `$1E`. Open: a test of the Prime Blue shop
  `$1D`, which opens only with Prime Blue (its guard `$88:C7EE` answers
  while `$07ED` is 0); which target answers when flag `$D8` spawns two.
