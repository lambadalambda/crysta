"""Explicit local native/phase mutation controls; no optional silent skip."""
import copy
import importlib.util
from pathlib import Path
import sys

spec=importlib.util.spec_from_file_location('pandora_check',Path(__file__).with_name('check.py'))
check=importlib.util.module_from_spec(spec)
spec.loader.exec_module(check)


def main():
    check.require(len(sys.argv)==4,'test_local.py ROM local/EXPORT CAPTURE')
    rom=Path(sys.argv[1]).read_bytes(); export=Path(sys.argv[2]); capture=Path(sys.argv[3])
    meta=check.exported(rom,export)
    check.phase_witnesses(rom,capture,export,meta)
    for mutation in ('missing-actor','wrong-position','wrong-selector','wrong-tie','wrong-mirror','wrong-tour-priority'):
        changed=copy.deepcopy(meta)
        phase=next(p for p in changed['phases'] if p['id']==('tour-41-0' if mutation=='wrong-tour-priority' else 'c-direct'))
        actors=phase['actors']
        if mutation=='missing-actor':actors.pop()
        elif mutation=='wrong-position':actors[0]['position'][0]+=1
        elif mutation=='wrong-selector':actors[0]['selector']=0
        elif mutation=='wrong-tie':actors[0]['tie_rank'],actors[3]['tie_rank']=actors[3]['tie_rank'],actors[0]['tie_rank']
        elif mutation=='wrong-mirror':actors[0]['hflip']=not actors[0]['hflip']
        else:actors[0]['priority_override']=2
        try:check.phase_witnesses(rom,capture,export,changed)
        except ValueError:continue
        raise ValueError('native phase mutation survived: '+mutation)
    print('Six native phase membership/position/selector/tie/mirror/priority mutations rejected.')

if __name__=='__main__':main()
