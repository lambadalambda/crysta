#!/bin/sh
# Reuse two existing native journeys. No emulator, boot, new capture, or RGB readback.
set -eu
if [ "$#" -ne 3 ]; then
    echo 'usage: compare.sh ROM SOURCE_JOURNEY PARENT_JOURNEY' >&2
    exit 2
fi
python3 -B tools/pandora-background-qualification/test_check.py
python3 -O -B tools/pandora-background-qualification/test_check.py
build=local/pandora-background-qualification/build
mkdir -p "$build/src"
cp tools/pandora-background-qualification/export.rs "$build/src/main.rs"
cat >"$build/Cargo.toml" <<'TOML'
[package]
name = "pandora-background-qualification"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
assets = { path = "../../../crates/assets" }
rom = { path = "../../../crates/rom" }
serde_json = "1"
TOML
out=$(mktemp -d local/pandora-background-qualification/run-XXXXXX)
cargo run --quiet --release --manifest-path "$build/Cargo.toml" -- "$1" "$out/export"
python3 -B tools/pandora-background-qualification/check.py "$1" "$out/export" "$2" "$3"
python3 -O -B tools/pandora-background-qualification/check.py "$1" "$out/export" "$2" "$3"
printf 'Pandora background comparison: %s\n' "$out"
