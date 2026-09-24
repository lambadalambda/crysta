# Show the area titles

## Summary

Entering some rooms shows their name (長老の家 at the Elder's house), whose
letters then fly away (user screenshots). The slice shows none.

## Dependencies

- [Make the Crysta slice fully playable](playable-crysta-slice.md)

## Requirements

- Decode which maps show a title, its text, and the letters' motion.

## Acceptance Criteria

- Entering the Elder's house shows and dismisses the title as natively.

## Resolution

- The title follows the resolved spawn list (`$80:F4AC`, `$85:8008`),
  hidden by flag `$14`; its letters type from the arrival's first lit frame
  and run effect 3 of `$B0:DE49` (`assets::labels`). Renders of the Elder's
  house match the native screenshots (2026-09-24). Not checked: leaving
  during the effect, and the `D2` code of the flag-`$1AC` variants.
