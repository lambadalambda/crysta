# ROM Revision and Version-Support Model

Status: decided 2026-08-25. This is an architecture decision record for
[the version-support issue](../meta/issues/define-version-support-model.md).

## Decision

| Aspect | Decision |
| --- | --- |
| Behavior reference (M1–M6 differential tests) | **Japan** (`Tenchi Souzou`, normalized SHA-256 `f331e394…344548`) |
| Localization reference | **Europe English** (`Terranigma`, normalized SHA-256 `93ba50d8…3edd38`) |
| European executable behavior before first release | Not supported; extraction of its *content* (text) is supported |
| Cross-version behavior support | Deferred to the localization issue in M8 |

## Version identity model

Three orthogonal identities, never conflated:

1. **Source revision** — which normalized cartridge dump content came from
   (`Revision` in `crates/rom`). Drives extraction and localization.
2. **Asset schema version** — the format/semantic version of decoded
   content (`schema_version`, starting at `1`). Bumped when decoded
   representations change meaningfully. Asset packs record both source
   revision and schema version in their manifest.
3. **Snapshot/save version** — the version of portable `GameState`
   serialization (`snapshot_version`, starting at `1`). Independent of asset
   schema; a snapshot records the schema and source revision it was produced
   against and refuses to load on mismatch.

## Cross-version correspondence

Japanese and European ROM layouts differ (different text engines, shifted
data). Correspondences are recorded as **symbolic semantic IDs** (e.g. map
IDs, event IDs, item IDs) rather than raw addresses once decoded; raw
per-version address tables live with the disassembly and are keyed by
revision. Nothing assumes the two revisions share layout.

## Rejection path

- Unknown ROM digests: rejected at load (`rom::Rom::load` →
  `UnknownRevision`), with the computed hashes in the error for reporting.
- Asset packs and snapshots record `source_revision` + `schema_version` /
  `snapshot_version`; mismatches are rejected with an error naming both
  sides, never silently coerced.

## Consequences

- All M1 reference traces, M2 symbol maps, and M4–M6 differential tests
  target the Japanese executable. One lane, one truth.
- The European dump's role until M8 is text/localization extraction only.
- `rom-free-ci` must not assume the European dump is present in any
  behavior test.
