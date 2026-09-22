#!/usr/bin/env python3
"""Emit an input-only Door5 or Map41 motion window after qualified progression."""
import argparse
import json
from pathlib import Path

TOOLS = Path(__file__).resolve().parent.parent


def route(name):
    if name not in ('Door5', 'Map41'):
        raise ValueError('window must be Door5 or Map41')
    stop = 'door-real-hit1-closed' if name == 'Door5' else 'pandora-neutral-stable'
    commands = []
    for line in (TOOLS / 'pandora-qualification/route.jsonl').read_text().splitlines():
        command = json.loads(line)
        commands.append(command)
        if command.get('label') == stop:
            break
    else:
        raise ValueError(f'progression route has no {stop} checkpoint')
    if name == 'Door5':
        moves = [('Up', 30), ('Right', 15), ('Up', 20),
                 ('Left', 30), ('Up', 20), ('Down', 20)]
        for index, (button, frames) in enumerate(moves):
            commands += [
                {'label': f'door5-contact-{index}', 'frames': frames,
                 'buttons': [button], 'motion': True},
                {'label': f'door5-contact-{index}-settle', 'frames': 12,
                 'buttons': [], 'motion': True},
            ]
    else:
        for line in (TOOLS / 'pandora-tower-discovery/discovery-route.jsonl').read_text().splitlines():
            command = json.loads(line)
            commands.append({**command, 'motion': True})
            if command.get('label') == 'disc-r3':
                break
        else:
            raise ValueError('discovery route has no disc-r3 boundary')
    return commands + [{'finish': True}]


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('window', choices=('Door5', 'Map41'))
    for command in route(parser.parse_args().window):
        print(json.dumps(command))
