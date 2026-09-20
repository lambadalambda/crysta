# Restore the formatting gate under current stable rustfmt

## Summary

`cargo fmt --all -- --check` fails on the committed tree. The workspace has
drifted from any locally reproducible rustfmt output, so the M0 exit criterion
"the Rust workspace passes formatting, lint, and test gates" is currently
unmet for formatting only. Lint, test and tracker gates still pass.

## Dependencies

- [Bootstrap the Rust workspace and quality gates](bootstrap-rust-workspace.md)

## Requirements

- Reformat the sources that no rustfmt reproduces, without changing behavior.
- Do not silently rewrite a provenance-pinned qualification producer source.
  Byte identity of pinned files is authenticated by
  `crates/map-inspector/tests/local_capture.rs`; a reformat is a pin change.
- Make the remaining gap explicit and bounded rather than disabling the gate,
  weakening the pin test, or reformatting under an unreviewed repin.
- Record the toolchain observation so the drift does not silently return.

## Acceptance Criteria

- `cargo fmt --all -- --check` passes on the default toolchain.
- `cargo clippy --workspace --all-targets -- -D warnings` and
  `cargo test --workspace` still pass, including the producer pin tests.
- Any file excluded from the reformat has a documented reason and its own
  tracked follow-up.

## Notes

- Milestone: [M0 — Safe foundation](../milestones.md#m0-safe-foundation)
- This is a repository hygiene repair, not new capability work.

## Measurements

- 12 diffs across 9 files on the committed tree. Both locally available
  rustfmt versions disagree with the tree identically: `1.8.0-stable`
  (toolchain 1.90.0) and `1.9.0-stable` (toolchains 1.96.0/1.97.0/1.97.1).
  The drift is therefore old, not a recent toolchain regression.
- No configuration reproduces the committed bytes. Explicit
  `style_edition = "2015" | "2018" | "2021"` all leave the same 12 diffs, and
  `"2024"` raises it to 183. The `ignore` option cannot exclude a file either:
  it is nightly-only and current stable warns and discards it.
- `rust-toolchain.toml` floats on `channel = "stable"`, which is structurally
  incompatible with byte-pinned producer sources. Pinning a version does not
  help here because no available rustfmt matches the tree.
- Eight of the nine affected files are unpinned and safe to reformat.

## Remaining gap — closed

`crates/map-inspector/src/main.rs` was a pinned library-producer source. Its
only diff is a `mod house_progression;` / `mod house_profiles;` declaration
reorder, which is behaviorally inert but still a byte change, and
`check_library_sources` failed with "library producer source changed; explicitly
revalidate before repinning". Repinning required the ROM-backed revalidation and
independent review that
[the library producer revalidation](revalidate-preview-library-producer.md)
established, so it was not folded into a formatting commit. It was tracked
separately and has since been done by
[Repin the formatted library producer source](repin-formatted-producer-source.md).

`cargo fmt --all -- --check` now passes on the committed tree with **no excluded
files**, so every criterion above is met and the M0 formatting exit criterion is
restored.
