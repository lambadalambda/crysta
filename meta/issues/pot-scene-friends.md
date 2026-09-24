# Place the friends in the pot scene as natively

## Summary

In the blue door's pot scene nobody stands in front of the door natively
(user screenshot); the slice draws a friend there. The actor may be a
hidden helper behind the door.

## Dependencies

- [Make the Crysta slice fully playable](playable-crysta-slice.md)

## Requirements

- Compare the scene's actors with the native route and fix what differs.

## Acceptance Criteria

- The scene's visible actors stand where the native route has them.

## Resolution

- The "friend" at the door was the door's hit target `$88:AAEE` (record
  `$38C32`), drawn with the friend's reused descriptor. Its script points
  its art at the object sheet (`COP 48`, `COP 48`, `COP D8 $A2C000`,
  `COP 80 0`); natively nothing shows there, so it is not drawn. The
  spear's display in `$42` now draws its own list the same way.
