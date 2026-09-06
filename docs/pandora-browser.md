# Pandora input-only browser verifier

[Owned issue](../meta/issues/verify-pandora-browser-journey.md).
This is a bounded semantic-preview acceptance tool, not a native-frame replay.
The production host/controller/compiler remain parent-owned.

## Status and gate

Synthetic projection/control/composition tests pass. **Full browser acceptance is
not yet claimed.** The enabled isolated host at8877 was assigned exclusively to
browser session `pandora-journey-alice`; live8765 is forbidden. Its actual New Game
button initializes tick0/map15/(304,112), not the host's saved-checkpoint startup.
The parent's reviewed11590-state export is expected observation data only.

A real **unprojected prefix diagnostic** then emitted904 consecutive actual UI
commands, matching every parent state field and raw snapshot hash and performing
904 independent full-canvas comparisons. It stopped on command905, the first
movement in the dialogue-paused `[2,60]` span. **No omission was made** and the
run's status is failed/stopped, not full acceptance. Private retained evidence:
`local/pandora-browser/prefix-result-cli.json`, `prefix-final-state.json`, and
`prefix-eval.js`; the named browser retains `PANDORA_PREFIX_RUN.promise`.
All76 Pandora source rasters also independently matched the committed indexed
hashes in the parent export. This does not mean all76 were displayed.

The13 Pandora synthetic tests and28 combined house/conversation/Pandora tests
pass. Independent correctness/compactness review approved the staged verifier
after an initialization-failure regression was fixed red→green. Repository safety
passes; tracker membership awaits parent-owned shared index/roadmap registration.
Nonblocking follow-up: bitmap image loading has no timeout yet (before any input).

Source inspection and that export expose a blocking distinction:

- Offline5706 loads C and requests `CEntry` while the doorway is still arriving
  at(120,464);5707 moves to(120,463) with visible dialogue. Seventeen arrival
  updates remain. The UI pauses on dialogue, but acknowledge rejects an active
  transition.
- The Box-entry reload sample similarly requests dialogue at(122,105), before
  the mandatory arrival sample at(136,128); acknowledge rejects active motion.

These are **not** omittable no-ops. Parent must resolve the presentation contract
before final producer/HTML pinning. Neither hidden transition progress nor pot
recovery may be erased to make the route fit. No production fix is supplied here.

## Explicit cadence projection API

Inputs are the exact committed `tools/pandora-runtime-qualification/route.json`
(11590 actions; SHA256
`b969d6877f595ff811a0de8a912de307aaf5a3eb2b42e1720f2f5da6db53b830`), the
committed Pandora text reference, and a privately generated, independently
reviewed fresh replay proof. `projectRoute(route, proof)` is pure and does not
execute inputs or initialize the game.

```js
{
  schema: 1,
  provenance: {
    route_sha256: "...", rom_sha256: "...",
    start: "NewGame", policy: "SemanticPreview",
    tick_erasure: "global-tick-only"
  },
  offline: [
    {state: /* full GET-shaped fresh tick0 */, continuation: "..."},
    // then exactly one post-state for each original expanded action
  ],
  projected: [
    {state: /* independently replayed fresh tick0 */, continuation: "..."},
    // then exactly one post-state per emitted UI command
  ]
}
```

Both arrays must be produced by continuous public-input execution from
`freshGameState`/NewGame with the authenticated compiler, not by copying states,
restoring native checkpoints, interpolating positions, setting flags, or editing
snapshot clocks in the execution branch. Use the **same read-only projection**
as GET. Record compiler identity and generator/source revision alongside the
private proof; a JSON digest authenticates file identity, not truth or provenance.
The parent must review the producer and pin the resulting proof before use.

`continuation` is SHA256 of a **hashing copy** of the canonical320-byte,
version5/profile13 snapshot with only global tick bytes`[72,80)` zeroed. Check
header/version/profile/size first. Never restore that normalized copy. Keep
animation phase, walking/input history, all1024 flags, motion cursor, pot age,
source reservations and sheet mutations intact. The existing sparse runtime
semantic-event report does not prove this equality.

Projection omits only command0..4 where the **preceding observable dialogue is
non-null**, the canonical continuation hash is unchanged, and every observation
except `tick`/`snapshot_sha256` is unchanged. An unchanged non-dialogue neutral is
**retained**: ordinary cue clocks and all35 doorway ticks are not accelerated.
Every B/A, acknowledgement and choice remains in order. Every retained boundary
must equal the second fresh replay in continuation and non-clock observations.
Unemittable inputs fail rather than inventing acknowledgements or alternate paths.

The returned steps retain `{command, offlineTick, state}` with projected UI ticks;
omissions retain original indices/command/proof hash. Browser GET comparison
checks **every field**, including the projected replay's exact raw
`snapshot_sha256`, and the exact projected tick. Therefore the offline11590
snapshot hash is not asserted as the browser final identity after omissions.

## Controls, semantics and pixels

- Only actual New Game, Resume/Pause, keyboard arrows, Interact, Continue,
  explicit choice buttons and native A **Lift / throw (Z)** dispatch inputs.
  The verifier never calls a POST endpoint. GET/state and GET/art are inspection.
- Reuse house motion controls and the conversation sequential-ack latch/raster
  comparator. Each command is paced by the actual tick DOM mutation; pause before
  inspection. No CLI long-await: the browser retains its promise/status/result.
- Preserve existing house source art/scene/text checks on their exact subsets.
  Pandora metadata and all76 source-page indexed-byte SHA256 hashes are checked
  against the committed text reference, including source-only retry resources.
  Only visited pages/choices earn visible-raster evidence.
- Check all34 direct source requests in order, including repeated D720 requests;
  fixed grants at their offline boundaries; source semantic checkpoints; and
  final controlled map41 left/up/right/down movement square. All other GET fields
  (owner, graph invocation/cue/motion/locals/counter, world sheet and carry) must
  exactly match fresh replay, not reconstructed flag heuristics.
- Existing `RoomSlice.prepareArt/selectBackground/selectActors` validate source
  phase roster/order, finite world atlas and typed carry. A **separate compositor**
  compares every canvas pixel: last opaque OBJ wins before OBJ2/high-BG testing;
  OBJ3 is above high BG. It never invokes the production drawing/composition code.
- Require multicolor visible background evidence for all six source sheets,
  visible Ark, lifting/standing/walking/throwing and both miss/hit flight pixels,
  changed unoccluded wooden-door, damaged/open cellar-door and all three consumed
  pot cells. Offscreen, fully occluded, alpha-zero or base-identical patches do
  not count. No blind-screenshot acceptance.

Limits: source composition, not native whole RGB/shadows/color math/scheduler
frames. The route covers direct34, not optional refusal/retry playback. Endpoint
is controllable41, not equipment acquisition or a world return.

## Running

```sh
node --test tools/verify-{house,conversation,pandora}-browser.test.js
```

The parent must supply an enabled isolated origin and reviewed complete proof.
Do not start or replace its server. Use your own **named** browser session.
Set these globals on that real page before loading the verifier:

```js
PANDORA_BROWSER_READY = location.origin; // explicit assigned isolated origin
PANDORA_BROWSER_ROUTE_TEXT = /* exact route file bytes as a JS string */;
PANDORA_BROWSER_TEXT_REFERENCE = /* exact text reference bytes as a string */;
PANDORA_BROWSER_PROOF_TEXT = /* exact private proof bytes as a string */;
PANDORA_BROWSER_PROOF_SHA = /* independently reviewed proof file SHA256 */;
```

Load `verify-house-browser.js` with `HOUSE_BROWSER_HELPERS_ONLY=true`. To reuse
conversation's existing CommonJS helper seam **without starting its journey**,
wrap that source as follows (lexically hide `document` only within this helper
module; never alter the real page/global document):

```js
globalThis.ConversationBrowserHelpers = ((module, document) => {
  // exact tools/verify-conversation-browser.js source here
  return module.exports;
})({exports:{}}, undefined);
```

Then eval `tools/verify-pandora-browser.js` normally (`PANDORA_BROWSER_HELPERS_ONLY`
absent/false). It returns a short start message immediately. Inspect
`PANDORA_BROWSER_RUN.status`, `.lastTick`, `.offlineTick`, `.error`, then `.result`.
Its `.promise` stays in the page. Do not long-await it through the CLI. Any failure
retains evidence and stops input; it never reports a partial journey as passed.
Private full-state/raster/proof artifacts remain under ignored `local/`.
