# Support the bounded Pandora event flag projection

## Summary

The qualified Pandora route writes `$243`, `$244` and `$292`, beyond the existing house runner’s512-bit event block. Add a typed1024-bit semantic flag/sequence variant while preserving the existing house API, validation and snapshot format.

## Dependencies

- [Qualify the native Pandora route and state changes](qualify-pandora-route.md)

## Requirements

- Reuse the current bounded event runner rather than duplicating its page/choice semantics or adding a general event VM.
- Preserve the existing512-bit variant and its out-of-range rejections. Wider storage is explicitly selected by the new caller, not silently admitted by old house data.
- Store and expose the selected semantic bit projection in canonical low-bit-first bytes; this is not a claim that all1024 native positions are global event flags or a valid raw WRAM initializer.
- No live preview or slice snapshot change until the parent source/runtime integration is accepted.

## Acceptance Criteria

- Red → green tests show higher flags remain unset before acknowledgement, are set at the proper page return, and survive canonical byte reconstruction and continuation.
- Out-of-range flags and wrong actions are rejected atomically for each variant; existing512-bit house tests remain unchanged and pass.
- Core remains dependency-free/no_std; native/Wasm tests/builds and independent correctness/architecture review pass.

## Notes

- Parent: [Port the bounded Pandora route and sequence](port-pandora-sequence.md).
- Room-local reset and stage-specific valid flag combinations remain the owning game state’s responsibility. No original CPU execution is added.


## Accepted component result

- Existing64-byte aliases now share a typed implementation with explicit128-byte story aliases. No GameState/profile9/snapshot/preview behavior is changed.
- Red: new story-API imports failed before implementation. Green: four focused projection/acknowledgement/choice/boundary tests and one compile-fail mismatched-storage test pass. Existing house512-bit tests are unchanged.
- Required core fixture suite, strict core Clippy, Wasm build, native/Wasm shared-probe Rust tests and owned-ROM house-identity/conversation-through-exterior regressions pass. Independent correctness/architecture review found no blockers.
- The generic zero/one-byte extent test suggested by review is optional and outside the two admitted aliases; deferred rather than expanding this component task.
- This closes only the reusable storage/runner component. The independent Pandora source replay discrepancy and live story integration remain separate open gates.
