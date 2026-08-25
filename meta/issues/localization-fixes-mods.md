# Support additional localizations and opt-in fixes or mods

## Summary

Define data-driven extension points for language content, bug fixes, and trusted modifications.

## Dependencies

- [Qualify the full classic-mode replay suite](qualify-classic-mode.md)
- [Define the ROM revision and version-support model](define-version-support-model.md)
- [Select the project license and contribution policy](select-project-license.md)

## Requirements

- Separate language resources from gameplay logic and font rendering.
- Recognize supported user-provided localization sources by hash.
- Version feature flags and mod data with explicit conflicts and provenance.
- Prevent arbitrary native code loading in data packages by default.

## Acceptance Criteria

- At least one additional user-provided localization can be generated without committing its content.
- A representative bug fix is opt-in and leaves classic tests unchanged when disabled.
- Invalid, conflicting, or incompatible packages fail safely.

## Notes

- Milestone: [M8 — Enhancements and extensibility](../milestones.md#m8-enhancements-and-extensibility)
- Licensing and distribution must be reviewed per localization or mod source.
