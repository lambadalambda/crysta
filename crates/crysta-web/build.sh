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
# The Rust that cargo runs here must have the wasm32 standard library.
if [ ! -d "$(rustc --print sysroot)/lib/rustlib/wasm32-unknown-unknown" ]; then
  echo "$(command -v rustc) has no wasm32-unknown-unknown target." >&2
  echo 'Add it (rustup target add wasm32-unknown-unknown), or put a rustup' >&2
  echo 'toolchain that has it first on PATH.' >&2
  exit 1
fi
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
