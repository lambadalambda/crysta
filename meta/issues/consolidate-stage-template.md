# Consolidate the qualification stage template

## Summary

The map-inspector qualification evidence chain is now four frozen stages deep
(`current-producer` → `library-producer` → `repin-producer` →
`oracle-repin-producer`), and the per-stage bridge modules are roughly 85%
copy-paste. `oracle_repin_bridge.py` was added in the same shape on purpose, so
the provenance commit stays reviewable; consolidate now. Consolidate the template and close
the coverage gaps an independent review found in the newest stage.

## Dependencies

- [Repin the formatted library producer source](repin-formatted-producer-source.md)

## Requirements

- Introduce a stage record in `bridge.py` carrying kind, descriptor/producer
  schema versions, descriptor fields, replaced files, extra group inventories,
  predecessor descriptor/bridge hashes and the main-delta function, so
  `library_bridge` and `repin_bridge` reduce to data plus their genuinely
  distinct parts. `verify_producer_envelope` is already the first slice.
- Mirror it in `crates/map-inspector/tests/local_capture.rs`. The successor
  stages already share `check_stage_identity` over an ordered `STAGES` list
  (done with the oracle repin); `check_library_identity` is still separate.
- Move the test-only `oracle_repin_bridge.historical_bytes` into a test helper.
- Add hermetic controls for `repin_bridge.compare_to_frozen`, which currently
  has six `require`s and no tests. Its predecessor `bridge.compare_unchanged`
  has a control class in `test_bridge.py`, including a symlink-aliasing test for
  the distinct-roots gate; that coverage regressed.
- Anchor `compare_to_frozen` on `migration.json`'s frozen `fixed` inventory via
  `check.authenticate` rather than transitively through the library report. The
  two are identical today, so this is directness, not added strength.
- Remove the replacement-current-source check subsumed by the later
  unchanged-source comparison, or give it a case the other cannot reach.

## Acceptance Criteria

- A new stage can be added by declaring data, without copying a module.
- Every `require` in the stage modules is covered by a mutation entry that goes
  red in normal and `-O` Python, or is documented as an assertion about a
  hash-pinned constant.
- No frozen descriptor, report, epoch or output pin changes.
- `cargo test --workspace` and every qualification Python suite still pass in
  both modes.

## Notes

- Milestone: [M0 — Safe foundation](../milestones.md#m0-safe-foundation)
- Raised by the independent review of
  [Repin the formatted library producer source](repin-formatted-producer-source.md).
  Deferred deliberately: refactoring provenance-adjacent code in the same commit
  as a provenance pin would have obscured what the pin actually changed.
- This is maintainability and coverage work. The pin itself was reviewed as
  sound, with the accepted set of `main.rs` proved to be a singleton.
- Also noted, minor: two distinct `compare_to_frozen` failures share the message
  `complete manifest changed`; the byte-compare loop there is unreachable as a
  failure given the preceding inventory equality; `frozen_library()` and
  `frozen_predecessor()` re-read and re-hash on every call; and
  `repin_bridge.PRODUCER_FIELDS` derives from `library_bridge.PRODUCER_FIELDS`,
  so editing that unpinned module silently changes the repin contract.
- From the oracle repin (2026-09-29): the mutation harnesses look for one
  `FAIL:` line, so an error elsewhere could pass unseen (the older harnesses
  too); and `vendor/ares/ares/sfc/cpu/memory.cpp`, which the European work
  changed, is not in the pinned inventory (the fresh builds covered it).
