#!/usr/bin/env python3
"""Emit an input-only discovery route onto Crysta's type6/7 slope boundary.

Reuses the qualified house conversation prefix through exterior-walk-down.
Unlike the earlier exact-position goto experiment, this uses bounded held
inputs without oscillation/dash retries. No ROM or captured state is needed.
Usage: slope_route.py Right|Left [--trace | --motion [--vertical]]
"""
import argparse
import json
from pathlib import Path


def route(direction, trace=False, *, motion=False, vertical=False):
    if direction not in ('Right', 'Left'):
        raise ValueError('direction must be Right or Left')
    if (trace and motion) or (vertical and not motion):
        raise ValueError('vertical requires motion; trace and motion are exclusive')
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
    if motion:
        suffix.pop()
        name = f'motion-{direction}' + ('-vertical' if vertical else '')
        moves = ([(direction, 6), ('Down', 24), ('Up', 40),
                  (direction, 14), ('Down', 32), ('Up', 48)]
                 if vertical else [(direction, 40)])
        for index, (button, frames) in enumerate(moves):
            # Preserve the first Right capture's historical label.
            step = label if direction == 'Right' and not vertical else f'{name}-{index}'
            suffix.append({'label': step, 'frames': frames,
                           'buttons': [button], 'motion': True})
            if vertical:
                suffix.append({'label': f'{step}-settle', 'frames': 12,
                               'buttons': [], 'motion': True})
    return prefix + suffix + [{'finish': True}]


def tree_route(side):
    """Reach the tree's lower faces: Up6/7, Left6 and Right7 handlers."""
    if side not in ('east', 'bottom'):
        raise ValueError('tree side must be east or bottom')
    commands = route('Right')[:-2]  # Through the same settled slope approach.
    approach = [('Up', 76), ('Left', 54), ('Up', 280)]
    if side == 'bottom':
        approach += [('Left', 44)]
    moves = ([('Left', 28), ('Up', 40), ('Left', 16), ('Up', 40),
              ('Left', 32), ('Right', 40)] if side == 'east' else
             [('Left', 54), ('Up', 22), ('Right', 40), ('Up', 40)])
    for name, legs in [('motion-tree-west', approach), (f'motion-tree-{side}', moves)]:
        for index, (button, frames) in enumerate(legs):
            commands += [
                {'label': f'{name}-{2 * index}', 'frames': frames,
                 'buttons': [button], 'motion': True},
                {'label': f'{name}-{2 * index + 1}', 'frames': 12,
                 'buttons': [], 'motion': True},
            ]
    return commands + [{'finish': True}]


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('direction', choices=('Right', 'Left', 'TreeEast', 'TreeBottom'))
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument('--trace', action='store_true')
    mode.add_argument('--motion', action='store_true')
    parser.add_argument('--vertical', action='store_true')
    args = parser.parse_args()
    if args.vertical and not args.motion:
        parser.error('--vertical requires --motion')
    if args.direction.startswith('Tree'):
        if args.trace or args.motion or args.vertical:
            parser.error('tree routes always record motion; no extra flags')
        commands = tree_route(args.direction[4:].lower())
    else:
        commands = route(args.direction, args.trace, motion=args.motion, vertical=args.vertical)
    for command in commands:
        print(json.dumps(command))
