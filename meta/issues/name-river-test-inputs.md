# Name the river test's inputs when they are missing

## Summary

The opt-in `owned_rom_river_cycle_and_native_membership` test panics with a
bare `NotPresent` when `CRYSTA_ANIMATION_ROM` or `CRYSTA_ANIMATION_CAPTURES`
is not set. It passes when both are set, as its README says.

## Dependencies

- [Animate the river in native Crysta](animate-native-crysta-river.md)

## Requirements

- The ignore reason and the failure message name the missing variable.
- The test's checks do not change.

## Acceptance Criteria

- Without the variables the test fails with a message that names them.
- With both set it still passes; `assets` Clippy passes.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice).

## Result

- The ignore reason names both variables, and each `unwrap` on them is an
  `expect` that names the variable. No check changed.
- Without the ROM variable the test now fails with "set CRYSTA_ANIMATION_ROM
  ..."; with only the ROM, with "set CRYSTA_ANIMATION_CAPTURES ...". Before,
  both failed with a bare `NotPresent`. With both set to the ROM and
  `local/pandora-tower-discovery/frozen-reference/journey` it passes.
  `assets` Clippy passes. This is test tooling, so there is no separate red
  test; the old bare message was the red observation.
