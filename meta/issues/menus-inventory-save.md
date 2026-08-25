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

## Notes

- Milestone: [M5 — Classic presentation and Chapter 1](../milestones.md#m5-classic-presentation-and-chapter-1)
- Portable snapshots and in-game SRAM saves are distinct formats.
