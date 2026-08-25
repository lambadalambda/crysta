# Complete the first tower and Chapter 1

## Summary

Implement all remaining content and behavior needed to finish Chapter 1 through portable systems.

## Dependencies

- [Complete the Crysta and Pandora vertical slice](opening-vertical-slice.md)
- [Implement the classic renderer](classic-renderer.md)
- [Integrate a compatible SPC audio backend](spc-audio-backend.md)
- [Port menus, inventory, configuration, and saves](menus-inventory-save.md)

## Requirements

- Cover tower maps, enemies, bosses, items, scripts, resurrection sequences, and chapter transition.
- Add focused fixtures for unique mechanics.
- Run a deterministic end-to-end Chapter 1 replay.
- Remove reference CPU fallbacks from the supported path.

## Acceptance Criteria

- Chapter 1 can be completed from a new game using only portable game logic.
- Named progression, inventory, boss, and transition state matches reference checkpoints.
- Classic rendering and audio remain stable for the full replay.

## Notes

- Milestone: [M5 — Classic presentation and Chapter 1](../milestones.md#m5-classic-presentation-and-chapter-1)
- Treat chapter completion as a vertical product milestone, not merely map coverage.
