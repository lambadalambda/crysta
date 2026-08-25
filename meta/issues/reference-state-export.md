# Export and compare reference frame state

## Summary

Capture the state needed to diagnose behavioral divergence between the original game and portable code.

## Dependencies

- [Record deterministic input replays and snapshots](deterministic-replays-snapshots.md)

## Requirements

- Export selected WRAM and SRAM ranges plus CPU, VRAM, CGRAM, OAM, framebuffer, and audio hashes where available.
- Support named semantic fields as the symbol map grows.
- Produce a concise first-divergence report.
- Allow documented exclusions for unstable irrelevant bytes.

## Acceptance Criteria

- A synthetic mismatch test reports the exact frame and field or range.
- Exports are deterministic for an existing replay.
- The comparison format is versioned.

## Notes

- Milestone: [M1 — Reference oracle](../milestones.md#m1-reference-oracle)
- Start broad during discovery, then compare semantic state to avoid coupling forever to scratch memory.
