# Create initial reference replay scenarios

## Summary

Capture a small early-game corpus that exercises boot, input, scripts, transitions, menus, and combat.

## Dependencies

- [Record deterministic input replays and snapshots](deterministic-replays-snapshots.md)
- [Export and compare reference frame state](reference-state-export.md)

## Requirements

- Cover boot to title and starting a new game.
- Cover movement and interaction in Crysta.
- Cover the Pandora's Box sequence.
- Cover entering the first tower, basic combat, a chest, and save/menu behavior.

## Acceptance Criteria

- Every scenario replays deterministically from documented prerequisites.
- The suite reports state hashes at named checkpoints.
- Fixtures are distributable without including prohibited content.

## Notes

- Milestone: [M1 — Reference oracle](../milestones.md#m1-reference-oracle)
- Scenario descriptions may reference events, but fixture payloads must pass the repository content policy.
- Boot-to-new-game is blocked by an SPC driver-upload handshake wedge after
  name entry: the CPU waits for an SPC echo on `$2140` while the SPC waits
  for its `$CC` chunk marker — a LakeSnes APU phase-sync gap, identical on
  JP and EU. Full trace and diagnosis:
  [Boot-flow probe findings](../../docs/oracle-boot-probes.md). Title and
  name-entry scenarios are not affected.
