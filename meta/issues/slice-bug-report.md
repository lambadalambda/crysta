# Fix the slice bugs the user reported on 2026-10-01

## Summary

Twelve faults the user found playing the slice: depth order at doors, the
Box's bookcases and the weapon; poses on stairs, with the weapon and in the
run after Yomi; the voice and freeze effects; the frozen townsfolk; the
weapon's text icon; and the world map's Mode 7 view and tower entrances.

## Dependencies

- [Make the Crysta slice fully playable](playable-crysta-slice.md)

## Requirements

- Each fault is a sub-issue, researched against the native game on both
  ROMs and checked against native frames.

## Acceptance Criteria

- All sub-issues are closed.

## Sub-issues

1. [Hide Ark's head behind a door's top as he enters](door-entry-depth.md)
2. [Show Ark's stair poses](stair-poses.md)
3. [Keep Yomi in front of the Box's bookcases](box-yomi-depth.md)
4. [Draw the weapon in front of its pedestal](spear-pedestal-depth.md)
5. [Draw the weapon's text icon with a clear background](spear-text-icon.md)
6. [Show the glow when a voice speaks from the blue door and the Box](voice-glow-effect.md)
7. [Play the freeze: ice crystals, the brightening, a blue Elle](freeze-effect.md)
8. [Hold the weapon over Ark's head when he gets it](weapon-overhead-pose.md)
9. [Run Ark down the screen after Yomi sends him out](yomi-run-out.md)
10. [Freeze the townsfolk after the freeze](frozen-townsfolk.md)
11. [Draw the world map in Mode 7](world-map-mode7.md)
12. [Enter the towers from the world map](enter-the-towers.md)
