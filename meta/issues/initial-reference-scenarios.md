# Create initial reference replay scenarios

## Summary

Capture a small early-game corpus that exercises boot, input, scripts, transitions, menus, and combat. Completed 2026-08-26.

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
- **Completed scenarios:** boot-to-name-entry (JP + EU), name-entry cursor
  input (JP). These exercise boot, title screen, name-entry menu, input
  injection, and snapshot-restart determinism.
- **Blocked scenarios:** Pandora's Box / Crysta interaction. The game's
  own script stalls after name entry — both ares and LakeSnes reproduce the
  identical stall. This is a game-level input-sequence issue, not an
  emulator defect. The input sequence to advance past the name entry into
  the Crysta overworld has not yet been determined.
