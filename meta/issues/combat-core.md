# Fight with the spear

## Summary

Ark's attacks, hits on enemies and on Ark, damage from the stat table `$8D:BDFA`, knockback, invulnerability, death, drops, experience and level-ups, life regeneration, game over (`docs/combat.md`).

## Dependencies

- [Make the whole underworld playable](underworld-playable.md)

## Requirements

- Model the combat core and the first enemies (Hiball, the yellow flyer).

## Acceptance Criteria

- Fights in tower 1's floors match native traces on both ROMs.

## Notes

- 2026-10-02: the thrust, hits, knockback, deaths, gems, EXP, level-ups, Ark's hurt and the game over play. Enemy walls (`$80:D101`), the line move (`COP CC`/`CD`), the yellow flyer and its bullets, the knight's front box (`COP D2`) and the Hiballs run. Open: the level-up and game-over presentations, wake callbacks (bits 1-11).
