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

## Progress: measured ordinary reactivation

- Replaced the permanent used-direction mask with the source-backed last-onset
  direction and 11-tick countdown. Reversals and ordinary release/repress work;
  accelerated triggers reject atomically rather than masquerading as walking.
- 42 authenticated fixture pairs match 3,994 successful transitions and 19
  accelerated-trigger rejections, including a 209-step revisit route. Every
  accepted step has a snapshot-restored comparison; walking encoding is v3.
- [Admission evidence](../../docs/input-admission.md) records two fresh boots,
  native gate traces and boundary mutations. Additional materials and the
  fresh-start exploration integration remain open.

## Progress: return doorway

- Two cold saved-game replays qualify map10→F selector6, last walking anchor
  392,336; departure392,319, spawn392,208 and settled392,191.
- Five native dispatch stops support the decoded selector/negative-Y source
  path. Fourteen additional authenticated walking steps match the core.
- Semantic profile4 supports both internal handoffs, retaining explicit17/load/17
  logical timing. CPU-free two-process verifier returns to F at tick164;
  per-step snapshots preserve both route ownerships, including map switches.
- [Return evidence](../../docs/house-return-doorway.md). Fresh startup integration,
  additional materials and an actual browser exploration test remain outstanding.
