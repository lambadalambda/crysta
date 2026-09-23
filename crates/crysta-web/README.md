# crysta-web

The Crysta slice in a browser. The page runs the native app's session,
renderer and music (`crysta-app`'s library) as WebAssembly. Players select
their own Japanese ROM; it is read with `File.arrayBuffer()` and stays in
memory, never fetched, uploaded or stored.

## Build and run

Needs a clang and llvm-ar that target wasm32 (Homebrew's `llvm`, or the
distribution's `clang` and `llvm`) and `wasm-bindgen-cli` 0.2.126 at
`local/toolchain/bin/wasm-bindgen` (or `$WASM_BINDGEN`).

```sh
sh crates/crysta-web/build.sh
python3 -m http.server 8888 --bind 127.0.0.1 --directory local/crysta-web/site
```

Open <http://127.0.0.1:8888/>, select the ROM, press **Start** (browsers
start sound only after a click).

## Checks

```sh
cargo test --release --manifest-path crates/crysta-web/Cargo.toml
node crates/crysta-web/smoke.mjs local/crysta-web/site 'local/Tenchi Souzou (Japan).sfc'
```

The smoke check runs 600 frames and renders ten seconds of sound into
memory; nothing plays. `.github/workflows/pages.yml` publishes the site to
GitHub Pages on pushes to `main`.
