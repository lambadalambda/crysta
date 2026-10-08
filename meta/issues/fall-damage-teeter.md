# Confirm the fall damage and the pit-edge teeter

## Summary

A fall without a floor below costs a thirty-second of the most life, at least 4, and keeps Ark at 1 (guesses); the `$048A` bit 15 gate is modelled (a tower floor, from the spawn list's header). The teeter at a pit's edge is not modelled.

## Dependencies

- [Play towers 2 and 3](towers-two-three.md)

## Requirements

- Confirm `$84:D4F4`'s damage and the `$048A` gate.
- Model the teeter.

## Acceptance Criteria

- The life lost and the teeter match native frames.

## Notes

- Marker: `world/fall.rs`. Research: `docs/tower-three.md`.

## Progress

- The damage is `$84:D4F4`'s: a sixteenth of the life or of half the most life, whichever is more, at least 4; life can reach 0, and the upkeep then sends Ark down ([life-upkeep](life-upkeep.md)). Its digits show over Ark, and he is hidden the frame he lands. Open: the teeter.
- Done: the teeter is the rope state (`$84:9C95`, `$80:CCB4`), which `world/fall.rs` models.
