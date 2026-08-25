# Record deterministic input replays and snapshots

## Summary

Define versioned replay and snapshot fixtures that can reproduce reference behavior from known starting states.

## Dependencies

- [Select and integrate the reference emulator](select-reference-emulator.md)
- [Define the oracle artifact and publication policy](define-oracle-artifact-policy.md)

## Requirements

- Represent controller input per authoritative frame.
- Record ROM hash and harness version in fixture metadata.
- Support reset-based and snapshot-based scenarios.
- Detect incompatible or corrupt fixtures.

## Acceptance Criteria

- Replaying the same fixture twice produces identical selected state hashes.
- Fixture format and compatibility rules are documented.
- No fixture contains prohibited ROM data.

## Notes

- Milestone: [M1 — Reference oracle](../milestones.md#m1-reference-oracle)
- Prefer a simple inspectable format with a compact binary payload only where measurements justify it.
