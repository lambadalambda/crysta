# Fix stale evidence paths in the cadence tool README

## Summary

`tools/crysta-cadence-qualification/README.md` tells the reader to verify
"existing private evidence" at `local/crysta-cadence/native/settledD.wram`
and `local/crysta-cadence/native.jsonl`. Those files no longer exist; fresh
probe runs replace them.

## Dependencies

- [Witness a class-2 Crysta walker natively](witness-class-two-walker.md)

## Requirements

- Point the verification commands at the output of a fresh probe run, and
  say that raw evidence is regenerated rather than retained.

## Acceptance Criteria

- Every command in the README runs as written from the repository root.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice).
