# Implement the portable event runtime

## Summary

Execute decoded event scripts through semantic game services in the Rust core.

## Dependencies

- [Define the deterministic portable core model](deterministic-core-model.md)
- [Reverse the event script bytecode](reverse-event-bytecode.md)

## Requirements

- Implement control flow, flags, waits, dialogue requests, actor movement, and map transitions needed by the opening.
- Validate operands and execution budgets.
- Make script state serializable and deterministic.
- Expose semantic commands rather than SNES memory writes.

## Acceptance Criteria

- Crysta and Pandora scripts reach the same named checkpoints as reference execution.
- Tests cover calls, branches, waits, invalid opcodes, and runaway scripts.
- Script snapshots resume deterministically.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice)
- Opcode semantics should link back to disassembly evidence.
