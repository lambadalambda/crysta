# Confirm the fall damage and the pit-edge teeter

## Summary

A fall without a floor below costs a thirty-second of the most life, at least 4, and keeps Ark at 1 (guesses); the `$048A` bit 15 gate is taken as always set. The teeter at a pit's edge is not modelled.

## Dependencies

- [Play towers 2 and 3](towers-two-three.md)

## Requirements

- Confirm `$84:D4F4`'s damage and the `$048A` gate.
- Model the teeter.

## Acceptance Criteria

- The life lost and the teeter match native frames.

## Notes

- Marker: `world/fall.rs`. Research: `docs/tower-three.md`.
