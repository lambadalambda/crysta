# Decode the shop stock

## Summary

Decode the shop records at `$96:C6DC`, their stock lists and the Prime
Blue costs from the ROM ([notes](../../docs/shops.md)).

## Dependencies

- [Open the shops in Crysta](crysta-shops.md)

## Acceptance Criteria

- `$1E` lists items `$10`, `$11`, `$13`, `$80`, `$A1` at 10, 25, 13, 170,
  190 (flag `$D8` clear) and `$1D` lists `$01`, `$03` at 5 each.
