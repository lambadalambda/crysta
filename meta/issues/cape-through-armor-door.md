# Equip the cape through the armor door

## Summary

`GRANT_ITEM` puts Elle's cape on at once (`actors.rs`, `CAPE`), a stand-in for the menu's armor door, which is not ported.

## Dependencies

- [Weave the cape and play tower 5](tower-five.md)
- [Port menus, inventory, configuration, and saves](menus-inventory-save.md)

## Requirements

- Remove the auto-equip when the menu's armor door is ported.

## Acceptance Criteria

- Ark wears the cape only after the player equips it; `$11D`'s orb check sends him back without it.

## Notes

- Research: `docs/tower-five.md`.
