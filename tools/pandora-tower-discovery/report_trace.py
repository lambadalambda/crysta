#!/usr/bin/env python3
"""Print one trace record from the probe's JSONL. Reads stdin or a file."""

from __future__ import annotations

import json
import sys


def render(record: dict) -> str:
    reg = record["registers"]
    state = record["state"]
    stack = " ".join(f"{b:02X}" for b in record["stack_bytes"])
    path = " <- ".join("$" + address for address in record["path"][:12])
    flags = [hex(f) for f in state["flags"]]
    return "\n".join(
        [
            f"target ${record['target']}  stop={record['stop']}  "
            f"instructions={record['instructions']}  frames={record['elapsed_frames']}",
            f"  pc=${reg['address']} a={reg['a']:#06x} x={reg['x']:#06x} "
            f"y={reg['y']:#06x} s=${reg['stack']} dp=${reg['direct_page']} "
            f"p={reg['status']:#04x}",
            f"  stack: {stack}",
            f"  path:  {path}",
            f"  state: map={state['map']} pos={state['position']} flags={flags}",
        ]
    )


def last_trace(stream) -> dict | None:
    """The final trace record in a probe log, or None if it holds no trace."""
    last = None
    for line in stream:
        if not line.strip():
            continue
        record = json.loads(line)
        if record.get("kind") == "trace":
            last = record
    return last


def report(stream) -> int:
    record = last_trace(stream)
    if record is None:
        print("no trace record found", file=sys.stderr)
        return 1
    print(render(record))
    return 0


def main() -> int:
    source = open(sys.argv[1]) if len(sys.argv) > 1 else sys.stdin
    with source:
        return report(source)


if __name__ == "__main__":
    sys.exit(main())
