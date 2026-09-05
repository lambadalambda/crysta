# New Game in the semantic house preview

The room-slice page now presents **New Game** as the primary way to begin, followed
by an explicit **Resume**. This is a semantic start at the controllable house
position, not playback of the original opening: **intro, dialogue, sprites and
audio are omitted; only the default name is supported**. Static BG1 presentation
and the existing unsupported-action boundaries remain explicit.

## Host contract and controls

| Control | Request | Expected start | After completion |
|---|---|---|---|
| New Game | `POST /new-game`, empty body | map 15, `(304,112)`, tick 0; `start_kind: "new-game"` | Paused; press Resume |
| Checkpoint Reset | `POST /reset`, empty body | saved checkpoint map 15, `(472,176)`, tick 0 | Paused; press Resume |
| Checkpoint doorway demo | `POST /reset`, then existing demonstration steps | Same saved checkpoint—not New Game | Demo runs, then pauses |

The response retains the usual `policy`, `map_id`, `x`, `y`, `tick`, `phase`,
`camera`, and `error` fields. New Game's backend phase is `walking`. The optional
`start_kind` is checked when supplied; omitted metadata remains compatible with
old hosts/test mocks. The frontend learns the normal phase label only at a known
tick-zero start, not from an arbitrary reloaded session. A `new-game` tag alone
cannot admit unknown coordinates. Start endpoint replies must match the requested
known origin. The client does not build room state, select a name, replay boot
inputs, reset simulation counters itself, or implement game initialization.

On initial connection, a saved-checkpoint response is clearly identified as a
diagnostic start and the page guides the user to New Game. Loading a mid-session
state requires New Game or Checkpoint Reset before driving. The old checkpoint
demo remains exactly 56 Left + 24 Down + 35 neutral logical steps; it is not a
new-game route or reference-video timing claim.

## Request ordering and input cleanup

New Game, Checkpoint Reset and the demo share the existing single-request pump:

- Starting clears the previous UI error, held controls, pending manual step and
  timer, and invalidates the old walking-phase admission immediately.
- An in-flight request is allowed to settle before sending the next request.
  **The latest undispatched start intent wins.** Already dispatched starts are
  not retroactively canceled at the host.
- When a newer start is pending, an older response/error is not applied to the
  displayed state or allowed to alter the newer start's demo-autoplay choice.
- Each dispatched start has its own autoplay intent. A queued New Game cannot
  inherit autoplay from an older demo reset; a queued demo cannot accidentally
  apply autoplay to an older New Game response.
- Pause, blur and hidden-tab handling cancel active/pending demo autoplay but do
  not discard an explicit start request. New Game always completes paused.
- Resume and manual steps cannot slip into a pending start. After New Game,
  resuming without fresh input sends neutral—not the formerly held direction.

These are client-side serialization guarantees, not a new backend transaction or
cancellation protocol. The native backend owns `/new-game`, source initialization,
collision/profile deployment and host-side state ownership.

## Verification

```sh
node crates/map-inspector/tests/room-slice-check.js
```

The dependency-free test evaluates the actual inline controller. TDD red was
`newGame is not a function`; green covers the empty-body endpoint call,
error/control/manual-step cleanup, paused completion, optional metadata,
known-start admission, all nine pending reset/new-game/demo combinations, four
in-flight start replacements, obsolete failures and canceled demo autoplay.
Existing pacing, checkpoint demo, keyboard/touch release, blur/visibility,
transition-neutral input, schema/error handling and drawing tests remain in place.
Independent read-only review found no blockers. One optional UX follow-up is
explicitly deferred: while a checkpoint-demo reset is pending, Escape or blur
cancels its autoplay, but the disabled Pause/Resume button cannot do so. This does
not affect New Game, which never autoplays.

This bounded change modifies only the HTML, its offline JavaScript test and this
document. The offline tests use a synthetic host. Separate parent integration now passes
`tools/verify-house-browser.js` against the actual CPU-free host: two identical
511-step runs click New Game, traverse both doorways and revisit positions.
See [playable house evidence](playable-house.md).
