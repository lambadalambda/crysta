# Revalidate the preview producer after library separation

## Summary

Authenticate the host-free library separation as a same-output native preview producer change without renewing any frozen observation or output pin.

## Dependencies

- [Expose the Pandora preview as a host-free Wasm library](expose-pandora-preview-wasm-library.md)

## Requirements

- Extend the current-producer inventory explicitly for the library/static-render boundary and update only reviewed source identities.
- Reproduce the existing normal/optimized producer bridge and fresh singleton capture checks against all ten accepted files and complete manifests.
- Preserve `observer.json`, `migration.json`, archived epochs, capture schedule, renderer/state outputs, and the exact authenticated `main.rs` registration proof.
- Decide during that revalidation whether the native host can consume `PandoraPreview` directly without weakening the historical main-source gate.

## Acceptance Criteria

- Independent and parent reproductions prove identical accepted outputs from the separated producer.
- The strict current-source gate passes with explicit complete inventories and its mutation controls remain effective.
- No ROM, SRAM, extracted assets, captures, or binaries are committed.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice)
- The library stage intentionally leaves a focused source-identity mismatch rather than repinning without producer evidence.

## Initial tracked impact

The frozen/current gate remains effective and is expected to reject exactly two
changed pinned identities until this issue is qualified:

- `crates/map-inspector/Cargo.toml`: expected
  `0ccdd13a83c94c72934d48814d5d4a4fa86b50f2f36a4afd54c912be8981f0a5`,
  separated source currently
  `a680042987b0cf56a28ffe8e2d631067984c5f0346e0ec6ae3e448e445ffc072`;
- `crates/map-inspector/src/room_preview.rs`: expected
  `885c2413fc9c2d7495b853381028a45a86e8ec640c1a42953d421b3de3b85594`,
  separated source currently
  `909a84fd2d83eac2373d6f6314b9c8cdf97c9c8cdd52ab6fb29a51540d00fdb3`.

`src/lib.rs`, `src/static_background.rs`, and `src/visual_export.rs` also need
explicit inventory treatment because the native exporter now reaches the pure
renderer through the library. `main.rs`, `room_server.rs`, and every frozen
observer/migration/epoch/output file are unchanged. No descriptor was repinned.

## Browser-adapter impact

The browser-local adapter additionally changes two pinned identities:

- `Cargo.lock`: expected
  `fa9a478c4abd6e3b147774669c862519c2205116c522e81cf1c2fd8f1254d537`,
  adapter source currently
  `23e335de25473b02230aae706ef19db003159c92dd1f3583303234d8134bd070`;
- `crates/map-inspector/web/room-slice.html`: expected
  `d4103bcfee965378ced16ee9bc74b2c04f0b1a7794e4802a877cb4c354744271`,
  injected-seam source currently
  `1dab2e1895a7db842d69188931e784d15f52c8d51758bc019b9d16bbf6225fad`.

The final inventory must also bind root `Cargo.toml`, `crates/pandora-web/`, and
`tools/pandora-preview/`. The native fallback request paths, `main.rs`,
`room_server.rs`, and all frozen evidence remain unchanged. The full browser
journey now passes through the settled adapter; only the same-output producer
proof and source-bridge inventory remain deliberately deferred. This section
records source identities, not producer acceptance.
