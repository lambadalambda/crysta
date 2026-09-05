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

## Verified result

- Committed owned-ROM harness reproduces13 checkpoint reports and the7,100-frame
  stream in two independent empty-SRAM processes. Both exit0; fresh negative
  controls without name confirmation or deliberate movement fail the checker.
- Visually identified the load menu and actual kana entry. Default アーク confirms
  with Start; no name-confirmation stall is established.
- Controllable bedroom mapF begins at304,112; deliberate Right/Down and release
  stabilize at332,140 with ownership/event/input checks, not just elapsed time.
- Parent reran `sh tools/new-game-qualification/replay.sh` successfully; captures
  are ignored under `local/new-game-qualification/replay-oOSrnU/`.
- [Evidence and decoded/opaque initialization boundary](../../docs/new-game-bootstrap.md).
  This reference-qualification issue is complete; portable initialization and
  user-facing New Game remain part of the open house-exploration parent.
