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
