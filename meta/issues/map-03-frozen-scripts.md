# Run the frozen display scripts on `$03`

## Summary

With the Crysta flags `$20`-`$2D` set, three display scripts on `$03` freeze (`$84:DEE5`, `$84:E3E6`, `$87:990A`). Their effect is unknown; they look cosmetic.

## Dependencies

- [Finish the underworld](underworld-end.md)

## Requirements

- Find what they draw and run them.

## Acceptance Criteria

- No script on `$03` freezes in the flag survey.

## Progress

- Done: `$03`'s `FD` record is the player's (header flags `$0400`, `$80:F46E`) and no resident; `$87:990A` (the view's HDMA) and `$84:E3E6` (the horizon band) are the hosts' (`docs/world-map-mode7.md`). The band's drawing: [world-map-horizon-band](world-map-horizon-band.md).
