# Asset codecs

Pure, bounded codecs for caller-owned Terranigma content. This crate does not
read files, authenticate ROMs itself, or expose an extraction CLI. Callers must
validate a user-provided image with `rom` before passing ROM-backed slices.

Currently implemented:

- [Compressed packet codec](../../docs/compression.md): bounded decoder and
  deterministic greedy encoder; unknown packet variants fail explicitly.
- [Loaded-map model](../../docs/maps.md): validated metadata, raw cell words,
  coordinate lookup, and lossless runtime layer export from caller-owned WRAM.
  Static ROM map decoding and collision semantics remain unqualified.

Graphics interpretation, static map formats, and the local asset pack remain
separate M3 work. The [map inspector](../map-inspector/README.md) supplies local
oracle capture and browser visualization without adding I/O to this library.

```sh
cargo test -p assets
```

Synthetic tests are ROM-free. Optional local tests authenticate JP/EU dumps
under `local/`, then verify representative graphics and map packet boundaries,
sizes, independent hashes, and byte-exact re-encoding. They skip explicitly when
dumps are absent.

## Dependencies and provenance

The library has no third-party dependencies. Tests use the workspace `rom` crate
(MIT). Community algorithms were studied but no unlicensed implementation was
copied; source revisions and local qualification are recorded in the format doc.
