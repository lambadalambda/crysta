# Play sound effects

## Summary

Scripts (`COP 37`, the `COP 76` queue at `$04D6`, `COP 60`'s sound id) and
the player's actions request sound effects; the runtime steps over them.

## Requirements

- Trace the sound effect command format and the host's port writes.
- Emit sound effect events from the runtime; the app sends them to the
  driver.

## Acceptance Criteria

- A door, a pot and the spear's fanfare send the native commands; tests stay
  silent.

## Notes

- Parent: [Play the slice's music and sound effects](crysta-music-and-sounds.md)
