#!/bin/sh
# Compile caller-owned ROM; never navigate or operate a reference core.
set -eu
rom=${1:-local/Tenchi Souzou (Japan).sfc}
probe=local/pandora-dialogue-qualification/probe
mkdir -p "$probe/src"
cp tools/pandora-dialogue-qualification/export.rs "$probe/src/main.rs"
cat >"$probe/Cargo.toml" <<'TOML'
[package]
name = "pandora-dialogue-export"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
rom = { path = "../../../crates/rom" }
assets = { path = "../../../crates/assets" }
serde_json = "1"
TOML
cargo build --release --manifest-path "$probe/Cargo.toml"
root=$(mktemp -d local/pandora-dialogue-qualification/export-XXXXXX)
"${CARGO_TARGET_DIR:-$probe/target}/release/pandora-dialogue-export" "$rom" "$root/pages"
python3 -B tools/pandora-dialogue-qualification/check.py "$rom" "$root/pages"
python3 -O -B tools/pandora-dialogue-qualification/check.py "$rom" "$root/pages"
printf '%s\n' "$root"
