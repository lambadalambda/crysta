# Decode the shop texts' indexed controls

## Summary

The shop texts pick sub-texts by an engine word through the `CE` control
(`$0DE8` the shop type, `$0DD0` the item), which `assets::text` refuses.

## Dependencies

- [Open the shops in Crysta](crysta-shops.md)

## Acceptance Criteria

- Every stocked item's name decodes (`$92:8179`).
- The greeting, help, confirm, refusal and thanks texts of both shop types
  decode, given the words they read.
