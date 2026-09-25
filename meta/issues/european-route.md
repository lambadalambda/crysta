# Check the European slice against a native route

## Summary

Record a native European route through the slice in the reference emulator
and run the story tests against it.

## Dependencies

- [Accept the European ROM in the app and the web page](european-hosts.md)
- [Run the European version at its own timing and sound](european-timing.md)

## Acceptance Criteria

- The European story tests pass against the recorded route.

## Progress

- An input-only native European New Game now starts from empty SRAM,
  accepts the default name, plays Elle's wake-up dialogue to event `$20`,
  and walks from bedroom `$0F` into room `$10` at `(392,353)`.
  `crates/oracle/tests/local_roms_eu.rs` replays this route in a fresh child
  process; `crates/crysta-runtime/tests/local_european.rs` checks the same
  opening story and doorway in the portable runtime. Both pass with the
  owned European ROM. This is an opening witness, **not** the requested
  bedroom-to-world-map native route or a continuous portable replay.
- The next native checkpoints are the Elder's `$26` conversation, the town
  and Pandora progression, the frozen return, and world-map `$03`. Their
  European dialogue acknowledgement schedules must be observed rather
  than copied from the Japanese route.
