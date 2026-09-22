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

## Completed

- Native windowed runs print the session path and record build/ROM identity,
  ticks, semantic movement/interact inputs, map/position, movement and interaction
  results, dialogue page, refusals and checked world errors. Separate action
  results preserve a refusal even when interaction succeeds in the same frame.
  A fatal frame flushes once; further stopped updates do not overwrite its history.
- Logging errors disable the writer, not the game. Native owned-ROM regressions
  verify exact Right/release/Left ordering, the tree refusal at `(360,472)`, later
  escape, continued gameplay with logging disabled, and checked destination failure
  with no subsequent world/tick/log changes. Eight logger tests and these three
  session tests pass; strict native Clippy passes.
- A real native window launched with `--no-music` printed its path and wrote
  1,555 valid JSONL records (ticks 1–1,550), including schema/build/ROM metadata;
  the smoke service was then stopped. No additional audible-device verification
  was attempted in the restricted environment.
- Independent integration review found and verified fixes for repeated fatal-frame
  logging and overwritten movement outcomes, then approved with no blockers.
- README documents files to send, one active app per root, roughly 64 MiB retention,
  segment ordering, buffering and partial-history limitations. No ROM bytes/assets
  or dialogue text are logged. This is diagnostic evidence, not guaranteed replay
  or native-reference qualification. Screenshot mode intentionally skips logging.
- Archived. Full root release workspace tests, formatting and strict Clippy also
  pass; the native crate stays outside that workspace and the root lock is unchanged.
