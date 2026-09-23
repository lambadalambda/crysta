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
- Idea to keep (2026-09-23, from a player discussion): an opt-in
  "progressive moves" rule set. Rush, Spinner, Slider and Slicer unlock
  over the game instead of at the start (NPCs, scroll items, optional
  rooms built from unused maps).
  - Locking moves alone does not stop jump-attack spam, the stated
    problem. Pair it with enemy overrides that use the table's per-element
    and per-physical-move resistances, or with a jump attack rule.
  - Keep mod state in its own save section and check it in the combat
    rules. Do not take over native event flags (unused building flags, the
    tower door flags): the story tests compare the native flags with the
    route.
  - Rush must come before the Tower 5 boss. Each lock needs a fallback
    before every point of no return, so that no move can be lost for good.
  - Needs combat and saves first. Order: a small rules layer (move gates,
    enemy table overrides, extra save state), then the unlocks with the
    resistances, then the content.
