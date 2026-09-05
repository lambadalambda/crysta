# Qualify a fresh new-game house bootstrap

## Summary

Establish the actual Japanese new-game input path and initialization needed to reach controllable Ark in his house. Existing name-entry experiments do not qualify confirmation or a defect.

## Dependencies

- [Qualify an opening room transition](qualify-opening-room-transition.md)
- [Inspect stopped CPU registers](inspect-stopped-cpu-registers.md)

## Requirements

- Use fresh `Session::new` boots with default empty SRAM, one boot per process; no state patches or snapshot restores.
- Visually confirm menu/name-entry selections before interpreting continued waits as a stall.
- Capture named map/player/input-control/event-state checkpoints and source evidence for initialization.
- Provide an automatic repeatable input replay/checker; keep raw memory and visual artifacts local.

## Acceptance Criteria

- Two independent fresh boots reach the same controllable-house checkpoint from documented inputs and reproduce selected hashes.
- A committed optional owned-ROM harness asserts startup/control ownership, not only coordinates or elapsed time.
- Documentation separates decoded initialization semantics from opaque intro/dialogue behavior.

## Notes

- Subissue of [start and explore Ark's house](start-and-explore-arks-house.md).
