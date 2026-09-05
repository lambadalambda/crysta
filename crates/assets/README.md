# Asset codecs

Pure, bounded codecs for caller-owned Terranigma content. This crate does not
read files, authenticate ROMs itself, or expose an extraction CLI. Callers must
validate a user-provided image with `rom` before passing ROM-backed slices.

Currently implemented:

- [Compressed packet codec](../../docs/compression.md): bounded decoder and
  deterministic greedy encoder; unknown packet variants fail explicitly.
- [Loaded-map model](../../docs/maps.md): validated metadata, raw cell words,
  coordinate lookup, and lossless runtime layer export from caller-owned WRAM.
- [Static map layers](../../docs/static-maps.md): bounded dimension-prefixed
  containers, exact source preservation, and pure metatile-attribute lookup.
  The cavern matches actual loader output; collision movement semantics remain
  unqualified.
- [Map loading scripts](../../docs/map-scripts.md): bounded map-ID/table lookup,
  packed pointers, calls/jumps/deferred streams and lossless resource instructions.
  State-dependent branches and full runtime composition remain unsupported.

- [Static cavern graphics](../../docs/static-graphics.md): pure 4bpp tile/color/word
  primitives and a conservative map-ID-resolved cavern background recipe with
  transparency and priority preserved. Full layer sampling needs no oracle.

Broader graphics/animation, static map formats, and the local asset pack remain
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

The library uses the workspace `rom` crate (MIT) for typed address validation;
callers still own authentication and I/O. Community algorithms were studied but no unlicensed implementation was
copied; source revisions and local qualification are recorded in the format doc.
