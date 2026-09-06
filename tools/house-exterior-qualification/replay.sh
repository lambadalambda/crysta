#!/bin/sh
# Export only ROM source assets, then verify the conversation owner's captures.
# No event/navigation implementation lives here.
set -eu
if [ "$#" -ne 2 ]; then
    echo 'usage: replay.sh ROM CONVERSATION_CAPTURE_ROOT' >&2
    exit 2
fi
rom=$1
captures=$2
python3 -B tools/house-exterior-qualification/test_checks.py
python3 -O -B tools/house-exterior-qualification/test_checks.py
build=local/house-exterior-qualification/build
mkdir -p "$build/src"
cp tools/house-exterior-qualification/export.rs "$build/src/main.rs"
cat >"$build/Cargo.toml" <<'TOML'
[package]
name = "house-exterior-qualification"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
assets = { path = "../../../crates/assets" }
rom = { path = "../../../crates/rom" }
serde_json = "1"
TOML
cargo build --release --manifest-path "$build/Cargo.toml"
bin=${CARGO_TARGET_DIR:-$build/target}/release/house-exterior-qualification
out=$(mktemp -d local/house-exterior-qualification/run-XXXXXX)
"$bin" "$rom" "$out/export"
python3 -B tools/house-exterior-qualification/check.py "$rom" "$out/export" "$captures" >"$out/check.json"
python3 -O -B tools/house-exterior-qualification/check.py "$rom" "$out/export" "$captures" >"$out/check-optimized.json"
cmp "$out/check.json" "$out/check-optimized.json"
printf 'Exterior source/native qualification: %s\n' "$out"
