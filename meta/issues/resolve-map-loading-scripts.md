# Resolve map loading scripts and qualify additional layers

## Summary

Resolve Japanese map IDs through the loader's script tables and identify static
layer resources without a hard-coded cavern offset. Qualify additional map data
against the reference and explicitly reject unimplemented script behavior.

## Dependencies

- [Decode map, metadata, and collision formats](decode-map-collision-formats.md) (parent)
- [Qualify static map loading and decode the cavern layer](static-map-cavern.md)

## Requirements

- Establish map-script entry indexing, pointer conversion, command operand lengths
  and relevant call/return/control behavior from source and oracle evidence.
- Provide a pure bounded representation and resolver for the supported loading
  script subset, retaining instruction/source provenance and exact raw operands.
- Reject unknown commands, unsupported state-dependent behavior, invalid pointers,
  out-of-range IDs and runaway control flow without guessing or unbounded work.
- Replace the cavern's fixed layer offset in an inspection path with map-ID resolution.
- Exercise additional authentic map resources and record the strength of qualification
  separately from general map/collision or event-interpreter support.

## Acceptance Criteria

- Synthetic tests cover pointer/operand bounds, control flow and budget failures.
- The cavern resolves to its already-qualified layer through the map ID and script.
- Additional map resources decode with pinned extents/hashes and at least one
  additional layer agrees with actual loader execution.
- A local CLI emits readable script/resource provenance from an authenticated ROM.

## Notes

- Milestone: [M3 — Content and script pipeline](../milestones.md#m3-content-and-script-pipeline)
- Loading scripts are not automatically the gameplay event bytecode. This issue
  does not close [Reverse the event script bytecode](reverse-event-bytecode.md).
- Original bytes and raw traces remain under ignored `local/`.

## Completion record

- Added a pure `assets::maps::scripts` projection with bounded map/subscript tables,
  typed ROM addresses, packed pointer conversion, resource instruction framing,
  calls, jumps, flagged unwinds, deferred streams and returns. Raw instructions
  retain exact bytes and runtime mirrors.
- Unknown/noncanonical opcodes, game-flag-dependent FD, F9 bit 14, bad tables or
  pointers, truncated/bank-crossing streams, loops and recursion fail explicitly.
  Audio/display effects and cached/partial layer composition are not simulated.
- Twelve synthetic tests cover command families, control and pointer state, exact
  bounds and budgets. The absent-resource-start regression was red before the fix.
- Authenticated fixtures resolve five map IDs to eight static layers and pin their
  entry pointers, instruction counts, extents, dimensions and output hashes.
- Added `resolve-map` JSON inspection by hexadecimal map ID, without SRAM or
  emulator boot. It reports ordered source loads, not a final composed map.
- The loader experiment now resolves cavern resources by ID instead of fixed
  offsets. It also proves all 1,536 raw layer bytes for map `$0004` during save
  selection. Cavern raw/initialized equality and final single-cell difference
  remain intact. These tests actually ran with owned ROM/SRAM inputs.
- Independent correctness/architecture reviews approved the resolver and tooling.
  Full workspace tests, formatting, Clippy and rustdoc (warnings denied) passed.
  A clean detached worktree also passed the full suite with explicit local-input
  skips. Documentation received an independent review with no findings.
- [Loading-script format and qualification](../../docs/map-scripts.md) records
  exact supported behavior, source provenance, CLI commands and evidence levels.

Map `$0004` is a menu scene, not another gameplay room. Maps `$0024`, `$0025` and
`$0263` have static-only qualification. Arbitrary save slots, room transitions,
collision semantics and the gameplay event VM are not claimed. Parent issues
remain open for those requirements and state-dependent loading/composition.
