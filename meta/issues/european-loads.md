# Time the European loads and arrivals

## Summary

The exits' loads, darks and arrival walks are measured mostly on the
Japanese ROM (`world/transition.rs`). The European (PAL) ROM is modelled
from three samples: room loads a frame shorter, darks 3 frames shorter
(the bedroom door's 14, `$0C` to `$0D`, `$0C` to `$10`) and its own door
arrival (native traces `loads/eu_*`, 2026-10-18). The big loads (the town,
the world map, the towers' entrance) and the stairs take the same rules
unmeasured.

## Dependencies

- [Make the Crysta slice fully playable](playable-crysta-slice.md)

## Requirements

- Measure the European big loads and stair transitions and fix the rules
  where they differ.

## Acceptance Criteria

- European traces of the town, world-map and stair transitions match the
  model frame by frame.
