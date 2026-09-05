"""Require source-backed semantic startup to agree with authenticated live evidence."""
from hashlib import sha256
import json
from pathlib import Path
import sys
from startup import semantic_new_game
from verify import require, verify


def check(rom_path, boot_directory, trace_root):
    semantic = semantic_new_game(Path(rom_path).read_bytes())
    verify(boot_directory)
    report = json.loads((Path(boot_directory) / 'checkpoints.json').read_text())
    control = next(c for c in report['checkpoints'] if c['frame'] == 6800)
    require(semantic['map'] == control['map'], 'source-derived map')
    require(semantic['position'] == control['player'], 'source-derived spawn')
    require(semantic['events_nonzero'] == control['event_flags_nonzero'], 'source-derived events')
    require(semantic['event_flags_sha256'] == control['event_flags_sha256'], 'source-derived event hash')
    reference = json.loads(Path(__file__).with_name('native-reference.json').read_text())
    for mode, expected in reference.items():
        raw = (Path(trace_root) / mode / 'native-stops.json').read_bytes()
        require(sha256(raw).hexdigest() == expected['report_sha256'], f'{mode}: native evidence hash')
        stops = json.loads(raw)
        require(len(stops) == expected['stops'], f'{mode}: stop count')
        for stop in stops:
            require(stop['completed_frames'] == expected['completed_frames'], f'{mode}: frame boundary')
            memory = (Path(trace_root) / mode / f"stop-{stop['pc']:06x}.wram").read_bytes()
            require(sha256(memory).hexdigest() == stop['wram_sha256'], f'{mode}: raw stopped WRAM')
    print('CPU-free room-only NewGame matches live spawn/events; all 22 native stops authenticated')


if __name__ == '__main__':
    if len(sys.argv) != 4:
        raise SystemExit('usage: semantic_check.py OWNED_JP_ROM BOOT_DIRECTORY TRACE_ROOT')
    check(*sys.argv[1:])
