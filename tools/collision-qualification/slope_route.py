#!/usr/bin/env python3
"""Emit an input-only discovery route onto Crysta's type6/7 slope boundary.

Reuses the qualified house conversation prefix through exterior-walk-down.
Unlike the earlier exact-position goto experiment, this uses bounded held
inputs without oscillation/dash retries. No ROM or captured state is needed.
Usage: slope_route.py Right|Left [--trace]
"""
import json
from pathlib import Path
import sys


def route(direction, trace=False):
    if direction not in ('Right', 'Left'):
        raise ValueError('direction must be Right or Left')
    source = Path(__file__).resolve().parent.parent / 'house-conversation-qualification/route.jsonl'
    prefix = []
    for line in source.read_text().splitlines():
        row = json.loads(line)
        prefix.append(row)
        if row.get('label') == 'exterior-walk-down':
            break
    else:
        raise ValueError('house route has no exterior-walk-down witness')
    label = f'slope-{direction.lower()}'
    suffix = [
        {'label': 'slope-approach', 'frames': 76, 'buttons': ['Down']},
        {'label': 'slope-settle', 'frames': 12, 'buttons': []},
        {'label': label, 'frames': 6 if trace else 40, 'buttons': [direction]},
    ]
    if trace:
        suffix += [
            {'trace': label, 'buttons': [direction]},
            {'label': f'{label}-rest', 'frames': 33, 'buttons': [direction]},
        ]
    return prefix + suffix + [{'finish': True}]


if __name__ == '__main__':
    if len(sys.argv) not in (2, 3) or (len(sys.argv) == 3 and sys.argv[2] != '--trace'):
        sys.exit(__doc__)
    for command in route(sys.argv[1], len(sys.argv) == 3):
        print(json.dumps(command))
