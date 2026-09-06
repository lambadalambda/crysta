# Revalidate the preview producer without renewing observation pins

## Summary

Registering ROM-only preview modules changes the whole-file map-inspector producer source pin. Revalidate the new producer within headless-sync-video-v1 while preserving the original observer migration evidence and all output pins.

## Dependencies

- [Render bounded Pandora world patches](render-pandora-world-patches.md)
- [Renew map-inspector fixture for completed video publication](renew-map-inspector-observer-fixture.md)

## Requirements

- Authenticate the old main source and exact reviewed module-registration-only delta; no whitespace normalization or arbitrary line stripping.
- Retain original observer.json, migration.json, threaded epoch archive and producer envelopes unchanged. Separate current producer identity from historical migration authentication explicitly.
- Fresh independently built producer and two new singleton processes use the exact owned SRAM/input/capture schedule. Compare all ten files and complete manifests/nonpixel evidence against accepted fixed roots and each other.
- Same epoch/policy; no output pin change or generic producer registry. Additional source hashes bind new module hooks without retroactively changing historical inventories.
- Reject identity substitution and any non-registration main change; no weakened whole-file source gate.

## Acceptance Criteria

- Exact source-delta proof, retained private build/process evidence and no-output-change bridge pass independent review.
- Historical normal/optimized audits still reproduce; current Rust source/capture gate and focused negative tests pass.
- Parent independently reproduces checks before archive. Future additional module registration requires explicit revalidation, not an implicit wildcard.
