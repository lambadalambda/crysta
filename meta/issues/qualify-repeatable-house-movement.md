# Qualify repeatable house movement and collision

## Summary

Replace the preview's permanent used-direction guard and narrow flat-only collision domain with the evidence needed for normal repeatable exploration of Ark's house.

## Dependencies

- [Implement a reference-qualified portable room slice](portable-room-slice.md)

## Requirements

- Qualify release/repress and direction-history admission, including the boundary between ordinary walking and accelerated/dash behavior.
- Trace relevant mixed corners, furniture/partial materials and flagged-cell dispatch in the house; do not invent generic AABB collision or mask unknown flags away.
- Establish repeatable routes through the required house rooms and internal doorway ownership boundaries.
- Implement supported behavior with synthetic red-green tests and authenticated per-step reference comparisons; reject only genuinely remaining unsupported actions.

## Acceptance Criteria

- A committed reproducible harness captures fresh-input trajectories exercising normal revisits/reversals and required collision classes.
- Portable tests match the selected repeated exploration routes without the old used-once restriction or unsupported-cell pauses.
- Replay/snapshot determinism and native/Wasm builds remain valid.

## Notes

- Subissue of [start and explore Ark's house](start-and-explore-arks-house.md).
- The previous 1,204-step strict profile remains a regression corpus, not sufficient evidence for arbitrary input histories.

## Progress: open/solid corners

- Collision profile v2 now supports decoded mixed open/solid perpendicular
  nudges with main-axis snap/rollback; type16 and flagged cells remain rejected.
- Twelve authenticated trajectories match 1,971 positions/stream steps and
  snapshot-restored continuations. Dedicated ±Y nudge routes reproduce six
  CSV/WRAM artifacts byte-for-byte across two cold boots.
- Synthetic tests cover all four directions/material orders/remainders plus
  mixed rollback, magnitude2 and q-boundary continuation. Old snapshots are
  explicitly rejected; source/data identity includes the new profile version.
- See [corner evidence](../../docs/house-movement.md). This is partial progress:
  repeatable direction admission and the new-game/exploration goal remain open.
