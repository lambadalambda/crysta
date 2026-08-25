# First ROM-Backed Verification Run

Recorded 2026-08-25, per [the tracking issue](../meta/issues/record-first-rom-backed-run.md).
All results from `cargo test -p rom --test local_roms` on the author's dumps
in `local/` — 3 passed, 0 skipped.

| Check | Tenchi Souzou (Japan) | Terranigma (E) |
| --- | --- | --- |
| Raw size | 4,194,304 (headerless) | 4,194,816 (4 MiB + 512-byte copier header) |
| Normalized size | 4,194,304 | 4,194,304 |
| SHA-256 | `f331e394…344548` ✓ | `93ba50d8…3edd38` ✓ |
| CRC32 | `3CC7FDF4` ✓ | `974523FF` ✓ |
| Internal title @ `0xFFC0` | `TENCHI-JPN` ✓ | `TERRANIGMA P` ✓ |

Corrupted-image rejection also verified (single flipped byte produces
`UnknownRevision` carrying the computed digests).

Conclusions:

- The copier-header claim for the European dump is confirmed end to end.
- The `internal_title` constants are confirmed against real headers; the
  21-byte SNES title field is space-padded, and these 11/12-byte prefixes
  match the actual bytes.
- M0's exit criterion "both known dumps are normalized and validated without
  modifying them" is met.
