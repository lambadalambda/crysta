# Build canonical RAM and hardware symbol maps

## Summary

Replace anonymous addresses with evidence-backed names and reusable typed metadata.

## Dependencies

- [Disassemble boot, interrupts, and the main loop](disassemble-boot-main-loop.md)
- [Export and compare reference frame state](reference-state-export.md)

## Requirements

- Import and verify existing public RAM-map findings.
- Record width, lifetime, aliases, and confidence for each symbol.
- Distinguish hardware registers, direct-page state, WRAM, SRAM, and buffers.
- Generate formats consumable by disassembly, tracing, and Rust tooling.

## Acceptance Criteria

- Known player coordinates, current map, inventory, and event flags are verified by traces.
- Conflicting aliases are documented rather than silently merged.
- Generated symbol outputs are reproducible.

## Notes

- Milestone: [M2 — Matching disassembly foundation](../milestones.md#m2-matching-disassembly-foundation)
- Treat community maps as leads, not ground truth.

## Implementation

- Added schema v1 in `crates/memory-map/data/japan-v1.json`, bound to the
  normalized Japanese revision and SHA-256, with typed address space, width,
  kind, access, lifetime, aliases, confidence, evidence, and conflicting claims.
- Hardened provenance, physical overlap, bank-local SRAM-window, and whole-region
  short-operand validation. Typed conflict queries keep disputed ranges separate
  from canonical address lookup.
- `current_map`, `player_x`, and `player_y` are trace-corroborated. Broad event
  and inventory ranges remain imported claims; the trace checks only selected
  contents and record starts.
- Retained the competing `$7E:081E` meanings. Armor starts at selected imported
  boundary `$7F:8068`, while Data Crystal's `$7F:8066` boundary remains an
  unresolved typed conflicting claim; neither is trace-proven over the other.
- Added stable symbol trace v1 with separate scenario labels/actual core frames
  and domain-separated canonical binary digests. The qualified local-SRAM trace
  digest is `6d2c6756ff22e9c41a8287f649252599a493422b54d684e886b64e9b12741595`.
- Added deterministic ca65 include generation and Rust/oracle consumers. The
  schema supports SRAM regions, but no canonical SRAM fields are claimed.

## Verification

Targeted checks recorded as run:

- `cargo test -p memory-map` — schema, provenance, physical overlap, SRAM-window,
  short-operand, typed-conflict, confidence, and include tests passed.
- `cargo test -p oracle canonical_symbols_export_from_wram_in_requested_order`
  and `cargo test -p oracle symbol_trace` — targeted export and stable
  symbol-trace v1 unit tests passed.
- `cargo test -p oracle --test local_roms local_sram_trace_verifies_gameplay_symbols -- --nocapture`
  — the qualified Japanese-ROM plus local-SRAM child trace ran at labels
  1600/1799/1840 and produced the expected `6d2c6756...` digest.
- `cargo run -p disasm -- reconstruct 'Tenchi Souzou (Japan).sfc'` — generated
  `memory-symbols.inc` in the ignored run directory and matched all 4 MiB of the
  normalized Japanese ROM byte-for-byte with cc65 2.18.

- `cargo fmt --all -- --check`, workspace Clippy with warnings denied, all-target
  workspace tests, and rustdoc with warnings denied passed.
- `python3 tools/check_repo_safety.py` passed before final tracker archival.
- Independent final review found no blockers or high-severity issues.
