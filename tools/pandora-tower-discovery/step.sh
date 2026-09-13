#!/bin/sh
# Append one exploration command to a running session and report the result.
# usage: step.sh LABEL FRAMES [BUTTON...]
set -eu
dir=${DISCOVERY_DIR:-local/pandora-tower-discovery/session}
label=${1:?usage: step.sh LABEL FRAMES [BUTTON...]}
frames=${2:?frames}
shift 2
buttons=$(for b in "$@"; do printf '"%s",' "$b"; done | sed 's/,$//')
[ -p "$dir/in" ] || { echo "no session FIFO at $dir/in" >&2; exit 1; }
# Opening the FIFO for writing blocks until a reader exists, so a dead probe
# would hang here rather than in the timeout loop below. Check it first.
[ -f "$dir/probe.pid" ] || {
  echo "no running session: $dir/probe.pid is missing (start one with session.sh)" >&2
  exit 1
}
pid=$(cat "$dir/probe.pid")
kill -0 "$pid" 2>/dev/null || {
  echo "probe $pid is not running; last error output:" >&2
  tail -5 "$dir/err.log" >&2
  exit 1
}
# Resume scanning from the current end of the per-frame log. The probe prints a
# line per frame, so out.jsonl reaches tens of megabytes and rescanning it from
# the start makes every poll progressively slower.
offset=$(( $(wc -c <"$dir/out.jsonl") + 1 ))
printf '{"label":"%s","buttons":[%s],"frames":%s}\n' "$label" "$buttons" "$frames" >>"$dir/in"
# Wait for this label's own checkpoint, not merely for the checkpoint count to
# rise: another command still draining would otherwise satisfy the wait and we
# would read a stale or missing capture. serde_json emits keys alphabetically.
needle="\"kind\":\"checkpoint\",\"label\":\"$label\""
i=0
until tail -c "+$offset" "$dir/out.jsonl" | grep -q "$needle"; do
  i=$((i + 1))
  if [ "$i" -gt 300 ]; then
    echo "timeout waiting for $label" >&2
    tail -5 "$dir/err.log" >&2
    exit 1
  fi
  kill -0 "$pid" 2>/dev/null || {
    echo "probe $pid died waiting for $label:" >&2
    tail -5 "$dir/err.log" >&2
    exit 1
  }
  sleep 0.5
done
python3 -B tools/pandora-tower-discovery/observe.py "$dir/journey/$label.wram"
