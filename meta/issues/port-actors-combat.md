# Port the actor system and combat primitives

## Summary

Model actor slots, lifecycle, update dispatch, hitboxes, damage, knockback, and early enemies.

## Dependencies

- [Define the deterministic portable core model](deterministic-core-model.md)
- [Build canonical RAM and hardware symbol maps](canonical-memory-symbols.md)
- [Export and compare reference frame state](reference-state-export.md)

## Requirements

- Document original actor record fields and state transitions.
- Implement deterministic spawn/update/despawn ordering.
- Implement combat primitives required by the first tower.
- Compare actor tables and player combat state at checkpoints.

## Acceptance Criteria

- Selected encounters replay without unexplained state divergence.
- Unit tests cover hitbox boundaries, damage arithmetic, invulnerability, and lifecycle ordering.
- Unknown actor fields remain represented and traceable.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice)
- Avoid replacing verified slot ordering with an ECS if it changes update semantics.
