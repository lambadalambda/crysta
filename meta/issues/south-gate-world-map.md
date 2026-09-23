# Leave through the south gate onto the world map

## Summary

The south exit `$818DB3` leads to map `$03` under a separate world-map controller, arriving at (536,544).

## Dependencies

- [Play the frozen return and the Elder's mission](frozen-return-mission.md)

## Requirements

- Admit map `$03` with its background, collision and world-map player movement from source.
- Establish whether the exit is gated before the mission, and model that.

## Acceptance Criteria

- The full replay from new game arrives on the world map as natively at frame 59760; Ark can walk there.
- Tests and gates pass.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice).
- Parent: [Play the Crysta story from the wake-up scene to the world map](play-crysta-story.md)

## Research (source and native, 2026-09-23)

- `$03` is a Mode 7 world map. Its layer comes from spawn record `$F0`
  (`$83:88FB`: `F0 04 04 00 00 00 D6`, handler `$80:F689`): 64x64 cells,
  one byte each, uncompressed at `$D6:0000`; `$8D:850A` copies it to
  `$7E:A000`, then `$8D:855D` applies table `$D8:0000` (events `$400+`).
  Metatiles `$B0:E0D1` (256 x four 8bpp char numbers), chars `$A4:E278`
  (256 x 8bpp), palettes `$B1:E7C5` (colour `$20`), `$B2:8BB8` (`$D0`).
  Rebuilt, it matches all 16,384 native tilemap entries.
- Scene prefix `40 94`: `$0868 = $80` (world mode), profile `$14`, BGMODE
  `$0F` (Mode 7). Camera (`$87:9123`): origin (x-128, y-160), wrapping at
  1024; no page region.
- World walking (`$84:DEE0`; probes `$80:C164`/`C21D`/`C2CE`/`C387`): 16 px
  grid steps at 2 px per frame, a step finishes after release; the next cell
  blocks when its byte is `$A0` or more; a blocked step slides sideways.
  Native 60/60 step starts fit. Arrival: raw (528,528) + (8,0), then a
  1 px/frame walk down to (536,544).
- The south exit has no condition; the gate is the flag table's `$296`
  entry (`$96:CE65`), which the runtime already applies on load.

## Plan

1. assets: the `$F0` record and a `WorldMap` decoder.
2. runtime: world mode on `$03`: grid steps, probe and slide, exits,
   arrival.
3. app: a flat top-down render with the world camera (perspective is a
   known difference).

## Progress

- `WorldMap` decodes `$03`'s layer; the runtime walks it on a `Plane`
  (grid steps, probe, one-sided slide, wrap at 1024) and the native legs
  rest where they did; the south gate opens with `$296`; Crysta's
  rectangle on the plane leads back into town. The app draws the flat plane
  with the world camera.
- Not modelled: the perspective view and its wrap in the app, the slide's
  side-walk animation, the `$D8:0000` patch table (`$8D:855D`: on `$03`
  one drawn byte at (41,36) when flag `$402` is set; collision unchanged),
  the residents' art on `$03`.
- Remaining for the acceptance: the full replay from a new game.
