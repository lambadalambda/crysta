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

## Resolution

- `COP 37` / `36` / `38` write the latch `$04B6` / `$04B7`; the app hands it
  to ports 2 and 3 on every other frame, as the NMI does, after uploading
  the sound effect bank (`$C6:2191`) at boot. `COP 76` writes PPU registers,
  not sounds.
- Modelled by hand: exits `$4D`, stair landings `$17`, wooden doors `$1A`,
  pot lift `$11`, break `$12` and door hit `$13` (native route
  `route-audio.log`).
- Gaps: the text window's blips (`$28`/`$25` on port 3, from `$09B3`) are
  not played, since pages appear whole; the door's hit follows the break at
  once rather than four frames later.
