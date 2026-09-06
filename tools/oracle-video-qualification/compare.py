#!/usr/bin/env python3
"""Export exact replay differences, never an alternate golden or pixel allowlist.

Compare capture directories and their sibling .jsonl full logs. Exit 1 for ANY
mismatch (including intentional observer changes), with the report on stdout.
Only names, sizes and hashes leave the private capture directories.
"""
import hashlib
import json
from pathlib import Path
import sys


def sha(data):
    return hashlib.sha256(data).hexdigest()


def inventory(root):
    return {str(p.relative_to(root)): {'bytes': p.stat().st_size, 'sha256': sha(p.read_bytes())}
            for p in sorted(root.rglob('*')) if p.is_file()}


def compare(left, right):
    a, b = inventory(left), inventory(right)
    common = sorted(a.keys() & b.keys())
    changed = {name: {'left': a[name], 'right': b[name]}
               for name in common if (left / name).read_bytes() != (right / name).read_bytes()}
    la, lb = left.with_suffix('.jsonl').read_bytes(), right.with_suffix('.jsonl').read_bytes()
    surfaces = {}
    for ext in sorted({Path(name).suffix for name in a.keys() | b.keys()}):
        aa = {n: v for n, v in a.items() if Path(n).suffix == ext}
        bb = {n: v for n, v in b.items() if Path(n).suffix == ext}
        surfaces[ext] = {'left_count': len(aa), 'right_count': len(bb),
                         'equal_count': sum(n in bb and n not in changed for n in aa),
                         'left_inventory_sha256': sha(json.dumps(aa, sort_keys=True).encode()),
                         'right_inventory_sha256': sha(json.dumps(bb, sort_keys=True).encode())}
    return {'byte_equal': a.keys() == b.keys() and not changed and la == lb,
            'left_count': len(a), 'right_count': len(b),
            'only_left': sorted(a.keys() - b.keys()), 'only_right': sorted(b.keys() - a.keys()),
            'frame_log': {'byte_equal': la == lb, 'left_sha256': sha(la), 'right_sha256': sha(lb)},
            'surfaces': surfaces, 'changed': changed}


if __name__ == '__main__':
    if len(sys.argv) != 3:
        sys.exit('usage: compare.py LEFT_CAPTURE_DIR RIGHT_CAPTURE_DIR')
    result = compare(*(Path(p) for p in sys.argv[1:]))
    print(json.dumps(result, indent=2, sort_keys=True))
    sys.exit(0 if result['byte_equal'] else 1)
