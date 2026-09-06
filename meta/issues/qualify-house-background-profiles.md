# Qualify complete house background profiles

## Summary

Decode and authenticate the static first-background/collision profiles for all six fresh house rooms, not just the original F/10 pair.

## Dependencies

- [Qualify the fresh house room and actor roster](qualify-house-scene-roster.md)

## Requirements

- Admit only source-qualified loading recipes for B,C,D,F,10,11; establish their map cameras, natural palette, hardware background assignment and priority behavior.
- Compare source static grids/backgrounds to fresh native surfaces, account for runtime flag/occupancy overlays and coordinate with the navigation subissue on C/B door mutation.
- Distinguish first-layer setup from secondary layers, sunlight/windows/dialogue and transient text effects. Do not advertise gated E/20/21 as playable.

## Acceptance Criteria

- ROM-only static profiles and immutable collision overlay contracts cover all six admitted rooms with source/native evidence.
- Existing F/10 graphics remain unchanged; synthetic tests and fresh selected pixel/tile/palette/priority comparisons pass.
- Independent review passes, with raw artifacts local only.

## Notes

- Subissue of [complete fresh house setup](complete-house-scene-setup.md).

## Completion

- Six ROM-only loading recipes and source-derived camera/occupancy projection are implemented; F/10 retain their prior grids and graphics. D uses the frozen source-origin stamp and preserves the independent exterior gate.
- Source/native qualification, including selected pixels, hardware BG2 priority and explained runtime deltas, is recorded in `docs/house-backgrounds.md` and the metadata-only qualification profiles. Parent independently repeated the driver successfully at ignored `local/house-background-qualification/run-a76GGH`.
- Synthetic/source-mutation tests, owned-ROM complete grid/camera hashes, existing two-room host regression and strict host Clippy pass. Independent compiler correctness/architecture review found no blockers.
- C/B door mutation remains owned by the navigation subissue; these are explicitly baseclosed profiles, not an unexplained copy of post-door captures.
