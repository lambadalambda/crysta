#!/bin/sh
# Build the browser-local Pandora preview under ignored local/.
set -eu
: "${WASM_BINDGEN:=local/toolchain/bin/wasm-bindgen}"
[ "$("$WASM_BINDGEN" --version)" = 'wasm-bindgen 0.2.126' ] || {
  echo 'Install matching wasm-bindgen-cli 0.2.126 under local/toolchain' >&2
  exit 1
}
cargo build --locked -p pandora-web --lib --release --target wasm32-unknown-unknown
site=local/pandora-preview/site
rm -rf "$site"
mkdir -p "$site"
"$WASM_BINDGEN" --target web --out-dir "$site" \
  target/wasm32-unknown-unknown/release/pandora_web.wasm
cp tools/pandora-preview/bootstrap.mjs tools/pandora-preview/main.mjs \
  tools/pandora-preview/worker.mjs tools/pandora-preview/parity.mjs \
  tools/pandora-preview/parity-actions.json tools/pandora-preview/runtime-loader.js "$site/"
if [ "$#" -gt 0 ]; then
  cargo run --locked --quiet --release -p pandora-web --example parity -- "$1" > "$site/native-parity.json"
fi
python3 - "$site/index.html" <<'PY'
from pathlib import Path
import sys
source = Path('crates/map-inspector/web/room-slice.html').read_text()
marker = '</head>'
assert source.count(marker) == 1
source = source.replace(marker, '<script src="./runtime-loader.js"></script>\n' + marker)
Path(sys.argv[1]).write_text(source)
PY
printf 'Built %s\nServe with: python3 -m http.server 8888 --bind 127.0.0.1 --directory %s\n' "$site" "$site"
