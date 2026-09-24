# Keep money, Prime Blue and the inventory

## Summary

Model money (`$0694`/`$0696`, BCD), Prime Blue (`$07ED`) and the inventory
at `$7F:8000` with its slot rules, in place of the plain list of items
given ([notes](../../docs/shops.md)).

## Dependencies

- [Open the shops in Crysta](crysta-shops.md)

## Acceptance Criteria

- Adding and taking money and items follows `$8D:95DB`, `$8D:95FF`,
  `$8D:9653` and `$8D:96A0`, with the caps and slot limits.
- Items scripts give (`COP 54`, `COP 60`) go into the inventory.
