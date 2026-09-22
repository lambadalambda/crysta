# Correct native Crysta tree compositing

## Summary

The user reports trees do not look fully correct, possibly a transparency issue.

## Requirements

- Identify the source-backed layer/transparency behavior at exterior tree pixels; correct only the demonstrated native rendering gap.
- Use regression tests and independent review before implementation commits.

## Acceptance Criteria

- A focused regression distinguishes incorrect and corrected composition; compare an exterior tree view to source/native evidence without weakening renderer qualification.
- Relevant tests and strict lint gates pass; document fidelity limits.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice).
- Raw ROM assets, screenshots and reference captures remain ignored under `local/`.
