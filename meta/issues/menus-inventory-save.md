# Port menus, inventory, configuration, and saves

## Summary

Implement the gameplay UI and persistent state required for a normal Chapter 1 playthrough.

## Dependencies

- [Define the deterministic portable core model](deterministic-core-model.md)
- [Decode text and gameplay data tables](decode-text-gameplay-data.md)
- [Implement the portable event runtime](portable-event-runtime.md)

## Requirements

- Port inventory/equipment rooms, configuration, status, and relevant dialogue choices.
- Model SRAM layout and checksum behavior.
- Provide portable storage adapters and versioned snapshots.
- Test save/load at representative progression points.

## Acceptance Criteria

- A classic save round-trips through the documented SRAM representation.
- Menu input and selection replays match reference checkpoints.
- Corrupt and incompatible saves fail safely without data loss.

## Sub-issues

1. [Keep the native save slot in the world](save-slot-block.md)
2. [Read and write native SRAM](sram-codec.md)
3. [Save at the bedroom desk](save-point.md)
4. [Continue a saved game in the app and on the page](continue-saved-game.md)
5. [Open Yomi's box after the frozen return](yomi-menu.md)

## Notes

- Milestone: [M5 — Classic presentation and Chapter 1](../milestones.md#m5-classic-presentation-and-chapter-1)
- Portable snapshots and in-game SRAM saves are distinct formats.
- Decided 2026-09-29: the menu keeps the native gate (flag `$FE`); saves
  come first. Research: [saves](../../docs/saves.md), [menu](../../docs/menu.md).
