# Bounded browser-local extraction spike

Acceptance evidence for [the spike issue](../../meta/issues/validate-web-extraction-spike.md).
This is not a frontend, renderer, source qualification, or game-fidelity claim.

## Reproduce

Run from the repository root. Only hashes/metadata are public. Keep all generated
Wasm, glue, captures and local ROM inputs in ignored `local/`; **serve only
`local/web-spike/site/`, never the repository or `local/` root**.

```sh
rustup target list --installed                 # wasm32-unknown-unknown required
agent-browser skills get core --full          # mandatory CLI skill workflow
agent-browser --version
# If absent, local install; no global/system installation required:
cargo install wasm-bindgen-cli --version 0.2.126 --locked --root local/toolchain
mkdir -p local
ln -s '/absolute/path/to/owned/Tenchi Souzou (Japan).sfc' 'local/Tenchi Souzou (Japan).sfc'
cargo test --locked -p web-spike -- --nocapture
node --test tools/web-spike/run.test.mjs
sh tools/web-spike/build.sh 'local/Tenchi Souzou (Japan).sfc'
# Separate foreground terminal or agent service; leave room-preview:8765 alone:
python3 -u -m http.server 8876 --bind 127.0.0.1 --directory local/web-spike/site
```

The version-pinned wasm-bindgen dependency is Wasm-only, MIT/Apache-2.0. Its
matching CLI generates safe-to-call JS bindings without adding handwritten
unsafe code, wasm-pack, a bundler, npm dependencies, WASI, or browser frameworks.
All original Rust inherits the unchanged workspace `unsafe_code = "forbid"`.
Third-party binding internals are not an unsafe-free dependency guarantee.

Use an isolated browser session; the CLI's `upload` command selects a local file
on the input via browser automation, **not an HTTP upload**:

```sh
agent-browser --session wasm-spike open about:blank
agent-browser --session wasm-spike network har start
agent-browser --session wasm-spike open http://127.0.0.1:8876
agent-browser --session wasm-spike wait --fn 'document.querySelector("#result").textContent.startsWith("Ready.") && !document.querySelector("#rom").disabled'
agent-browser --session wasm-spike snapshot -i
agent-browser --session wasm-spike get text '#result'  # must say Ready
agent-browser --session wasm-spike network requests --json > local/web-spike/network-before.json
# Use the actual file-input ref from the fresh snapshot (observed @e2):
agent-browser --session wasm-spike upload @e2 "$PWD/local/Tenchi Souzou (Japan).sfc"
agent-browser --session wasm-spike wait --fn '(() => { try { return ["pass", "fail"].includes(JSON.parse(document.querySelector("#result").textContent).status); } catch { return false; } })()'
agent-browser --session wasm-spike get text '#result' > local/web-spike/browser-cold.json
agent-browser --session wasm-spike network requests --json > local/web-spike/network-after.json
agent-browser --session wasm-spike network har stop local/web-spike/browser.har
agent-browser --session wasm-spike screenshot local/web-spike/browser.png
```

Inspect the HAR and generated bindings, not merely server access logs. This
minimal check compares actual browser/native reports and all captured requests:

```sh
python3 - <<'PY'
import json
from pathlib import Path
p = Path('local/web-spike')
load = lambda name: json.loads((p/name).read_text())
b = load('browser-cold.json')
assert b['status'] == 'pass' and b['native_match']
assert b['report'] == load('site/native.json')
assert load('network-before.json') == load('network-after.json')
entries = load('browser.har')['log']['entries']
paths = {'/', '/main.mjs', '/run.mjs', '/web_spike.js', '/web_spike_bg.wasm', '/native.json'}
assert len(entries) == len(paths) == 6
assert {e['request']['url'] for e in entries} == {'http://127.0.0.1:8876'+x for x in paths}
for e in entries:
    r = e['request']
    assert r['method'] == 'GET' and r['bodySize'] == 0 and not r.get('postData')
print('native/Wasm match; six startup GETs; zero selection requests/body bytes')
PY
node --input-type=module - <<'JS'
import fs from 'node:fs';
const module = new WebAssembly.Module(fs.readFileSync('local/web-spike/site/web_spike_bg.wasm'));
console.log(WebAssembly.Module.imports(module));
JS
```

For warm samples, start another HAR, snapshot then reselect the same file twice,
and save `#result` as `browser-warm-1.json` and `browser-warm-2.json`. Test rejection
by selecting a local synthetic 4 MiB zero file; save `browser-invalid.json`.
After **every** selection, repeat the terminal pass/fail predicate wait above
before saving output or stopping the HAR. These CLI waits have bounded timeouts;
a timeout is a failed smoke run, not evidence. Do not substitute fixed sleeps.
The recorded `reselection.har` has zero entries across all three selections.
Close **only this session** with `agent-browser --session wasm-spike close`.

## Observed evidence (2026-09-06)

Code: `cd7a8f9`. Rust 1.97.0/LLVM 22.1.6; native `aarch64-apple-darwin`,
macOS 26.5.2 ARM64; agent-browser 0.30.1, HeadlessChrome 150.0.0.0. The browser's
legacy user-agent says Intel/Mac OS X 10_15_7; it is not host hardware evidence.
Release Wasm is 70,614 bytes, SHA-256
`693a41ee825cd5ab2cb04adffd7947ec25a3184470581e19c0e8a28c935b2c7f`.

- Owned input is a symlink to the parent's ignored JP dump, 4,194,304 bytes.
  `Rom::load` authenticates its normalized SHA-256 before the explicit JP gate.
- Existing intro Earth packet, normalized offset `0x2D0000`: 18,112 consumed
  bytes → 32,768 output bytes (32 KiB decoder budget). Output SHA-256 matches
  the independently established codec fixture in `crates/assets/tests/local_roms.rs`:
  `e61b2cb1d7f0e37b9610073a2004513317547e78ade2f978a8a6e9a43f98e4ce`.
- Replay is the existing room-core six-frame walking test: a 32×64 open grid,
  start `(104,112)`, inputs Left/neutral/neutral/Down/Down/Down, snapshot restore
  after each step. Canonical final 16-byte snapshot SHA-256:
  `5794285e77e705ad7f20335be388a6efd1b81c9c40a5cc89854042fd74db82e5`.
  SHA-256 of all six concatenated snapshots:
  `c4512040a2288ee874b1d9815005bd5be4ea0fd12ba11810b2ce233374911e5a`.
  Both are pinned in ROM-free tests and identical in actual native and browser runs.
- Browser startup compares replay before enabling input; each selection compares
  the entire metadata report against freshly generated native metadata. Valid
  headered normalization also passes the local Rust test. The browser rejects
  the synthetic same-size invalid image and resets/re-enables its file control.
- Browser HAR: exactly six startup GETs to the listed static paths, zero request
  body bytes, no query strings or other origins. Before/after request logs are
  byte-identical. Generated JS and measured pipeline inspection finds no
  selection-time networking. This is observed page traffic plus source
  inspection, not an OS-wide packet capture, extension audit, or proof against
  compromised same-origin code. CSP still allows same-origin startup fetches.

### Time and memory — not a benchmark or total-peak claim

| Selection | File read ms | Probe ms | Combined ms | Wasm capacity before → after |
| --- | ---: | ---: | ---: | ---: |
| First on fresh instance | 2.5 | 90.5 | 93.0 | 1,179,648 → 9,699,328 bytes |
| Reselection 1 | 4.3 | 18.3 | 22.6 | 9,699,328 → 9,699,328 bytes |
| Reselection 2 | 2.0 | 15.1 | 17.1 | 9,699,328 → 9,699,328 bytes |

Three native release-process probes: 25.272, 28.636, 27.545 ms, excluding file
read/startup. They include validation, decoding, output hashing, and replay;
Wasm probe timing additionally includes binding input copying and JSON parsing.
No statistics, speedup, mobile performance, or worst-case claims from these
few uncontrolled desktop samples. First selection is not cold disk/browser cache.

Wasm linear capacity high-water is **9.25 MiB**, not live Rust allocations.
A nonshared instance's linear memory cannot shrink, so its post-call capacity
captures growth even during synchronous processing; warm values include prior
runs. `performance.memory.usedJSHeapSize` reported 12,700,000 both before and
after every run: coarse, optional, Chromium-only samples, **not peak or delta**,
and not a portable accounting of external ArrayBuffers, file backing, browser
processes, compilation, or allocator slack. `browser_total_peak_bytes` is null.
The explicit copy model includes a JS file ArrayBuffer, a Wasm input copy and a
normalized Rust copy; linear capacity alone must not be used as a browser budget.

`startup_ms` was 2.9 ms: Wasm initialization plus native JSON fetch/parse only.
It excludes initial module loading and the subsequent startup replay check;
it is not page-ready latency. No true browser total-peak measurement was obtained.

## Architecture constraints and fallbacks

- Keep validation/decoding/replay host-free. This compiled module imports only
  externref-table initialization and string conversion from the generated glue;
  no filesystem, clock, network, thread, or host-runtime imports. Actual memory
  buffer is `ArrayBuffer`, not shared; `crossOriginIsolated` is false.
- Async `File.arrayBuffer()` is the browser file boundary. The native harness's
  blocking file read and clock are native-only, not linked browser requirements.
  Size admission precedes reading/copying; known-ROM authentication precedes decode.
- Require modern ES modules, Wasm, TextDecoder and File.arrayBuffer. No legacy
  FileReader/UI fallback is implemented. Caught initialization errors display a
  startup error with input disabled; unsupported module/API environments have
  no guaranteed fallback/error UI (missing File.arrayBuffer fails on selection).
  wasm-bindgen's generated loader uses streaming instantiation
  with an ArrayBuffer fallback for wrong Wasm MIME; fallback was inspected, not
  separately exercised. Serve `application/wasm` as in this run.
- This tiny synchronous main-thread probe visibly blocks for up to the observed
  90.5 ms. It proves neither responsive full extraction nor a production memory
  budget. Larger work needs a separately scoped scheduling/memory decision,
  not automatic adoption of threads, blocking filesystem APIs or shared memory.
- No cache, service worker, persistent storage, audio, rendering, controller,
  room-preview integration or full-core portability qualification is included.

## Retention and checks

Ignored `local/web-spike/` retains `acceptance.json` (assertion summary and
SHA-256 manifest), both HARs, request logs, three browser successes, invalid-image
failure, runtime inspection, native reports/times, screenshot, generated
artifacts and server log. HAR SHA-256:
`c002995a728f352cc8c702dd76f0721b9e931ca74a4197ea826e34c4969c4f1a`.
No captures, raw ROM/decoded assets, or Wasm binaries are committed.

TDD: three Rust tests failed against stubs then passed, including the real JP
packet/normalization test. JS tests failed on missing adapter then passed;
five now cover comparison, size admission, invalid ROM, file-read failure,
malformed JSON, and headered-size admission. Review added the pinned trace hash
and remaining cheap adapter failure tests. Native/Wasm compilation, workspace
Clippy (`-D warnings`), workspace tests, tracker and repository safety checks
pass. Optional unrelated ROM-backed tests still skip absent inputs.
`cargo fmt -p web-spike -- --check` passes; the full workspace fmt check finds
pre-existing formatting in assets sprites/tests and map-inspector module order.
Those unrelated files are deliberately untouched. Independent code review found
no must-fix issues; total browser peak remains explicitly unmeasured.


## Independent main-checkout acceptance

The parent reproduced the build and actual file-input workflow after integration.
The Wasm artifact hash and complete native/browser report match the run above.
Main `local/web-spike/parent-acceptance.json` binds the parent HARs and reports:
six bodyless startup GETs, zero selection requests, and zero requests across a
valid reselection plus invalid-ROM rejection. Parent read/probe times were
2.4/84.0 ms cold and15.3 ms for the warm probe; the same9.25 MiB linear-capacity
high-water was observed. These are further samples, not a benchmark or a new
browser-total-memory measurement. The runtime inspection confirmed nonshared
ArrayBuffer memory. All339 main-workspace tests and native/Wasm Clippy passed.
Both parent spike service and isolated browser session were stopped afterward.
