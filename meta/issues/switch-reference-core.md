# Switch reference core to bsnes

## Summary

LakeSnes (upstream + dinkc64 fork) cannot execute past the post-name-entry
SPC driver-upload handshake, blocking the M1 boot-to-Crysta scenarios.
Swap the vendored reference core to bsnes (ISC) behind the unchanged
`Session` boundary.

## Dependencies

- [Create initial reference replay scenarios](initial-reference-scenarios.md)
  (its post-name-entry requirements are blocked by the LakeSnes wedge)

## Requirements

- Vendor the bsnes library core (sfc + nall + processor + emulator, no GUI)
  under `vendor/bsnes/`, licensed (ISC) per the publication policy.
- Implement a project-authored headless C shim so the oracle's `ffi`
  surface (init/load/runFrame/setButtonState/saveState/loadState plus the
  WRAM/VRAM/CGRAM/APU-RAM/CPU-PC accessors) keeps working unchanged.
- The synthetic-ROM unit tests must keep passing (or the synthetic fixture
  must be adapted to what bsnes accepts).
- Boot the Japanese dump past the wedge to Crysta; the boot-to-name-entry
  scenario suite must be re-pinned to bsnes frame counts and checkpoints.
- Record the swap in ADR 0002 and archive this issue once the acceptance
  criteria below are met.

## Acceptance Criteria

- `cargo test` for synthetic-ROM unit tests passes.
- The ROM-backed scenario suite runs against the new core, all three
  existing fixtures re-pinned to bsnes timings, and at least one scenario
  reaches past the old LakeSnes wedge (map 15 → Crysta/next room change).
- The SPC-handshake blocker no longer applies to boot scenarios.
- No LakeSnes files remain in the vendored tree; the license record lists
  the bsnes vendored sources.

## Notes

- Milestone: [M1 — Reference oracle](../milestones.md#m1-reference-oracle)
- ADR 0002's escape hatch ("the boundary stays; the core can be swapped")
  applies by design.
- Input frame timings and checkpoint state bytes are core-dependent; the
  fixtures (input streams) stay, their reference checkpoints get re-pinned.
