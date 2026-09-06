#!/bin/sh
# Standalone host compiler: avoids the parent-owned main/module registration.
set -eu
[ "$#" -ge 1 ] && [ "$#" -le 2 ] || { echo 'usage: export.sh ROM [local/NEW-OUTPUT]' >&2; exit 2; }
rom=$(cd "$(dirname "$1")" && pwd)/$(basename "$1")
build=local/pandora-navigation-qualification/build
mkdir -p "$build/src"
printf '#[allow(dead_code)] // Host API is exercised by parent integration.\n#[path = "%s/crates/map-inspector/src/pandora_navigation.rs"]\nmod pandora_navigation;\n' "$PWD" > "$build/src/main.rs"
cat tools/pandora-navigation-qualification/export.rs >> "$build/src/main.rs"
cat > "$build/Cargo.toml" <<'EOF'
[package]
name = "pandora-navigation-qualification"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
assets = { path = "../../../crates/assets" }
rom = { path = "../../../crates/rom" }
room-core = { path = "../../../crates/room-core" }
serde_json = "1"
EOF
PANDORA_ROM="$rom" cargo test --quiet --manifest-path "$build/Cargo.toml" -- --include-ignored
if [ "$#" -eq 2 ]; then
    cargo run --quiet --manifest-path "$build/Cargo.toml" -- "$rom" "$2"
fi
