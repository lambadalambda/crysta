#!/bin/sh
# Run at repository root. Raw native output stays local; each probe is a fresh process.
set -eu
export PYTHONDONTWRITEBYTECODE=1
cargo test -p room-core --test animation
root=local/player-animation-qualification
for kind in probe compare; do
    mkdir -p "$root/$kind/src"
    cp "tools/player-animation-qualification/$kind.rs" "$root/$kind/src/main.rs"
    cp tools/new-game-qualification/bootstrap.rs "$root/$kind/src/"
    cp crates/room-core/src/animation.rs "$root/$kind/src/"
    cat >"$root/$kind/Cargo.toml" <<TOML
[package]
name = "animation-$kind"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
oracle = { path = "../../../crates/oracle" }
rom = { path = "../../../crates/rom" }
room-core = { path = "../../../crates/room-core" }
serde_json = "1"
TOML
    cargo build --release --manifest-path "$root/$kind/Cargo.toml"
done
out=$(mktemp -d "$root/replay-XXXXXX")
python3 tools/player-animation-qualification/replay.py "${1:-local/Tenchi Souzou (Japan).sfc}" "$out"
printf 'Two fresh boots per plan reproduced animation pins. Private captures: %s\n' "$out"
