# Qualify the full classic-mode replay suite

## Summary

Turn completed gameplay into a release-quality deterministic compatibility baseline.

## Dependencies

- [Complete Chapter 4 and the ending](complete-chapter-four-ending.md)

## Requirements

- Create completion, optional-content, menu, save/load, death, and reset scenarios.
- Eliminate unexplained state divergences in covered semantic fields.
- Measure rendering and audio regressions at representative checkpoints.
- Publish the known-issues and compatibility policy.

## Acceptance Criteria

- The full suite passes repeatedly on supported native platforms and in headless CI where fixtures permit.
- Every exclusion has an owner, rationale, and scope.
- No classic replay uses original CPU execution as a fallback.

## Notes

- Milestone: [M6 — Full classic game](../milestones.md#m6-full-classic-game)
- ROM-backed qualification remains local; public CI can validate replay machinery with synthetic and user-supplied fixtures.
