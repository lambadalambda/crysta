# Qualify an opening room transition

## Summary

Establish a reproducible gameplay checkpoint and one room transition toward the
Crysta opening, then decode the event/trigger path and required destination data.

## Dependencies

- [Render the static cavern from decoded graphics](render-static-cavern.md)
- [Resolve map loading scripts and qualify additional layers](resolve-map-loading-scripts.md)

## Requirements

- Reach a gameplay checkpoint through documented oracle inputs without silently
  patching game state or treating a forced warp as a real transition.
- Identify the trigger, source/destination positions and maps, relevant flags and
  bounded event instruction path needed by one interaction or doorway.
- Decode only evidenced formats; preserve source bytes and reject unknown modes.
- Inspect required destination map assets and retain local visual artifacts.

## Acceptance Criteria

- A fresh-process replay reproduces named before/after transition checkpoints.
- Source/data/trace evidence connects the trigger to its event or transition
  record, with synthetic parser tests and owned-ROM equality tests.
- Required map resources can be inspected without running the original CPU.
- Documentation distinguishes the covered path from a complete event VM.

## Notes

- Parents: [event bytecode](reverse-event-bytecode.md),
  [map formats](decode-map-collision-formats.md).
- Actual slot 1 is now selected by two Up taps; the older cavern replay used
  default slot 3. Name-entry experiments do not prove an ares/game-script stall.
  See [doorway qualification](../../docs/opening-doorway.md).

## Verification

- Pure bounded exit parser: synthetic framing/geometry tests and owned-ROM
  source hashes for maps `$000F` / `$0010`.
- `qualify-opening`: seven full-WRAM pins and two complete cold-boot JSON results
  identical; `trace-opening`: eight PC/register/state stops connect the selected
  record to pending/current map changes and destination actor initialization.
- ROM-only `render-map … f` / `10`: first-background assets extracted with strict
  profiles and runtime equality tests; see [room graphics](../../docs/room-graphics.md).
- Workspace tests, strict Clippy, safety/tracker checks and independent reviews
  passed. Native/COP path evidence is documented, not promoted to a full VM.
