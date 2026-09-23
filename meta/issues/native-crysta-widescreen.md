# Add a 16:9 view to the native Crysta app

## Summary

The native app draws the classic 256x224 view. On modern screens a 16:9 mode
should show more of the map to the sides, without stretching pixels.

## Dependencies

- [Clip the native Crysta camera to the source map region](clip-native-crysta-camera.md)

## Requirements

- An opt-in wide view of 400x224 (about 16:9). The classic view stays the
  default and is unchanged.
- The wide camera follows the player within the source region. A region
  narrower than the view is centred, and everything outside the region is
  blank, so the wider view never shows a neighbouring room or unrelated
  layer data.
- Dialogue boxes stay placed relative to the classic 256-pixel area.
- Simulation, actor behaviour and timing do not change with the view.
- Select the mode from the command line and toggle it at runtime; headless
  screenshots can use it too.

## Acceptance Criteria

- Pure tests cover the wide camera, region masking and dialogue placement.
- Classic-mode frame tests pass unchanged.
- An owned-ROM headless screenshot shows the exterior wider and a one-page
  room centred with blank sides.
- Root formatting, strict Clippy and relevant tests pass.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice).
- A narrow first slice of
  [enhanced and widescreen rendering](enhanced-widescreen-rendering.md), for
  the native Crysta app only. All residents already run whether on screen or
  not, so no spawn or culling change is needed here.

## Result

- `frame.rs` composes into a `Canvas` of runtime width (256 or 400). The
  camera is signed. At 256 it is the source clamp, which a grid test proves
  against `CameraRegion::settled_origin`. At 400 it follows inside wider
  regions and centres narrower ones. `mask_outside` blanks everything outside
  the region before dialogue is drawn. Pages keep to the centred classic area.
- `--wide` selects the view for windowed and screenshot runs, and `V` toggles
  it at runtime. The window keeps its height and fits its width
  (`fitted_width`, tested) unless maximized. The session log header records `view_width`, and a toggle logs a
  `view` host event.
- Red: the new camera and mask tests failed on stubs. Green: 18 frame tests
  and 64 app tests with the owned ROM, including wide `$0C` (`(-72,256)`,
  blank sides) and wide exterior (`(304,768)`). Classic screenshots for three
  scripts are byte-identical to the previous commit. Wide screenshots of `$0C`,
  the exterior and an open dialogue were inspected. App Clippy is clean.
- Not verified here: the interactive `V` toggle and window resize, because
  this session has no display. Needs a check on the desktop.
