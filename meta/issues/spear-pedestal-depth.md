# Draw the weapon in front of its pedestal

## Summary

In the weapons room the pedestal draws over the weapon, which is barely visible.

## Dependencies

- [Fix the slice bugs the user reported on 2026-10-01](slice-bug-report.md)

## Requirements

- Find the native order of the weapon and the pedestal (priority, OAM order) and draw by it.

## Acceptance Criteria

- The weapons room matches native frames with the weapon in place.
