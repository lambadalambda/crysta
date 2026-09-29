# Web replay

Replays a crysta-web route trace through a real page (by default the deployed
<https://lambadalambda.github.io/crysta/>) in headless Chromium. It selects the
owned ROM through the page's file input (the page reads it locally and never
uploads it), presses Start, replaces `requestAnimationFrame` with a queue that
it pumps one console frame at a time, and sends each frame's buttons as
keydown/keyup events to the page's own handlers. Sound is rendered into a
silent `AudioContext` stub and never played.

For each frame it checks what the page passed to `WebGame.frame_with_presses`
against the trace (same held bits and action edges), and stops on a fault,
an error in the status line or a stopped frame loop. At each named checkpoint
it hashes the canvas (FNV-1a 64) and compares the hash with the Rust route's
view at the same frame.

```sh
T=local/web-replay
mkdir -p $T
CRYSTA_WEB_EU_TRACE=$PWD/$T/eu-door.trace CRYSTA_WEB_EU_DETOUR_TRACE=$PWD/$T/eu-detour.trace \
  cargo test --release --manifest-path crates/crysta-web/Cargo.toml european_route
node tools/web-replay/replay.mjs --trace $T/eu-door.trace
node tools/web-replay/replay.mjs --trace $T/eu-detour.trace --shots $T/detour
node --test tools/web-replay/trace.test.mjs
```

Options: `--url` (for example a local build served from `local/crysta-web/site`),
`--rom` (default `local/Terranigma (E) [!].smc`), `--chrome` or `$CHROME`
(default: Playwright's cached `chrome-headless-shell`). `--shots` saves a PNG
of the canvas at each checkpoint and must be under `local/`: the pictures show
the game's art. `--step-timeout` (default 30000 ms) bounds each browser call
and wait; `--timeout` (default 600000 ms) bounds the whole run. On a timeout
the replay says what did not answer, closes Chromium and exits 1. The exit
status is 0 when every checkpoint matches, 1 when one differs or a timeout
hits, 2 when the replay stops for another reason.

Chromium runs with `--no-sandbox` because its own sandbox cannot start inside
a sandboxed agent; it uses a throwaway profile.
