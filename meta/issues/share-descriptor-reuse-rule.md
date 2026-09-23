# Share one descriptor-reuse rule between sprites and cadence

## Summary

A spawn record with a zero descriptor pointer reuses the previous record's
resource. The sprite loader (`HouseActor::from_records`) steps over `$00`
and `$FD` records to find that predecessor; the cadence derivation refuses
a chain that contains them. Two rules for one source behaviour will drift.

## Dependencies

- [Derive each walker's step and idle timing from source](derive-walker-cadence-from-source.md)

## Requirements

- Establish from the record parser (`$80:F4EA`, `$80:FA65..FB1A`) whether
  `$00` and `$FD` records update the reuse state (`$46/$48/$4A`).
- Implement the rule once, in `assets`, and use it from both callers.

## Acceptance Criteria

- Pure tests of the shared rule, including `$00`/`$FD` in the chain.
- Sprite and cadence owned-ROM tests pass unchanged, or any change is
  explained by the established rule.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice).

## Result

- Source (`$80:F4EA`, `$80:FA65..FB1A`): `$00` and `$01` records share the
  parse path, so a `$00` with a real descriptor becomes the predecessor.
  `$FD`/`$FE` never touch `$46/$48/$4A` or the packet. `$FA` branches decide
  which records run, so the predecessor follows executed order. A zero
  address word reuses; a bank at or above `$90` skips the parse.
- `assets::maps::actors` now has `descriptor_owner`, `parsed_before` and
  `SpawnRecord::descriptor_offset`. `HouseActor::from_records` takes its
  predecessor from `parsed_before`; `residents()` and `residents_art()` pass
  the executed list from `SpawnList::resolve`; `Resident` carries the
  resolved `descriptor`, and the cadence derivation takes it directly.
- In the 24 maps no `$00` record precedes a reuse and no `$FD` sits in a
  chain, so every sprite and cadence result is unchanged: the frozen
  resident-art tests and all eight walker derivations pass as before.
- Art resolves the executed list under the flags at map entry
  (`World::spawn_events`), and poses under the flags now, so a conversation
  that flips a spawn-branch flag cannot hide a spawned resident; a test
  failed that way before the split and passes after.
- Behaviour change outside new-game flags: records reached only through a
  `$FA` branch sit after the main list's end, so the old full-list lookup
  never found them and drew them invisible. They now decode.
- Records whose length is not ten bytes, or whose descriptor bank is at or
  above `$90`, stop the chain; one ROM-pointer helper serves both callers.
- The pure rule test failed to compile before the function existed (red)
  and passes. Workspace fmt/Clippy/tests, rustdoc and 65 app tests with the
  ROM pass. Independent review approved; its should-fix and nits applied.
