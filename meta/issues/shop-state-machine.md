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
