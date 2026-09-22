# Record native play-session diagnostics

## Summary

The user offered debug logging so outdoor bugs found during ordinary play can
be reproduced and diagnosed. Record bounded local input/state/outcome traces.

## Dependencies

- [Native macOS window, renderer and gamepad for the Crysta slice](crysta-native-shell.md)

## Requirements

- Automatically print the current log path when launching the native app.
- Record semantic inputs, ticks, map/position, movement refusals and errors;
  include schema/build/ROM identity, but no ROM bytes, extracted assets or secrets.
- Keep logs under ignored local storage with bounded size/retention.
- Logging failure must not crash or freeze gameplay; flush useful failure records.
- Document how a user can provide a log and what it can/cannot reproduce.

## Acceptance Criteria

- Tests verify trace shape, input ordering, flushing and bounded retention.
- A local replay/diagnostic can identify the reported outdoor refusal from a log.
- Native gameplay remains usable when logging is unavailable.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice)
- Requested alongside [outdoor stall investigation](fix-native-crysta-outdoor-stalls.md).
- These are portable-host diagnostics, not a renewed native reference qualification.

## Implementation

- Added an asset-independent JSONL writer with four owned session slots, two
  8 MiB segments each, repeated schema/session metadata and bounded flushing.
  Unknown files and symlinks are preserved; one active writer per root is required.
- Eight ROM-free logger tests cover ordering, flushes, rotation/retention,
  oversized entries, unavailable storage and preservation of unknown paths.
  Independent correctness/architecture review approved the module; the
  immediate-flush test now uses an explicit long interval to avoid timer flakiness.
- Host integration and user-facing instructions follow separately.
