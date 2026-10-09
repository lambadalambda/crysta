# Play towers 2 and 3

## Summary

Towers 2 and 3: the gate rule, floor switches, jewel statues, moving blocks, tile-patch bridges, wall traps, holes to the floor below, Cadet, the knight, Guardner, High Cadet.

## Dependencies

- [Make the whole underworld playable](underworld-playable.md)

## Requirements

- Model these mechanics and enemies.

## Acceptance Criteria

- Towers 2 and 3 can be finished as natively.

## Progress

- Tower 2 plays (gate, Cadets, statues, blocks, switches, Hiball wave).
- Tower 3 plays from `$10E` to the resurrection: pits and falls, `COP 3F` floors, pedestals, the ball wave, the High Cadet (`docs/tower-three.md`).
- The Guardners (drawn, hit, sleep and vacuum, transfer back down the tower).
- Open: the landing's drop on `$114`, the darts' damage.
- Done (2026-10-09): the landing on `$114` (`world/landing.rs`) and the darts' hits (traced) close the last items.
