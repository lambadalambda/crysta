# Run Ark down the screen after Yomi sends him out

## Summary

After Yomi tells Ark to go outside, Ark runs down the screen natively; here he slides down with his back turned.

## Dependencies

- [Fix the slice bugs the user reported on 2026-10-01](slice-bug-report.md)

## Requirements

- Research the scripted run's pose, facing and animation and play it.

## Acceptance Criteria

- The scripted run matches native frames.

## Notes

- The player script's `COP 84` dash (resource 1 list `$17`) and brake (resource 0 list 9) draw facing Down; 36 dash and 16 brake frames, as natively (`local_story.rs`).
