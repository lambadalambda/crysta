# Build canonical RAM and hardware symbol maps

## Summary

Replace anonymous addresses with evidence-backed names and reusable typed metadata.

## Dependencies

- [Disassemble boot, interrupts, and the main loop](disassemble-boot-main-loop.md)
- [Export and compare reference frame state](reference-state-export.md)

## Requirements

- Import and verify existing public RAM-map findings.
- Record width, lifetime, aliases, and confidence for each symbol.
- Distinguish hardware registers, direct-page state, WRAM, SRAM, and buffers.
- Generate formats consumable by disassembly, tracing, and Rust tooling.

## Acceptance Criteria

- Known player coordinates, current map, inventory, and event flags are verified by traces.
- Conflicting aliases are documented rather than silently merged.
- Generated symbol outputs are reproducible.

## Notes

- Milestone: [M2 — Matching disassembly foundation](../milestones.md#m2-matching-disassembly-foundation)
- Treat community maps as leads, not ground truth.
