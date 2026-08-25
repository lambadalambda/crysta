# Disassemble boot, interrupts, and the main loop

## Summary

Annotate native-mode setup, interrupt vectors, frame synchronization, and top-level state dispatch.

## Dependencies

- [Establish a byte-matching disassembly build](matching-disassembly-build.md)
- [Select and integrate the reference emulator](select-reference-emulator.md)

## Requirements

- Track M/X width and bank assumptions across all entry points.
- Name reset, NMI, IRQ, BRK, and COP behavior.
- Identify the authoritative frame boundary and top-level dispatch.
- Document hardware initialization and direct-page setup.

## Acceptance Criteria

- Annotated source rebuilds byte-identically.
- Control-flow entry points are linked from documentation.
- A trace from reset to the first main-loop iteration agrees with the reference harness.

## Notes

- Milestone: [M2 — Matching disassembly foundation](../milestones.md#m2-matching-disassembly-foundation)
- Existing inspection found reset at `$C0:8000` in both normalized dumps.
