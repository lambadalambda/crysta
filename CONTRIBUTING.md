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

## Commits

Use small topical commits and Conventional Commit messages, for example:

```text
feat(rom): validate known cartridge hashes
test(assets): cover compression round trips
docs: map the native interrupt vectors
```
