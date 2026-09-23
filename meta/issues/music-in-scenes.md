# Change music in scenes

## Summary

Scripts change the music during situations (for example `COP 38` writes a
music word to `$04B6`). How the host turns that into driver commands is not
traced.

## Requirements

- Trace the scene music services and their host path to the driver.
- Model them in the runtime as audio events the app plays.

## Acceptance Criteria

- The slice's scene music changes (the box, the frozen return, the town)
  follow the native route; tests stay silent.

## Notes

- Parent: [Play the slice's music and sound effects](crysta-music-and-sounds.md)

## Resolution

- `COP 30` / `31` / `32` start, fade to, or restore a track through a
  worker actor (`$8D:950A`, `$8D:94C5`); `COP 33` waits for its load.
  `COP 60`'s fourth operand is a fanfare; the player's presentation goes
  back to the map's track after the grant's word in frames (`$84:BF0A`).
- Story tests pin the blue door's reaction (fade to 1, back to 4), the box
  (`$31`) and the spear's fanfare (`$34`).
- Gap: natively the spear's return comes 405 frames after the fanfare, not
  420; the difference is not traced.
