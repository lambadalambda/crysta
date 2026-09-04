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
- Trace artifacts follow the commit-safe/local-only classes in
  [CONTRIBUTING](../../CONTRIBUTING.md): only structured format metadata,
  checkpoints, counts, and hashes are committed; raw traces remain local.

## Verification

Verified locally on 2026-08-27 against the authenticated Japanese dump and the
ares reference harness:

- The annotated reconstruction matches all 4 MiB and the selected reference
  SHA-256, `f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548`.
- A bounded pre-instruction trace reaches the first main-loop instruction at
  runtime `$80:8043` after 965,059 records and 64 frame events.
- Two clean-process trace runs produced the same version-1 structured digest,
  `8a6db5e98dac5f7b6e085a7509f4259dafa283ab3df4b929fda76d21d7b09df9`.
- The ROM-backed integration test checks the reset prefix, processor mode
  transition, final PC, count, frame count, and full trace digest.
- Full control-flow findings and links to source and tests are recorded in
  [Boot, interrupts, and main loop](../../docs/boot-main-loop.md).
