# Run scene dialogue, choices and flags from scripts

## Summary

Talking today merges every callback's text and applies all flag writes before the first page. Run interaction scripts instead: pages, choices and flag writes at their true point, with the player's control owned by the scene.

## Dependencies

- [Play the Crysta story from the wake-up scene to the world map](play-crysta-story.md)

## Requirements

- A scene executor steps a script over frames: text services show pages and wait for acknowledgement, choice services show the catalog and branch on the answer (`$22` jump tables), flag writes happen where the script makes them.
- Control ownership: while a scene owns control, player input only advances or answers it.
- Unknown services stop the scene visibly and are logged, not skipped.

## Acceptance Criteria

- The Elder in B sets `$26` after the first request and before the choice; either answer continues, as natively.
- The weaver in `$13` sets `$28` only for answer 1; answer 2 or cancel refuses and re-prompts.
- Pure tests per service, owned-ROM tests for both conversations; gates pass.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice).
- Parent: [Play the Crysta story from the wake-up scene to the world map](play-crysta-story.md)

## Result

- `crysta_runtime::scene` holds the window (`Dialogue`: request, pages,
  retained `$D4` page, choice cursor) and `Globals` (flags, window, input
  mask). The actor interpreter runs `COP 1B/1F/20/1A/07/21/29/2A/23/24/C0/06/
  BC/48`; callbacks run on the resident with its own script held, and a
  blocking service holds the world until answered. `World::update(direction,
  presses)` replaces the merged `talk()`. The app draws the world's page and
  choice cursor; B cancels (X/Backspace, gamepad East).
- Model and service table: [scene scripts](../../docs/scene-scripts.md).
- Owned-ROM tests with real presses: the Elder shows arrival text, one
  request, sets `$26` before choice 0, and either answer continues into
  three cooperative follow-up pages (as natively for option 1); the weaver
  refuses on option 2 and grants `$28` on option 1 when asked again.
- Pure tests: the window state machine (5) and synthetic callbacks, input
  locks, deletion, jumps and continuations (3); tests written with the code
  rather than strictly before it. 54 runtime unit tests, all integration
  tests, workspace gates and 66 app tests with the ROM pass. Headless
  screenshot shows choice 0 with the cursor on option 2.
- Independent review approved; its fixes are in: `RTL` in an actor's own
  script resumes at `+$0A` as natively (the continuation `COP BC`/`C0` set,
  else where the frame began); a blocked actor is found again after
  deletions; `step`/`step_checked` hold while a scene is active; `update`
  returns both the walking step and a doorway interaction. Discovery tests
  acknowledge a held world as a player would; reachability is unchanged.
- Visible change: `COP 48` now deletes six residents on entry that used to
  freeze in place (e.g. D's hidden gate once `$26` is set), as natively.
- Open: the Elder's arrival page closed on held Up natively; not traced.
