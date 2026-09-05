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
