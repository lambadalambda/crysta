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
- The oracle now records bounded CPU writes to `$2140–$2143` in bus order,
  preserving them across mid-frame trace exits and clearing the timeline on
  successful state load. A fresh European route drains it each frame without
  overflow and witnesses adjacent native port-2/3 pairs for pot lift `$1100`,
  break `$1200`, door hit `$1300` and open `$001A`, plus bounded fade/change
  patterns (`F1`, parameter/`F0`, `FF` with the `$1048` destination, isolated
  final `F4`). The portable route pins the corresponding cues and avoids an
  unchanged-track restart.
- This does not close the playtest or prove PCM/DSP fidelity. The spear windows'
  source/route identities remain inferred because tracks `$34/$1C/$06` all use
  stop parameter 5. Native door effects order `$1200 → $001A → $1300`; portable
  currently orders `$001A → $1200 → $1300` because contact runs four frames
  early. Deferring only the cue would detach it from the ROM's immediate
  `COP 37 $1A`, so the timing mismatch remains explicit. Spear music returns
  after 405 native frames versus 420 portable.
