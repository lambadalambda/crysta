# Animate the river in native Crysta

## Summary

The user reports the exterior river does not flow. The native host currently caches a static background per map.

## Requirements

- Identify the original exterior background animation data and timing; add bounded native playback without fabricated animation or changes to gameplay cadence.
- Use regression tests and independent review before implementation commits.

## Acceptance Criteria

- Tests prove source-derived animation changes the river over time and repeats correctly; preserve static map rendering and existing qualified scenes.
- Relevant tests and strict lint gates pass; document fidelity limits.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice).
- Raw ROM assets, screenshots and reference captures remain ignored under `local/`.
