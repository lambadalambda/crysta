# Bound the animation comparator's initial idle exemption

## Summary

The ordinary-animation comparator currently skips pose/facing checks for any row using the native idle-fidget table. Limit that exception to the observed first delayed-input frame, so later unexpected fidgets cannot pass unnoticed.

## Dependencies

- [Qualify Ark's standing and walking animation](qualify-ark-walking-animation.md)

## Requirements

- Permit only the qualified fresh-start context and initial fidget pose; keep movement checks active.
- Reject later fidgets or changed facing/selector/composition instead of exempting them.
- Do not change gameplay, animation timing, or implement idle gestures.

## Acceptance Criteria

- Red-to-green regressions reject mutated later/initial fidget rows.
- Existing authenticated ordinary animation comparisons and the two-frame negative control still pass.
- Independent review and repository safety/tracker checks pass.
