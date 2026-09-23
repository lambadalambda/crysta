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
