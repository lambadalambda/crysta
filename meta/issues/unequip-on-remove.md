# Unequip an item when it is taken away

## Summary

`Inventory::remove` (`$8D:96A0`) takes an item but does not unequip it (`$0648`).

## Dependencies

- [Port menus, inventory, configuration, and saves](menus-inventory-save.md)

## Requirements

- Clear the equipped slot as natively.

## Acceptance Criteria

- A test removes an equipped item and finds it unequipped.
