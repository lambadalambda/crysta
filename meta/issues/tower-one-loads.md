# Load tower 1 from the world map

## Summary

The tower exits on `$03` lead to maps the runtime does not admit. Tower 1 (`$100`) needs its spawn list (`$82:8000 + 2*map`), its first layer (graphics `00 20 01`, `$4000`), its camera region (scene list in bank `$82`) and its track (`$11`).

## Dependencies

- [Enter the towers from the world map](enter-the-towers.md)

## Requirements

- Admit `$100` and load its spawn list, first layer, collision, camera and music on both ROMs.

## Acceptance Criteria

- Walking onto tower 1's entrance on `$03` loads `$100` and draws its first layer as natively.
