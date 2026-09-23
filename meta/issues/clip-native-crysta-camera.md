# Clip the native Crysta camera to the source map region

## Summary

Several Crysta maps share one background layer. The native app clamps its
camera to the whole layer, so the view can show part of a neighbouring room.
The game clamps to a per-map region instead.

## Dependencies

- [Native macOS window, renderer and gamepad for the Crysta slice](crysta-native-shell.md)

## Requirements

- Decode the region from the source camera record at `$96BE30 + 2*map`
  (`$869371..93F7`) and the clamp height `$0866` from the scene's display
  profile, for every map in the slice. Refuse records and profiles that are
  not understood rather than falling back to the layer size.
- Clamp as `$8790A1..9105` does: `clamp(x-128, left, right-256)` and
  `clamp(y-112, top, bottom-$0866)`.
- Share the clamp with the existing Pandora `SourceCamera` instead of a copy.

## Acceptance Criteria

- Pure tests cover region decoding, refusal of unknown profiles, and the clamp
  at every edge.
- An owned-ROM test decodes a region for all 24 slice maps, and a
  one-page room that shares its layer no longer shows the neighbour.
- Root formatting, strict Clippy and relevant tests pass.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice).
- Source rule: [house backgrounds](../../docs/house-backgrounds.md) and
  [house exterior](../../docs/house-exterior.md). All 24 maps use display
  selectors 6, 8 or `$1B`, each with `$0866 = 256`.

## Result

- `assets::maps::visual::camera::CameraRegion` reads the record, refuses a
  bank-`$82` scene, unknown prefix bits, a clamp height other than bit-6 256
  and a region smaller than its clamp. Pandora's `SourceCamera` now takes its
  bounds and clamp from it; its pinned records and mutation tests are unchanged.
- The native app caches the region with each background and composes with
  `settled_origin`. In map `$0C` at `(136,300)` the camera moved from
  `(8,188)`, which showed `$0B`'s floor, to `(0,256)`. The exterior now stops
  at `y=768`, not at the 1280-pixel sheet.
- Three pure decoder/clamp tests and one owned-ROM app test (all 24 maps
  decode; `$0C` and `$0A` cameras) failed before and pass after. Workspace
  fmt, Clippy and tests, the app's 58 tests with the ROM, and the Pandora ROM
  camera tests pass.
