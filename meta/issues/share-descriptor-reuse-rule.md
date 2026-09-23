# Share one descriptor-reuse rule between sprites and cadence

## Summary

A spawn record with a zero descriptor pointer reuses the previous record's
resource. The sprite loader (`HouseActor::from_records`) steps over `$00`
and `$FD` records to find that predecessor; the cadence derivation refuses
a chain that contains them. Two rules for one source behaviour will drift.

## Dependencies

- [Derive each walker's step and idle timing from source](derive-walker-cadence-from-source.md)

## Requirements

- Establish from the record parser (`$80:F4EA`, `$80:FA65..FB1A`) whether
  `$00` and `$FD` records update the reuse state (`$46/$48/$4A`).
- Implement the rule once, in `assets`, and use it from both callers.

## Acceptance Criteria

- Pure tests of the shared rule, including `$00`/`$FD` in the chain.
- Sprite and cadence owned-ROM tests pass unchanged, or any change is
  explained by the established rule.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice).
