#!/bin/sh
# Breakpoint a script address in a running trace session (PROBE=trace).
# usage: trace.sh HEX_TARGET [FRAME_BUDGET]
#
# Reports whether the address was reached, the CPU state when it was, and the
# tail of the execution path into it. A FrameLimit stop means "not reached
# within the budget", which is itself the answer to "is this code gated".
set -eu
dir=${DISCOVERY_DIR:-local/pandora-tower-discovery/trace}
target=${1:?usage: trace.sh HEX_TARGET [FRAME_BUDGET] [INSTRUCTION_BUDGET]}
frames=${2:-600}
instructions=${3:-2000000}
case "$target" in
  *[!0-9A-Fa-f]* | "") echo "target must be hex digits: $target" >&2; exit 1 ;;
esac
[ "${#target}" -le 6 ] || { echo "target above 24 bits: $target" >&2; exit 1; }
case "$frames$instructions" in *[!0-9]*) echo "budgets must be decimal" >&2; exit 1 ;; esac
kind=$(cat "$dir/probe.kind" 2>/dev/null || echo walk)
[ "$kind" = trace ] || {
  echo "session at $dir runs the '$kind' probe; start it with PROBE=trace" >&2
  exit 1
}
[ -f "$dir/probe.pid" ] || { echo "no running session at $dir" >&2; exit 1; }
pid=$(cat "$dir/probe.pid")
kill -0 "$pid" 2>/dev/null || {
  echo "probe $pid is not running; last error output:" >&2
  tail -5 "$dir/err.log" >&2
  exit 1
}
offset=$(( $(wc -c <"$dir/out.jsonl") + 1 ))
printf '{"trace":"%s","frames":%s,"instructions":%s}\n' "$target" "$frames" "$instructions" >>"$dir/in"
i=0
until tail -c "+$offset" "$dir/out.jsonl" | grep -q '"kind":"trace"'; do
  i=$((i + 1))
  if [ "$i" -gt 600 ]; then echo "timeout tracing $target" >&2; exit 1; fi
  kill -0 "$pid" 2>/dev/null || {
    echo "probe $pid died tracing $target:" >&2
    tail -5 "$dir/err.log" >&2
    exit 1
  }
  sleep 0.5
done
tail -c "+$offset" "$dir/out.jsonl" | grep '"kind":"trace"' |
  python3 -B tools/pandora-tower-discovery/report_trace.py
