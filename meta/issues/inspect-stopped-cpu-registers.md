# Inspect stopped CPU registers for gameplay qualification

## Summary

Expose a read-only CPU register snapshot at oracle trace stops so event and
collision research can identify actual entities and return values without guessing.

## Dependencies

- [Disassemble boot, interrupts, and the main loop](disassemble-boot-main-loop.md)

## Requirements

- Expose PC, A, X, Y, S, D, P, DBR and emulation mode without changing execution.
- Preserve the existing trace record ABI and trace digest version.
- Keep this reference-only API out of the portable simulation.

## Acceptance Criteria

- A ROM-free synthetic program establishes and verifies known register values at
  an instruction stop, with a repeated read proving no side effects.
- Layout assertions protect the FFI boundary; oracle tests and strict lint pass.

## Notes

- Supports [transition qualification](qualify-opening-room-transition.md) and
  [portable movement qualification](portable-room-slice.md).
