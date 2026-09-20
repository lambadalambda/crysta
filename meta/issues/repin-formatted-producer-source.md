# Repin the formatted library producer source

## Summary

`crates/map-inspector/src/main.rs` is the one formatting-gate holdout. It is a
pinned library-producer source, so reformatting it invalidates the descriptor
hashes authenticated by `crates/map-inspector/tests/local_capture.rs`. Apply the
reformat together with an evidenced repin instead of silently rewriting the pin.

## Dependencies

- [Restore the formatting gate under current stable rustfmt](restore-format-gate.md)
- [Revalidate the preview producer after library separation](revalidate-preview-library-producer.md)

## Requirements

- Apply only the rustfmt-required change: the `mod house_progression;` /
  `mod house_profiles;` declaration reorder. No other edit rides along.
- Demonstrate the change is output-neutral by producing fixture output with the
  reformatted producer and comparing it to the current pinned expectation.
- Update the producer descriptor and bridge hashes through the established
  revalidation path, not by pasting a new hash until the test turns green.
- Keep `check_main_registration`'s anchored-insertion control and the mutation
  controls in `non_registration_main_changes_are_rejected` strict.
- Obtain an independent review of the evidence before the repin commit.

## Acceptance Criteria

- `cargo fmt --all -- --check` passes with no excluded files.
- `cargo test --workspace` passes, including `fixture_library_producer_sources_match`
  and `non_registration_main_changes_are_rejected`.
- The repin records that producer output is unchanged, with the comparison
  retained under ignored `local/`.
- Negative controls still fail when the pin or registration guards are disabled.

## Notes

- Milestone: [M0 — Safe foundation](../milestones.md#m0-safe-foundation)
- Needs the owned Japanese ROM; it cannot be verified on a clean checkout.
- A behaviorally inert diff is still a provenance change. The point of the pin is
  that nobody gets to decide a producer edit was harmless without evidence.

## Qualification

Done. `cargo fmt --all -- --check` passes with no excluded files.

**Recorded as a third stage, not a rewritten pin.** The library descriptor and
report stay byte-frozen at `00298d93…6143` and `18cfd3ec…0b75d`; a new
`repin-producer.json` records exactly one replacement over them
(`2f77608e…4e2d` → `5eeb11bd…73ed`), with `repin-producer-bridge.json` as its
evidence report. `observer.json`, `migration.json`, `current-producer.json`,
`producer-bridge.json` and `epochs/` are untouched.

**Only the rustfmt change was applied.** The reformatted `main.rs` still
reconstructs byte-exactly to `7736b543…a4d3` after undoing two exact anchored
deltas — the reorder and the unchanged 57-byte registration insertion. Each
delta must occur once in the current file and each anchor once in the
authenticated old blob, which makes the reconstruction injective: the accepted
set is a singleton, so no other `main.rs` passes.

**Output neutrality.** Two fresh isolated builds of the reformatted producer
(`local/map-inspector-repin/{repin-a,repin-b}`, own clean target, build log,
copied binary, one singleton capture each) reproduce all ten capture files at
855,207 bytes, byte-identical to each other and to the frozen inventory,
complete manifest `a7f23508…664a` and canonical nonpixels `7998be25…0cb22`.
The report is emitted by `repin_bridge.py` and reproduces byte-exactly in
normal and `-O` Python.

**Verification.** `cargo fmt --all -- --check`, `cargo clippy --workspace
--all-targets -- -D warnings` and `cargo test --workspace` pass (548 tests, 0
failures). The eight `local_capture` tests pass with the owned ROM present and
none skipped. All eight qualification Python suites pass in normal and `-O`.

**Negative controls.** `test_repin_mutations.py` disables each of the nine
repin gates in turn and requires a red test in both Python modes; 18/18
detected. Records retained under ignored `local/map-inspector-repin/`.

**Review.** An independent review found the first draft's defect: it had
rewritten the library report's header while leaving its producer envelopes
describing the pre-reformat builds, and no gate caught it. That draft was
discarded for this stage design, and `check_repin_report` now requires every
retained envelope to bind to the repin descriptor. The same review confirmed by
brute force over all anchor sites that the gate's accepted set is a singleton.

## Bounded gap

The full five-root `library_bridge.py` audit cannot be re-run by anyone. Its
accepted roots lived under ignored `local/` in deleted worktrees, and
`migration.json` pins their `binary_sha256` and build-log identities from builds
the qualification README itself records as not bit-reproducible. This predates
and is independent of the repin. This stage therefore compares against the
frozen library report's inventory, which carries per-file sizes and SHA-256, so
matching it is byte equality with the accepted output. The source worktrees were
restored from Git (`7c5c90b`, `ad0c049`) and the descriptor chain reproduces
against them unmocked.

**Second review.** An independent review of this stage reproduced the report
byte-identically from the retained roots, confirmed all six earlier pins
byte-unchanged, and brute-forced the entire preimage space of the two-delta
reconstruction (12,217 candidates over every insertion × reorder site, plus six
targeted semantic attacks): only the real file passes. It found the recorder's
repin branch untested and one gate whose control was satisfiable by a fallback
error message; both are fixed here, taking the covered gate count from eight to
nine. Remaining suggestions are tracked as
[Consolidate the qualification stage template](consolidate-stage-template.md).
