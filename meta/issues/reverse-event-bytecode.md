# Reverse the event script bytecode

## Summary

Identify the event interpreter and produce a readable, lossless representation of game scripts.

## Dependencies

- [Classify ROM code, data, and indirect dispatch](classify-rom-code-data.md)
- [Disassemble boot, interrupts, and the main loop](disassemble-boot-main-loop.md)

## Requirements

- Locate opcode dispatch, operand decoding, call/return behavior, waits, flags, dialogue, movement, and map transitions.
- Track control-flow targets and script entry points.
- Create a disassembler with stable symbolic output.
- Add an assembler or round-trip encoder after the format is understood.

## Acceptance Criteria

- The Crysta and Pandora sequences disassemble without unknown-length instructions.
- Control-flow targets are validated and symbolized.
- Supported scripts round-trip byte-exactly or document known relocation behavior.

## Notes

- Milestone: [M3 — Content and script pipeline](../milestones.md#m3-content-and-script-pipeline)
- Treat scripts as a first-class language rather than translating each event directly from raw bytes.
