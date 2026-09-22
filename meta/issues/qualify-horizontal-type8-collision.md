# Qualify horizontal type8 collision with native witnesses

## Summary

The passive clear-bit directional candidate's Left/Right type8 cases initially
had only synthetic controls. A new100-frame input-only window now qualifies
actual **Open/type8** player dispatch in both directions. Other horizontal
pairings remain in the parent qualification issue.

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

## Completion

- `slope_route.py Type8Horizontal` preserves the cap approach, then captures
  every frame14342–14441: Right30, neutral12, Down4, neutral12, Left30,
  neutral12. Two independent boot runs are byte-identical; the complete capture
  SHA-256 is pinned in `crysta-runtime/tests/local_collision.rs`.
- Five explicit native witnesses cover Left/Right Open/8 and1/2px attempts.
  Assertions derive tentative sample coordinates, require actual unflagged0/8
  cells, reject raw old-edge slope diversion, and prove ordered lookup/dispatch
  pairs inside the player's resolver segment. Nearby8 or NPC PCs cannot count.
- Portable mutation controls cover cells, flags, alignment, direction/attempt,
  sample coordinates, old-first/old-second slopes, wrong/reordered dispatches,
  and NPC-only evidence. Recipe and witness tests were red before implementation.
- All100 native frames agree without exclusions/resets or runtime changes.
  Ten pinned windows now total **5,641 frames**, alongside existing legacy
  regressions. Candidate **24/24 outbound and23/23 returns** remains green;
  production collision stays conservative **19/24**.
- Full release workspace tests, explicit-root native replay,45 Python tooling
  tests, strict clippy/rustdoc and formatting passed. Independent source review
  and executable evidence verification found no blockers.
- Native qualification is limited to **horizontal Open/8**. First8, Partial/8,
  Solid/8, slope-mediated contacts and other special modes remain unqualified;
  no production enablement is implied.
