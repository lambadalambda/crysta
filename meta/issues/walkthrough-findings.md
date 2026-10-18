# Fix the walkthrough's findings

## Summary

The local walkthrough (`local/walkthrough/`, not tracked) plays the game
as far as the runtime goes and takes screenshots. Its first run found
these problems.

## Dependencies

- [Make the Crysta slice fully playable](playable-crysta-slice.md)

## Requirements

- [x] Shadowkeeper's fight after the torch intro stops a native run at
  `INC $0498` (`$93:DA4D`).
- [x] `$11B`'s show draws no balls: the troupe's palette slot 0 was
  refused.
- [x] `$111`'s darts are placeholders and tower 2's statues draw blank:
  mode-4 frames name sheet tiles, not moved OBJ tiles.
- [ ] Mu (`$12A`) and Polynesia (`$12B`) show "unsupported house movement
  resource" placeholders; Mu's scripts stop.
- [ ] The box tour stops a script on `$041` (`$39554`).
- [ ] The Elder's head draws over Ark (Crysta, the opening).
- [ ] Check the grey screen as the box opens and the area-title glyphs.

## Acceptance Criteria

- A walkthrough run shows none of these problems, and each fix has a
  regression test.
