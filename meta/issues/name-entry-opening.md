# Enter a name and read the opening text

## Summary

A new game opens the name entry (`$87:89EF`); Start accepts the name (the
default when none is typed), then the opening text pages lead to the
bedroom.

## Dependencies

- [Show the title screen](title-screen.md)

## Requirements

- The typed name writes `$0610+` as natively (`docs/saves.md`, "New game").

## Acceptance Criteria

- The native bootstrap's presses (`crates/oracle/tests/fixtures`) reach the
  bedroom with the same name and slot bytes as natively, on both ROMs.
