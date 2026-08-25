# Contributing

## License

Original project code is under the [MIT License](LICENSE). By contributing,
you license your contributions under the same terms.

MIT covers only original work in this repository. ROM-derived content remains
the property of its rights holders:

- **Commit-safe:** input logs, schemas, hashes, symbol metadata, synthetic
  fixtures, documentation, and fully labeled reconstructed assembly/data
  representing original reverse-engineering annotation in the style of
  community matching-decompilation projects.
- **Local only (`local/`, ignored):** cartridge dumps, emulator save states,
  raw memory/video/audio exports, extracted content, reconstructed binaries,
  and generated asset packs.
- **Review required:** minimized derived fixtures or reconstructed source
  whose copyright/provenance status is not already covered by the classes
  above.

These classes are a project policy, not legal advice.

## Third-party dependencies

Original code is MIT, so permissively licensed dependencies (MIT, BSD,
Apache-2.0, ISC, Zlib) may be used freely with attribution. Copyleft
dependencies (GPL, LGPL, AGPL) must not be linked into workspace crates;
if one is genuinely required later, isolate it behind an optional feature
or separate tool and record the decision before it is added. Specifically:

- **SNESRecomp is PolyForm Noncommercial.** It may be studied and run locally
  for analysis, but its code must never be copied into this repository or its
  build.
- Emulator cores and SPC700/DSP implementations carry their own licenses;
  check compatibility before vendoring or linking, and record provenance in
  the crate that uses them.

When adding any dependency, note its license next to the dependency or in the
crate README so the next reviewer does not have to rediscover it.

## Before starting

1. Read [`README.md`](README.md) and
   [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).
2. Select an open issue from [`meta/issues.md`](meta/issues.md).
3. Read its detail file and confirm that dependencies and acceptance criteria
   are understood.
4. Keep changes focused on that issue.

New work must have an issue detail file under `meta/issues/` and one link in the
open issue index. See the format used by existing issues.

## Test-driven development

Use red-green-refactor for executable behavior when feasible:

1. Add the smallest failing unit, round-trip, replay, or differential test.
2. Implement only enough behavior to pass it.
3. Refactor while preserving a green test suite.

Pure research and documentation cannot always begin with a failing test. In
those cases, record the reproducible evidence and add automated structural or
fixture validation where practical.

## ROM and asset safety

Never commit:

- cartridge dumps or patched ROMs;
- extracted dialogue, maps, graphics, samples, or music;
- generated asset packs;
- emulator save states containing copyrighted ROM data unless their format and
  distribution have been reviewed explicitly.

Tools should accept a user-provided ROM, verify its hash, and generate local
ignored output under `local/`. Raw snapshots, memory/video/audio captures, and
other ROM-backed oracle artifacts belong there too. Public tests and CI must
not require copyrighted input. Optional ROM-backed integration tests should
skip clearly when the expected local ROM is not configured.

Before every commit, run:

```sh
git status --short --ignored
```

and verify that local ROMs remain ignored.

## Fidelity and enhancements

Reference behavior belongs in classic mode. Bug fixes and enhancements must be
opt-in and must not silently change the state used by classic differential
tests. Keep simulation deterministic and independent from wall-clock time,
rendering APIs, audio devices, and host filesystems.

## Issue completion

An issue is complete only after its acceptance criteria have been verified.
Move its index entry from `meta/issues.md` to `meta/issues_archive.md`, changing
`[ ]` to `[x]`; leave the detail file in place.

## Toolchain and quality gates

The repository pins the current stable toolchain in
[`rust-toolchain.toml`](rust-toolchain.toml); any Rust version capable of
building the workspace may be used locally, but CI uses the pinned channel.
Run the full gate suite before committing:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python3 tools/check_repo_safety.py
python3 tools/check_tracker.py
```

Clippy is configured at `warn` level in the workspace manifest for
incremental development, but commits must pass `-D warnings` as shown above;
CI enforces the same.

### Local ROM-backed tests

Tests that need real dumps read them from the git-ignored `local/`
directory (e.g. `local/Tenchi Souzou (Japan).sfc`) and **skip with a clear
message when absent** — `cargo test --workspace` stays fully green on a
clean checkout and in CI. To run them locally, copy your dumps into
`local/` first.

## Commits

Use small topical commits and Conventional Commit messages, for example:

```text
feat(rom): validate known cartridge hashes
test(assets): cover compression round trips
docs: map the native interrupt vectors
```
