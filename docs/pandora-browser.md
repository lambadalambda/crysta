# Pandora input-only browser verifier

[Owned issue](../meta/issues/verify-pandora-browser-journey.md).
This is a bounded semantic-preview acceptance tool, not a native-frame replay.
The production host/controller/compiler remain parent-owned.

## Browser-local Wasm preview stage

The current source-derived preview can now run from a user-selected local ROM
without `/state`, `/step`, `/new-game`, `/reset`, `/art.json`, or bitmap backend
requests. The generated static app and exact commands are documented in
[`tools/pandora-preview/README.md`](../tools/pandora-preview/README.md).
A stateful `wasm-bindgen` facade owns `map_inspector::PandoraPreview` in a Web
Worker; JavaScript only maps the existing closed endpoint protocol and converts
owned art/BMP copies into the same renderer inputs. It does not implement game
state, progression, art selection, camera policy, or input cadence.

The shared controller remains unchanged in meaning: visible dialogue is blocking
only when `dialogue_ready !== false`; visible-unready arrivals retain neutral
Resume updates and disable acknowledgement. Interact5, acknowledgement6,
choices7..9, and manual A/pot action10 remain distinct non-repeating commands.
The native loopback transport remains the default when no runtime is injected.

A bounded local smoke verified load, explicit New Game, movement, interaction,
reset, invalid replacement, recovery, and zero gameplay/backend network requests.
It is not a replay or renewal of the accepted continuous journey below, and it
does not update its proof, producer, canvas, or output pins.

## Full browser-local Wasm acceptance

The bounded browser runtime is qualified separately from the general browser ROM
bootstrap and WebAssembly release work, which remain open. The full verifier uses
the existing independently reviewed native cadence proof as expectation data and
observes the worker only through state, art, and background reads. New Game and
every subsequent input are dispatched through the actual shared UI controls; the
verifier has no step, reset, New Game, parity-replay, or generic worker-request
capability.

The local verifier fails closed when the injected runtime is missing, malformed,
stalled, or returns a non-Blob background. Native HTTP inspection remains
available only when no runtime was requested. Both paths retain the same complete
field, raw-snapshot, text-raster, source-semantic, and independent full-canvas
checks.

The accepted local run used a real `File` selection at
`http://127.0.0.1:8890/`, authenticated the owned 4 MiB Japanese ROM, observed the
saved checkpoint, and clicked New Game explicitly. The static server was stopped
after the verifier and worker had loaded; the journey continued to completion
without it. The final HAR contains five static startup records and twelve browser-local Blob
image reads. A cumulative session request capture additionally contains one
expected favicon 404; neither capture has a POST, ROM upload, gameplay endpoint,
or non-loopback network origin.

| Browser-local Wasm acceptance | Result |
|---|---|
| Original / projected inputs | 11590 / **11409** |
| Proved blocking-dialogue omissions | **181** |
| Retained visible-unready updates | **18** |
| Full 256×224 canvas comparisons | **11410**, including New Game tick 0 |
| Semantic/readiness checkpoints | **187** |
| Direct source invocation order | **34** |
| Final state | tick11409, map`$0041`,(136,208), walking/player, no dialogue/error |
| Final projected snapshot | `8aab7a37cb0c1115438e8f177c7213fabb0cee14f94d7eaed0cf58635c7484d5` |

Browser bootstrap measured a 362.3 ms file read and 656.7 ms worker compile.
Wasm linear-memory capacity was 289,734,656 bytes; this is not total browser
peak and excludes JavaScript/browser copies, decoded canvases, and process
memory. The generated release Wasm was 1,222,939 bytes. Generated site and full
run evidence remain ignored under `local/` and contain no ROM or extracted
asset files.

Current source identity was recorded, and every projected Wasm observation
matched the unchanged reviewed native expectation. The prior native HTTP full journey was
not rerun: legacy coverage in this stage is the unchanged house, conversation,
Pandora helper suites and shared native/injected UI regressions. The prior
accepted HTTP journey and its evidence remain the native full-route acceptance.
The strict current-producer source gate remains deliberately red pending the
separate producer revalidation issue; no source or output pin is renewed here.

The accepted verifier was signed commit `2593b3d`. Private evidence is retained
under this worktree's ignored `local/pandora-browser-wasm/`:

- `full-result.json` SHA256
  `3667755a93cffdc21fab90597be5545e2312de7608b59745b47b2df34e3c669a`;
- independent final runtime state SHA256
  `6a8a28d4569369d28fbd0b37ad12a4cb8cee88d11d553945389da1f3c161ccf2`;
- network HAR SHA256
  `691cb416337ef822c632e9c394440ca98e2ed2212433be88bb87b92e89c0736a`;
- audit SHA256
  `b3d9684190a27f7034668d4f8e4e14bac9110f10346c3070e44b38ab2567687e`;
- complete provenance manifest SHA256
  `7940168ac78f692ff3c82eb250be9483e72c33121cf350cdffeb232638a42488`.

Reproduce on an isolated port by building as documented in
`tools/pandora-preview/README.md`, serving the generated site on `8890`, starting
a named `agent-browser` HAR capture, selecting the ROM through `#local-rom`, and
loading the three verifier helpers plus the reviewed proof as documented in this
file's Running section. Retain the returned promise in the page and use one
completion wait rather than a long evaluator call or repeated status polling.

## Accepted continuous browser journey

**The authorized New Game → final controllable map41 browser run passed.** The
named session `pandora-journey-alice` reloaded the parent's reviewed readiness UI
at `http://127.0.0.1:8877/` and clicked actual New Game. The saved-checkpoint startup
was not used. Live8765 and production files were untouched. The browser retained
its promise; CLI eval returned immediately. No state initialization, direct POST,
forced acknowledgement, inferred flag or disabled dialogue pause was used.

| Measured acceptance | Result |
|---|---|
| Original / projected inputs | 11590 / **11409** |
| Proved blocking-dialogue omissions | **181**:60 Right +121 neutral |
| Retained visible-unready updates | **18**: C17 + Box1 |
| Full-canvas comparisons | **11410**, including fresh tick0 |
| Saved semantic-change/readiness checkpoints | **187** |
| Direct source invocation order | **34**, including all repeated D720 requests |
| Explicit manual inputs retained |6 Interacts,80 acknowledgements,4 choices,6 pot actions |
| Visible distinct rasters |81 text keys across house/Pandora +8 choice crops |
| Source-only Pandora raster hashes checked |All76, including unvisited retry resources |
| Final controlled position |Map`$0041`,(136,208), walking/player, no dialogue/error |

Every emitted result matched **all** projected GET fields, including its exact raw
snapshot hash. Fixed grant timing, ordinary35-tick transfers, pot miss/hits and
recovery, persistent patches/reloads, all mandatory acknowledgements, and final
left/up/right/down movement checks remained active. A post-run GET independently
matched the saved final state; DOM showed11409/`$0041`/`136, 208` and **Resume**,
with no error and the promise still retained. This is one continuous browser run,
not a second native replay or optional refusal/retry acceptance.

Nonvacuous visible pixels include Ark **4,060,990**; all six backgrounds; all
required wooden, damaged/open cellar and three consumed-pot patches. Carry
counts are lifting10,902, standing255,348, walking44,736, throwing8,532,
miss-flight774 and hit-flight206. Counts are accumulated visible samples, not
unique source pixels. Cellar-sheet patch samples that remained offscreen scored
zero and are **not** claimed as visible; their retained state still matched GET.

### Reproduction pins and private evidence

The run used signed verifier commit **`b897899`**, unchanged during execution.
The parent independently qualified the two fresh cadence replays and authorized
this exact expectation-only proof:

- Proof: `/Users/lainsoykaf/repos/terranigma/local/pandora-cadence-qualification/parent-reviewed/debug/proof.json`
- Proof SHA256: `e5342dc5e6965d298e4e9a6d142e55a2161fac0ab247f143a098016c97465ab6`
- Producer source: `9db17d9373740d1084983d047817d6d37c2d54d6`
- Source tree: `705afd0220bff8d9a30faf14e633a7770c2f3401`
- Compiler content: `c6613fd4b30db083deb229964cd035bbd30d27f2d47a72d6f70530d919237be4`
- **Projected final snapshot**: `8aab7a37cb0c1115438e8f177c7213fabb0cee14f94d7eaed0cf58635c7484d5`

Private files under this worktree's `local/pandora-browser/`:

- `full-result.json`: complete retained result/checkpoints/pixel/raster/omission
  evidence; SHA256`93e899c58198646ab89b96f1507869c5f2a18e9313e31424306c0f74b3e0075b`.
- `full-final-get.json`, `full-final-dom.json`: independently inspected paused
  endpoint; `full-progress.jsonl`: read-only monitor observations.
- `full-manifest.json`: exact proof, evaluator, helper, route/reference and served
  HTML hashes. Served HTML SHA256 is
  `d4103bcfee965378ced16ee9bc74b2c04f0b1a7794e4802a877cb4c354744271`.
- `full-eval.js`: exact launched script; `served-room.html`: observed UI source;
  `audit.js` / `full-audit.json`: post-run consistency checks and artifact hashes,
  **not another browser run**. Every saved checkpoint equals its fresh projected
  expectation, and all manual-input counts are unchanged.

The17 Pandora synthetic tests and32 combined house/conversation/Pandora tests
pass, including red→green readiness adaptation tests. Parent owns final producer
pinning and shared issue-index/archive follow-through. Nonblocking follow-up:
bitmap image loading has no timeout yet (before any input).

### Historical prefix and readiness resolution

The earlier unprojected diagnostic stopped after904 successful actual UI/state/
canvas comparisons, before blocked movement905. It remains failed/partial
historical evidence (`prefix-result-cli.json`, `prefix-final-state.json`,
`prefix-eval.js`), **not** the accepted run or an extra pass. Reloading for the
accepted run replaced that page's old promise.

Source inspection exposed a visibility/readiness distinction:

- Offline5706 loads C and requests `CEntry` while the doorway is still arriving
  at(120,464);5707 moves to(120,463) with visible dialogue. Seventeen arrival
  updates remain. The old UI paused on any dialogue, but acknowledge rejects an
  active transition.
- The Box-entry reload sample similarly requests dialogue at(122,105), before
  the mandatory arrival sample at(136,128); acknowledge rejects active motion.

These are **not** omittable no-ops. The parent resolution preserves the core
request/timeline and transports read-only `GameState::dialogue_input_ready()` as
opt-in boolean `state.dialogue_ready`: the existing acknowledgement guard
(transition absent and motion absent), not a second inferred story state.
Visible text remains painted, including when readiness is false. Only
`dialogue != null && dialogue_ready !== false` blocks ordinary input. While
visible-but-unready, Resume/neutral arrival updates remain available and all
acknowledgement/choice buttons are disabled. The verifier retains these arrival
commands and uses the existing transition-neutral UI driver; it neither hides
requests nor disables ready-dialogue pauses. Legacy undefined readiness retains
the old blocking behavior; enabled browser acceptance requires the boolean.

The canonical proof and accepted browser run confirmed181 unchanged
blocking-dialogue omissions and17 C-arrival plus1 Box-completion update retained
(offline5707..5723 and9764). These counts are **not omission permissions** and
remain absent from projection logic; each omission requires its own unchanged
continuation/observation proof and a separate fresh replay of retained inputs.

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

Projection omits only command0..4 where the **preceding dialogue is blocking**
(`dialogue != null && dialogue_ready !== false`), the canonical continuation hash
is unchanged, and every observation except `tick`/`snapshot_sha256` is unchanged.
Readiness itself is compared, never erased. An unchanged ordinary neutral or a
visible-but-unready input is **retained**, even if its continuation is identical:
ordinary cue clocks and all35 doorway ticks are not accelerated.
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
