#!/bin/sh
# Exactly one empty-SRAM process; retains every checkpoint, never loads a state.
set -eu
rom=${1:-local/Tenchi Souzou (Japan).sfc}
python3 -B tools/house-conversation-qualification/test_check.py
python3 -O -B tools/house-conversation-qualification/test_check.py
sh tools/house-conversation-qualification/build.sh
probe=local/house-conversation-qualification/probe
binary=${CARGO_TARGET_DIR:-$probe/target}/release/house-conversation-probe
parent=$(mktemp -d local/house-conversation-qualification/replay-XXXXXX)
out=$parent/journey
printf 'Capture root: %s\n' "$out"
"$binary" "$rom" "$out" <tools/house-conversation-qualification/route.jsonl >"$out.jsonl"
python3 -B tools/house-conversation-qualification/check.py "$rom" "$out"
python3 -O -B tools/house-conversation-qualification/check.py "$rom" "$out"
printf 'Single fresh conversation/exterior journey verified: %s\n' "$out"
