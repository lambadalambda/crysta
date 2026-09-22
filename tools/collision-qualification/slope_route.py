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


def type8_route():
    """Approach and leave the town's type8 fence in a continuous window."""
    commands = tree_route('east')[:-1]
    for index, (button, frames) in enumerate([
            ('Left', 170), ('Up', 48), ('Down', 96),
            ('Left', 64), ('Up', 96), ('Right', 100)]):
        commands += [
            {'label': f'type8-{index}', 'frames': frames,
             'buttons': [button], 'motion': True},
            {'label': f'type8-{index}-settle', 'frames': 12,
             'buttons': [], 'motion': True},
        ]
    return commands + [{'finish': True}]


def type8_gap_route():
    """Reach the cap from above; stop before the later nonordinary descent."""
    commands = type8_route()[:-1]
    for name, moves in [
            ('type8-cross', [('Left', 190), ('Up', 64), ('Right', 130),
                             ('Down', 40), ('Left', 100)]),
            ('type8-gap', [('Up', 90), ('Right', 78), ('Up', 64), ('Right', 88),
                           ('Down', 44), ('Left', 30), ('Right', 60)])]:
        for index, (button, frames) in enumerate(moves):
            commands += [
                {'label': f'{name}-{index}', 'frames': frames,
                 'buttons': [button], 'motion': True},
                {'label': f'{name}-{index}-settle', 'frames': 12,
                 'buttons': [], 'motion': True},
            ]
    return commands + [{'finish': True}]


def type8_horizontal_route():
    """Keep the cap approach, then sample Open/8 from both horizontal sides."""
    commands = []
    for command in type8_gap_route():
        commands.append({k: v for k, v in command.items() if k != 'motion'})
        if command.get('label') == 'type8-gap-4-settle':
            break
    else:
        raise ValueError('type8 route has no settled cap checkpoint')
    # Four Down frames reposition within ordinary mode, not the later descent.
    for index, (buttons, frames) in enumerate([
            (['Right'], 30), ([], 12), (['Down'], 4),
            ([], 12), (['Left'], 30), ([], 12)]):
        commands.append({'label': f'horizontal8-{index}', 'buttons': buttons,
                         'frames': frames, 'motion': True})
    return commands + [{'finish': True}]


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('direction', choices=('Right', 'Left', 'TreeEast', 'TreeBottom', 'Type8', 'Type8Gap', 'Type8Horizontal'))
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument('--trace', action='store_true')
    mode.add_argument('--motion', action='store_true')
    parser.add_argument('--vertical', action='store_true')
    args = parser.parse_args()
    if args.vertical and not args.motion:
        parser.error('--vertical requires --motion')
    named = {'TreeEast': lambda: tree_route('east'),
             'TreeBottom': lambda: tree_route('bottom'),
             'Type8': type8_route, 'Type8Gap': type8_gap_route,
             'Type8Horizontal': type8_horizontal_route}
    if args.direction in named:
        if args.trace or args.motion or args.vertical:
            parser.error('named routes always record motion; no extra flags')
        commands = named[args.direction]()
    else:
        commands = route(args.direction, args.trace, motion=args.motion, vertical=args.vertical)
    for command in commands:
        print(json.dumps(command))
