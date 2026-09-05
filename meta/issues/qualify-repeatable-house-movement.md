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
