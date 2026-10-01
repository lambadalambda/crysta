# Draw the world map in Mode 7

## Summary

Outside the town the world map is drawn flat; natively it is a Mode 7 view.

## Dependencies

- [Fix the slice bugs the user reported on 2026-10-01](slice-bug-report.md)
- [Leave through the south gate onto the world map](south-gate-world-map.md)

## Requirements

- Research the Mode 7 setup (matrix, HDMA per line, horizon) and render it in the hosts.

## Acceptance Criteria

- The world map matches native frames as Ark walks.
