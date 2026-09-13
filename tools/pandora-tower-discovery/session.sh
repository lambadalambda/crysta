#!/bin/sh
# Start an interactive discovery session at the accepted Pandora endpoint.
#
# Discovery-only. One empty-SRAM Session, input-only from boot, no warp, no
# memory patch, no save and no state restore. It replays the accepted route
# prefix verbatim and then holds the probe's stdin open so later shells can
# append exploration commands with step.sh instead of paying the prefix again.
set -eu
rom=${1:?usage: session.sh <japanese-rom>}
dir=${DISCOVERY_DIR:-local/pandora-tower-discovery/session}
case "$dir" in
  local/*) ;;
  *) echo "DISCOVERY_DIR must stay under local/: $dir" >&2; exit 1 ;;
esac
# Retire a previous session before unlinking its FIFO: the probe never exits on
# its own, because the holder keeps stdin open.
sh tools/pandora-tower-discovery/stop.sh 2>/dev/null || true
rm -rf "$dir"
mkdir -p "$dir"
fifo=$dir/in
mkfifo "$fifo"
probe=local/house-conversation-qualification/probe
binary=${CARGO_TARGET_DIR:-$probe/target}/release/house-conversation-probe
[ -x "$binary" ] || sh tools/house-conversation-qualification/build.sh
[ -x "$binary" ] || { echo "probe binary missing: $binary" >&2; exit 1; }
# Holder keeps a writer open so the probe never sees EOF between shells.
sleep 86400 >"$fifo" &
echo $! >"$dir/holder.pid"
"$binary" "$rom" "$dir/journey" <"$fifo" >"$dir/out.jsonl" 2>"$dir/err.log" &
echo $! >"$dir/probe.pid"
grep -v '^{"finish":true}$' tools/pandora-qualification/route.jsonl >"$dir/prefix.jsonl"
count=$(wc -l <"$dir/prefix.jsonl")
# Background: the prefix is far larger than the FIFO buffer, so this write only
# completes as the probe consumes it. Foregrounding it makes the script look
# hung for the whole replay and puts the probe in reach of a Ctrl-C.
cat "$dir/prefix.jsonl" >"$fifo" &
printf 'prefix queued (%s commands); probe pid %s\n' "$count" "$(cat "$dir/probe.pid")"
printf 'replay takes ~15 minutes; wait for %s checkpoints:\n' "$((count + 1))"
printf '  grep -c %s %s/out.jsonl\n' "'\"kind\":\"checkpoint\"'" "$dir"
