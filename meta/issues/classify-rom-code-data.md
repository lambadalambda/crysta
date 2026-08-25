# Classify ROM code, data, and indirect dispatch

## Summary

Create a versioned ROM map that prevents data from being decoded as instructions and resolves key dispatch tables.

## Dependencies

- [Establish a byte-matching disassembly build](matching-disassembly-build.md)
- [Disassemble boot, interrupts, and the main loop](disassemble-boot-main-loop.md)
- [Build canonical RAM and hardware symbol maps](canonical-memory-symbols.md)

## Requirements

- Seed classifications from control-flow analysis and existing ROM maps.
- Track per-version normalized offsets and CPU addresses.
- Identify function pointer tables, callback records, and script entry tables.
- Attach confidence and provenance to classifications.

## Acceptance Criteria

- Every reachable boot/main-loop target is classified.
- Known indirect callback patterns resolve to valid function starts.
- The map can be consumed by the reconstruction build and inspection tools.

## Notes

- Milestone: [M2 — Matching disassembly foundation](../milestones.md#m2-matching-disassembly-foundation)
- 65C816 width state makes linear disassembly unreliable; classifications must be control-flow aware.
