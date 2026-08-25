# Harden the oracle session boundary

## Summary

Close the smaller gaps found in review of the oracle crate: the `new()` leak/mislabeled-error path, Send-soundness documentation and enforcement, and test coverage edges.

## Dependencies

- [Select and integrate the reference emulator](select-reference-emulator.md)

## Requirements

- Free the core if the i32 length guard in `Session::new` fails, and represent that failure distinctly from `CoreRejectedRom` (or prove it unreachable for validated ROMs and remove the path).
- Reword the `Send` justification (no mutable global state + unique ownership, not "no threads"), and add a CI check that fails on non-`const` file-scope statics in `vendor/lakesnes/*.c` so the invariant is pinned.
- Document or remove the redundant `snes_reset(true)` after `snes_loadRom`.
- Add a `#[should_panic]` test for `wram(0x20000)`.

## Acceptance Criteria

- No path from `Session::new` leaks the core allocation.
- The statics check runs in CI and passes on the vendored tree.
- The panic-bound test exists and passes.

## Notes

- Milestone: [M1 — Reference oracle](../milestones.md#m1-reference-oracle)
- Found in review of commit `4aab861`; deferred here to keep the emulator-selection commit focused.
