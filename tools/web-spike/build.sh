#!/bin/sh
# Run from the repository root. Generated files stay in ignored local/.
set -eu
: "${WASM_BINDGEN:=local/toolchain/bin/wasm-bindgen}"
rom=${1:?usage: sh tools/web-spike/build.sh path/to/owned-jp-rom}
[ "$("$WASM_BINDGEN" --version)" = 'wasm-bindgen 0.2.126' ] || {
  echo 'Install matching wasm-bindgen-cli 0.2.126 under local/toolchain' >&2
  exit 1
}
cargo build --locked -p web-spike --lib --release --target wasm32-unknown-unknown
mkdir -p local/web-spike/site
"$WASM_BINDGEN" --target web --out-dir local/web-spike/site \
  target/wasm32-unknown-unknown/release/web_spike.wasm
cp tools/web-spike/index.html tools/web-spike/main.mjs tools/web-spike/run.mjs local/web-spike/site/
cargo run --locked --quiet --release -p web-spike -- "$rom" > local/web-spike/site/native.json
