# Reject world loading when resident resolution fails

## Summary

`World::enter_with_policy` currently replaces a refused resident spawn stream
with an empty roster, removing occupancy without reporting the loading failure.

## Dependencies

- [Place Crysta residents and let the player talk to them](crysta-resident-interaction.md)

## Requirements

- Propagate resident-resolution failures with the destination map and original
  decoder error from every world constructor and both checked transition paths.
- Keep legitimate empty rosters valid; retain the existing conservative/candidate
  policies and legacy convenience-wrapper behavior.
- Do not change sprite/pose decoding or extend collision/progression admission.

## Acceptance Criteria

- Regression tests distinguish failed resolution from a successfully empty list.
- Failed checked transitions do not install a destination with missing residents.
- Valid world/route tests continue to pass, including the existing reachability
  expectations; public documentation no longer advertises an empty-roster fallback.
- Independent review and relevant test/lint/documentation gates pass.

## Notes

- Follow-up to the checked collision-candidate traversal review, explicitly
  requested separately from collision qualification.

## Resolution

- Replaced the shared constructor's empty-roster fallback with
  `WorldError::Residents { map, source }`; diagnostics retain the original typed
  `ResolveError`, including through `std::error::Error::source`.
- Both collision policies and checked transition paths use that constructor;
  destination construction must succeed before the current world is replaced.
  Legacy wrappers still return `Stayed` on failure rather than entering an empty
  destination. Sprite/pose decoding and collision admission are unchanged.
- TDD regressions first reproduced both the misleading downstream room error
  and actual successful empty-world entry for an absent resident list. They now
  pass for truncated/absent/unknown/unsupported-condition failures, all three
  constructors, both checked transition paths, and legitimately empty lists.
- Independent source review found no blocking issues. Full workspace tests
  (including owned-ROM world/route tests), strict Clippy, strict rustdoc,
  formatting, tracker and repository-safety checks passed.
