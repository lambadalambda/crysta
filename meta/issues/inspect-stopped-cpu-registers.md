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

## Completion

Implemented `Session::cpu_registers` with a layout-checked read-only snapshot
call. Synthetic red-green test establishes every native register, matches trace
endpoint fields, proves repeated reads leave frame/WRAM/PC unchanged and confirms
the stopped store has not executed. Existing trace ABI/digests are unchanged.
Independent review found no blockers; oracle/workspace tests, strict Clippy,
formatting and rustdoc passed in the ROM-free implementation worktree.
