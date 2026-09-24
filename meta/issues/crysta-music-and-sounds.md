# Play the slice's music and sound effects

## Summary

The app boots the sound driver once and plays Crysta's theme (selection 3)
throughout, with no sound effects. The game changes music on map loads and
in scenes, and plays sound effects from scripts and the player code.

## Dependencies

- [Play the Crysta story from the wake-up scene to the world map](play-crysta-story.md)

## Requirements

- Music follows each map's loading script (`08 FC` selection, flag-dependent)
  and scene changes, through the driver's own commands.
- Sound effects from scripts (`COP 37`, the `COP 76` queue, `COP 60`'s id) and
  the player's actions reach the driver as natively.
- Tests stay silent (`CRYSTA_PLAY_AUDIO` gates audible checks).

## Acceptance Criteria

- Along the slice, the selected track matches the native route at each map
  load and scene change; source-derived tests pin the selections.
- Sound effect commands match the native route's port writes for a sample of
  events (a door, a pot, the spear's fanfare).

## Sub-issues

1. [Select music per map load](music-per-map.md)
2. [Change music in scenes](music-in-scenes.md)
3. [Play sound effects](sound-effects.md)

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice).
- Base: `tools/native-music-qualification/README.md` (selection 3 upload and
  port protocol), [reverse the audio protocol](reverse-audio-protocol.md).

## Progress

- The three sub-issues are done (2026-09-23): the runtime cues tracks and
  sound effects (`crysta_runtime::audio`), the app plays them through the
  driver. Open: a playtest.
- The text window types a glyph a frame with its blip (`$28`/`$25` on
  port 3; `$C5` pauses, `$C7` blip, `$C8` speed), 2026-09-24.
