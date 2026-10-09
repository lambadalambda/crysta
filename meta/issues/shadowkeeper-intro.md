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

## Progress

- The intro runs (`actors/routines.rs`): the torch bits by camera band, the hold at 272, the camera's climb (`$0DEC`), flag `$001`, music 5 (`the_intro_lights_the_torches_holds_ark_and_climbs_to_shadowkeeper`).
- The darkness (`world/darkness.rs`): `COP AA $8182` sets it; per band of lines the app subtracts the map's mask (torch lit) or the fill, in the colour `$8F:8279` gives for the lit torches.
- The masks' own tiles from `$200` and the graphics adjustment `$0200` (`SecondLayer::tiles`): outside the light pools the view is black, as natively.
- The circle window (`world/circle.rs`: `COP 63`, `$0474`, the midpoint loop `$87:A5B3`), and BG palette 7's cycle (`COP 8A $22`, `Darkness::light`).
- Open: a frame-by-frame check against the native trace (it has lag frames).

