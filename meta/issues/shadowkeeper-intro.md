# Run `$123`'s intro natively

## Summary

`$8F:8005` freezes at its VRAM fill (`$8F:828F`); the stand-in in `world/shadowkeeper.rs` replaces it. Natively it lights the corridor's torches by camera band, holds Ark at y 272, pans the camera to y 80 through `$0DEC`, sets flag `$001` and music 5 (`docs/tower-five.md`, "Shadowkeeper natively").

## Dependencies

- [Play Shadowkeeper's full fight](shadowkeeper-full-fight.md)

## Requirements

- Run the script: its VRAM fill, HDMA children (`COP AA`, `4E`, `A2`), `$04A4` torch bits, the camera target `$0DEC`.
- The hosts draw the darkness and the lit torches.

## Acceptance Criteria

- From Ark's arrival to the pad's release the frames match the native trace.
