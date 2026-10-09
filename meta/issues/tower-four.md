# Play tower 4

## Summary

Tower 4: falls, burners, the tightrope, Three Cadets and the show boss in `$11B`.

## Dependencies

- [Make the whole underworld playable](underworld-playable.md)

## Requirements

- Model these mechanics, the fight and the boss.

## Acceptance Criteria

- Tower 4 can be finished as natively.

## Progress

- Plays from `$115` to the show (`$11B`) and the light room: the Cadets of `$118` drawn and hit, the ropes (walk, lean, fall), the falls, the Cadets' paralysis, the Guardners (sleep, vacuum, transfer back), the Dancing Huball Troupe, `$11A`'s ring.
- Open: the rope's Solid tables (Open as a first cut), the flyer burst (`$97:BCD4`, `LDA $002C,X` now the parent, a guess), the burn status, the lips of `$117`-`$119`.
- Done (2026-10-09): the rope's tables (`rope-collision-tables`), `+$2C` as the previous entity (the flyer burst's), the burn (traced), the lips (`lip-jumps`); the Guardner's last frame offsets stay in `guardner-fidelity`.
