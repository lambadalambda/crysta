# Render the static cavern from decoded graphics

## Summary

Produce a local full-map visual artifact from ROM-resolved cavern graphics,
palettes and metatiles rather than framebuffer atlas samples.

## Dependencies

- [Resolve map loading scripts and qualify additional layers](resolve-map-loading-scripts.md)
- [Qualify static map loading and decode the cavern layer](static-map-cavern.md)

## Requirements

- Qualify cavern graphics/palette/metatile resource layout against loader evidence.
- Add pure bounded SNES graphics and map rendering primitives with synthetic tests.
- Preserve tile palette, flip and priority metadata; distinguish static rendering
  from animation, sprites and hardware compositing.
- Export an inspectable full-map artifact from an authenticated local ROM without
  SRAM or emulator execution; keep all derived assets ignored.

## Acceptance Criteria

- Cavern map ID resolves to graphics, palette, metatiles and a rendered full layer.
- Synthetic tests cover planar pixels, color conversion, tile attributes and
  placement, and malformed resource bounds.
- Owned-ROM tests and runtime evidence qualify the decoded resources and compare
  representative static map pixels with the oracle, documenting exclusions.
- Local browser QA confirms the visual artifact is usable.
- Documentation states the supported scope and outstanding rendering semantics.

## Notes

- Subissue of [graphics decoding](decode-graphics-animation.md) and
  [map formats](decode-map-collision-formats.md).
- Initial scope is the portal cavern ($0128), not a generic scene renderer.

## Completion

Completed and archived after implementation, independent review and verification.

## Implementation and verification

- Pure `assets::graphics` primitives plus a strict map-ID-resolved cavern recipe
  retain raw resources and indexed transparency/priority. Synthetic tests were
  implemented red–green; reverse-engineering findings use loader and memory
  evidence instead of speculative format tests.
- `render-map <japanese-rom> 128` produces the full 1280×512 background with a
  separate offline browser inspector. No SRAM or emulator execution is required.
- `qualify-loader` now checks 16,384 graphics bytes, 4,096 definition bytes,
  192 palette bytes, 672 BG1 tilemap words and a 256-pixel opaque reference patch.
  The pixel check applies the checkpoint's reference-only color effect; the
  exported map deliberately uses natural ROM colors.
- Full workspace tests executed with the owned ROM/SRAM and passed. A clean
  detached-worktree `assets` suite passed with explicit absent-ROM skips.
- Formatting, workspace Clippy (`-D warnings`), rustdoc (`-D warnings`), repository
  safety and tracker checks passed.
- Independent reviews covered primitives, resource recipe, viewer, CLI/oracle
  qualification and documentation. A duplicate-load test offset and one
  reference gamma-ramp entry were corrected, each with regression coverage.
- Browser regression passed on the actual generated artifact at desktop and
  mobile widths. Full-map overview and mobile screenshots were visually checked;
  no browser console errors or integration defects remained. Session closed.
- See [static graphics](../../docs/static-graphics.md) for formats, hashes,
  reproduction commands and the explicit one-background scope. Generated
  images, extracted bytes, experiments and reports remain ignored under `local/`.

The graphics and map-format parents remain open for broader maps, animation,
sprites, scene composition, transitions and collision behavior.
