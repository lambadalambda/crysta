#!/bin/sh
# Builds the static site into local/crysta-web/site (or $1). Needs a clang
# and llvm-ar that target wasm32 (Homebrew's llvm, or the distribution's),
# and wasm-bindgen-cli 0.2.126 (local/toolchain/bin, or $WASM_BINDGEN).
set -eu
cd "$(dirname "$0")/../.."
: "${WASM_BINDGEN:=local/toolchain/bin/wasm-bindgen}"
: "${CC_wasm32_unknown_unknown:=$(brew --prefix llvm 2>/dev/null || echo /usr)/bin/clang}"
: "${AR_wasm32_unknown_unknown:=$(dirname "$CC_wasm32_unknown_unknown")/llvm-ar}"
export CC_wasm32_unknown_unknown AR_wasm32_unknown_unknown
site=${1:-local/crysta-web/site}
cargo build --release --locked --manifest-path crates/crysta-web/Cargo.toml \
  --target wasm32-unknown-unknown
rm -rf "$site"
mkdir -p "$site"
"$WASM_BINDGEN" --target web --no-typescript --out-dir "$site" \
  crates/crysta-web/target/wasm32-unknown-unknown/release/crysta_web.wasm
cp crates/crysta-web/www/index.html crates/crysta-web/www/main.js "$site/"
printf 'Built %s\nServe with: python3 -m http.server 8888 --bind 127.0.0.1 --directory %s\n' \
  "$site" "$site"
