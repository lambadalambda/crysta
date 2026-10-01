# Enter the towers from the world map

## Summary

On the world map the towers cannot be entered.

## Dependencies

- [Fix the slice bugs the user reported on 2026-10-01](slice-bug-report.md)
- [Draw the world map in Mode 7](world-map-mode7.md)

## Requirements

- Find the tower entrances' triggers and destinations; load the tower maps they lead to.

## Acceptance Criteria

- Walking onto a tower's entrance loads it as natively.

## Sub-issues

1. [Load tower 1 from the world map](tower-one-loads.md)
2. [Walk into and out of tower 1](tower-one-arrival-exit.md)
3. [Play tower 1's intro pan and text](tower-one-intro.md)
4. [Draw tower 1's sky backdrop](tower-one-backdrop.md)
5. [Enter towers 2 to 5](towers-two-to-five.md)

## Notes

- Research: `docs/world-map-mode7.md`, `docs/tower-entry.md`.
