# Decode the required opening dialogue presentation

## Summary

Decode only the ROM text/font resources and bounded text commands needed by the room B progression conversation. Reuse a compact data interface suitable for simple browser presentation; retain page/ack boundaries without requiring native window effects.

## Dependencies

- [Complete the fresh house scene with all residents](complete-house-scene-setup.md)

## Requirements

- Bounded subissue of [talk and leave the house](talk-and-leave-house.md).
- ROM/source-driven compilation, no capture-seeded production data or original CPU in simulation.
- Independent correctness/architecture review before substantial commits.

## Acceptance Criteria

- ROM-only dialogue presentation matches selected fresh text/font evidence, with synthetic malformed-source tests and explicit command limits. No copied text or glyph assets tracked.

## Completion

- ROM-only HouseDialogue decodes all14 pages across seven requests and both native choice catalogs. Next/End acknowledgement pages remain distinct from no-ack retained choice tails; actual option labels are source glyph crops, not invented text.
- Ten synthetic decoder tests, independent all-page reconstruction, source/tamper controls and separate implementation/qualification reviews passed. Parent export `local/house-dialogue-qualification/export-uuTD4a/pages` also matches67,904 native font-cell pixels across11 pages/tails against the fresh conversation replay, normally and under Python -O.
- Three alternate first-follow-up pages have independent ROM/source verification but no selected native raster sample. Simplified high-contrast glyph presentation intentionally omits native text timing/window/color effects. See `docs/house-dialogue.md`.
