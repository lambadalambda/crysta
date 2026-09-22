# Qualify horizontal type8 collision with native witnesses

## Summary

The passive clear-bit directional candidate admits horizontal type8 geometry,
but its Left/Right cases are currently synthetic-only. Capture actual player
resolver contacts before treating that coverage as native-qualified.

## Dependencies

- [Qualify the directional resolver for Crysta's remaining collision types](qualify-crysta-directional-collision.md)

## Requirements

- Use input-only empty-SRAM boot sessions, with no warps, memory patches, save
  states or position resets. Retain reproducible bounded held-input recipes.
- Capture contiguous ordinary-player Left and Right windows with live cells,
  attempted velocities and player-only resolver PCs. Prove type8 is actually
  dispatched, not merely nearby or handled by a later NPC call.
- Preserve passive/action-hook and clear-special-bit admission. Stop rather
  than skip frames if the native player leaves the admitted controller mode.
- Add failing coverage/regression tests before admitting evidence; fix runtime
  behavior only if a measured mismatch requires it. Keep production conservative.
- Commit only tooling, recipes, selected source metadata and hashes; raw native
  layers/traces and ROM remain ignored.

## Acceptance Criteria

- Native Left and Right type8 contacts have pinned replay windows and
  source-grounded sample/branch assertions with mutation controls.
- Every frame in each accepted window agrees with the candidate, or unexplained
  mismatches keep the issue open; no per-frame resets or exclusions.
- Existing collision and 24-outbound/23-return candidate tests remain green.
- Qualification limits are documented and independent review completed.

## Notes

- Scope is horizontal type8 only, not the special town descent, the full map-$41
  envelope, broader material branches or production enablement.
- Parent evidence: [collision](../../docs/collision.md).
