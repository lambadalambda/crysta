# Correct the COP service table bound

## Summary

`docs/rom-map.md` documents COP services as selectors `$00..$7C`: 125 bank-`$80`
pointers at normalized `$0083B2..$0084AB`, with the following word `$109A` taken
as proof of the table's end. The running game contradicts this. All eight bits of
a COP signature byte reach the dispatch index, and real scripts issue selectors
above `$7C`. The map's own wording is precise about being a "maximal contiguous
valid run"; the defect is presenting that scan artefact as the table's extent,
including in the local test that "resolves all 125 COP pointers".

## Dependencies

- [Classify ROM code, data, and indirect dispatch](classify-rom-code-data.md)

## Requirements

- Establish the table's actual extent from the dispatcher's behavior rather than
  from a contiguous-validity scan, and say what the `$7D..$7F` words are.
- Update the ROM map metadata, `docs/rom-map.md`, and the dispatch validation so
  a selector the game actually uses resolves and classifies.
- Keep the reconstruction byte-exact and the existing dispatch checks strict; a
  wider table must not weaken the boundary test into accepting anything.
- Classify the handlers this exposes, at least `$80:A395`, currently
  `region=unclassified entry=none`.

## Acceptance Criteria

- `cargo run -p disasm -- inspect-rom runtime:80:A395` reports a classified COP
  service region and entry.
- The local ROM-backed dispatch test resolves every selector the game issues,
  including `$91`, and still rejects a corrupted table.
- `cargo test --workspace` and the Japanese byte-exact reconstruction still pass.
- `docs/rom-map.md` states the real bound and why the old one was wrong.

## Notes

- Milestone: [M2 — Matching disassembly foundation](../milestones.md#m2-matching-disassembly-foundation)
- Found while tracing the Pandora tour guide in
  [the tower-approach route](qualify-tower-approach-route.md); the guide's parked
  script is `COP $91`, which resolves through `$0084D4` to `$80:A395` and runs.
- Evidence: the dispatcher at `$808378` does `LDA [$36]` / `AND #$00FF` / `ASL` /
  `JMP ($83B2,X)`. The mask only clears the high byte of a 16-bit load, so all
  eight selector bits reach the index. Across `$00..$EF`, the only entries that
  are not bank-`$80` pointers are `$7D`/`$7E`/`$7F` (`$109A`, `$349B`, `$729B`) —
  exactly where the documented contiguous scan stopped. Entry `$00` points to
  `$8592`, which bounds the table at 240 entries before it would collide with its
  own first handler.
- M2 is otherwise complete. This is a correction to a published map, not new
  capability, but it should not be left contradicting observed execution.
