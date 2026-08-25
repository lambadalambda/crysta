# Record first ROM-backed verification run

## Summary

Run the full local verification against the actual cartridge dumps and record the results, closing the loop on M0's ROM-backed exit criteria that ROM-free CI cannot verify.

## Dependencies

- [Establish ROM-free continuous integration](rom-free-ci.md)

## Requirements

- Load both local dumps through the public loader and record: normalized size, detected revision, computed SHA-256 and CRC32.
- Verify the `internal_title` constants (`TENCHI-JPN`, `TERRANIGMA P`) actually appear at offset `0xFFC0` of each normalized image.
- Verify the European dump's copier-header claim end to end (raw size 4 MiB + 512).
- Record results in a short note under `docs/` (hashes and titles only; no ROM content).

## Acceptance Criteria

- A documented record exists of both dumps passing `cargo test -p rom --test local_roms` with no skips.
- Any discrepancy in internal titles or header layout becomes either a fix or a documented correction.
- The M0 exit criterion "both known dumps are normalized and validated" is verifiably met.

## Notes

- Milestone: [M0 — Safe foundation](../milestones.md#m0-safe-foundation)
- The optional local tests currently pass on the author's machine, but results were not recorded in-repo; this makes that verification durable and reviewable.
