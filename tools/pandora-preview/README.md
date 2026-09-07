# Browser-local Pandora preview

This bounded adapter runs the accepted source-derived Pandora preview in a Web
Worker through `wasm-bindgen` 0.2.126. ROM bytes come from `File.arrayBuffer()`,
are transferred directly to the worker, and are never fetched, uploaded, logged,
or persisted. The existing room controller and renderer remain shared with the
native loopback host.

## Build and run

Install the matching `wasm-bindgen` CLI at the ignored path
`local/toolchain/bin/wasm-bindgen`, then run from the repository root:

```sh
sh tools/pandora-preview/build.sh
python3 -m http.server 8888 --bind 127.0.0.1 \
  --directory local/pandora-preview/site
```

Passing the owned ROM as the optional build argument emits an ignored native
expectation for the focused actual-Wasm parity check:

```sh
sh tools/pandora-preview/build.sh 'local/Tenchi Souzou (Japan).sfc'
```

Open <http://127.0.0.1:8888/>, select the owned Japanese 4 MiB ROM (a 512-byte
copier header is also accepted), then choose **New Game** explicitly. Selection
constructs the saved checkpoint first; it never silently starts New Game.
Replacing the file immediately invalidates and frees the old facade and clears
its projected presentation. A malformed replacement leaves controls disabled
until another valid ROM is compiled. Wasm/JS allocators do not promise secure
zeroization after free; this stage promises no upload or persistence, not
cryptographic erasure.

The generated site is ignored. It contains reconstructed source code, generated
Wasm/glue, and no ROM or extracted asset files. `runtime-loader.js` injects only
an I/O adapter: endpoint-shaped state commands and source art/background buffers
are serviced by Rust in the worker. Native HTTP fallback remains the default
when that adapter is absent.

## Checks

```sh
cargo test --locked -p pandora-web
cargo clippy --locked --target wasm32-unknown-unknown -p pandora-web --lib -- -D warnings
cargo build --locked --release --target wasm32-unknown-unknown -p pandora-web --lib
! cargo tree --locked --target wasm32-unknown-unknown -p pandora-web \
  --edges normal,build | grep 'oracle v'
node --test tools/pandora-preview/bootstrap.test.mjs
node crates/map-inspector/tests/room-slice-check.js
```

## Measured bounded smoke

On the local qualification machine, Chromium compiled the authenticated profile
in a dedicated worker in 540–657 ms after a 346–370 ms file read. Reported Wasm
linear-memory capacity was 289,734,656 bytes. This is **not** total browser peak
memory: temporary browser/JS copies, decoded canvas storage, and process overhead
are not measured. The release Wasm was 1,222,437 bytes before HTTP compression.

The smoke covered valid load, explicit New Game, rightward movement, an explicit
off-target interaction, checkpoint reset, malformed replacement, valid recovery,
and an empty browser request log during reset/New Game. Only static startup GETs
reached the loopback server; Blob image reads are browser-local. A separate
5,707-command generated-Wasm/native comparison matched complete state, art, and
all six BMP hashes through real interactions, acknowledgements, choices, and the
map12 visible-unready arrival. At tick5,707 an acknowledgement was an exact
no-op; neutral input advanced arrival to tick5,708 while dialogue remained
visible and unready. This is not the accepted full 11,409-input journey.
