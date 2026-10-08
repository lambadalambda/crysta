# Run the per-frame life upkeep

## Summary

Each frame the engine fills pending life (`$04CE`, `$85:E956`), sends Ark down at no life whatever took it (`$85:E15B` → `$85:E1BB`), and on a tower floor regenerates (`$07EF`, the weapon `$81` or `$9C`) and sounds the low-life warning (`$1C` every 128 frames at a quarter of the most life or less, `$85:E19E`). The runtime sends Ark down only after an enemy's hit, and has no pending life, regeneration or warning.

## Dependencies

- [Make the whole underworld playable](underworld-playable.md)

## Requirements

- Model `$85:E14B` and `$85:E93C`'s life fill as one per-frame step, gated by `$048A` bits 14 and 15 and Ark's state.
- A fall that takes the last life sends Ark down.

## Acceptance Criteria

- Unit tests cover the fill, the warning's frames, the regeneration's frames and the way down; a local test falls with little life and goes down.

## Notes

- `$097C` (the player's action word) is modelled only for forced actions (`$0810`); the game-over test reads `$8E17` of it, and the fill `$8000`.
- Research: `docs/combat.md` §5.

## Progress

- Done: `world/life.rs` runs the fill, the way down, the regeneration and the warning each game frame. Ark's action word stands in as the fall, the jump, the rope and a hit's push for `$097C & $8E17`.
