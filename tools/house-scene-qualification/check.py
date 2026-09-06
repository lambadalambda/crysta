"""Recompute both fresh census runs; committed output is metadata/hashes only."""
from pathlib import Path
import json
import sys
from census import ROM_SHA, checkpoint, require, sha, source_graph, u
from roster import source_roster

LABELS=('boot','room10','settledC','north-door-settled','settledD','settled11','wait11','front-settled','cellar-blocked','cellar-final')
RESIDENTS={
    11:{0x1040:0x838b96},
    12:{0x1040:0x838c0a,0x1080:0x838c14,0x10c0:0x838c1e,0x1100:0x838c28},
    13:{0x1040:0x838cb4},
    15:{},
    16:{0x1040:0x838d7c,0x1080:0x838d86},
    17:{0x1040:0x838de2},
}
EXPECTED_MAPS=(15,16,12,11,13,17,17,13,12,12)

def canonical(value):return json.dumps(value,sort_keys=True,separators=(',',':')).encode()

def hardware_census(census):
    # Output pixels are a non-atomic diagnostic surface, not this census contract.
    return {**census,'hashes':{k:v for k,v in census['hashes'].items() if k!='pixels'}}

def inspect(rom,root,roster):
    source={r['source']:r for m in roster for r in m['records']}
    reports=[]
    for label,map_id in zip(LABELS,EXPECTED_MAPS):
        c=hardware_census(checkpoint(rom,root,label)); require(c['map']==map_id,'itinerary map mismatch')
        require(c['event_bits']==([1,32,251] if label=='cellar-final' else [32,251]),'changed fresh event branch')
        expected=RESIDENTS[map_id]; entities={e['slot']:e for e in c['entities']}
        owners={o['entity']:o for o in c['oam_owners']}
        require(set(expected)<=owners.keys(),'resident missing from hardware OAM')
        residents=[]
        for slot,identity in expected.items():
            e=entities[slot];s=source[identity]
            if label!='front-settled':require(e['position']==s['position'],'source resident spawn position')
            require(owners[slot]['priorities']==[2],'nonordinary resident priority')
            residents.append({'source':identity,'slot':slot,'position':e['position'],'facing':e['facing'],'native_composition':e['composition'],'selector':e['selector'],'palettes':owners[slot]['palettes'],'linked_ordinal':list(entities).index(slot)})
        others=set(owners)-set(expected)-{0x1000}
        # One linked shadow is present; F also has the script-created table prop.
        shadows=[slot for slot in others if entities[slot]['resume']==0x84a917 and entities[slot]['composition']==0xa2c825]
        require(len(shadows)==1,'shadow ownership')
        others-=set(shadows)
        if map_id==15:
            require(len(others)==1,'F prop ownership')
            e=entities[next(iter(others))];parent=source[0x838d4f]
            call=rom[0x8d618:0x8d621]
            require(call[:5]==bytes([2,0x9c,0x41,0xd6,0x88]),'changed child creation')
            dx=int.from_bytes(call[5:7],'little',signed=True);dy=int.from_bytes(call[7:9],'little',signed=True)
            require(e['position']==[parent['position'][0]+dx,parent['position'][1]+dy] and e['resume']==0x88d655 and e['composition']==0xa2f0ac,'source child placement/pose')
        else:require(not others,'unclassified visible entity')
        reports.append({'label':label,'map':map_id,'camera':c['camera'],'event_bits':c['event_bits'],'residents':residents,'linked_entities':len(entities),'auxiliary_oam':c['auxiliary_oam'],'hardware_census_sha256':sha(canonical(c)),'hashes':c['hashes']})
    return reports

def report(rom_path,directory):
    rom=Path(rom_path).read_bytes();require(sha(rom)==ROM_SHA,'Japanese ROM authentication')
    root=Path(directory);roster=source_roster(rom)
    runs=[inspect(rom,root/run,roster) for run in ('a','b')]
    require(runs[0]==runs[1],'independent fresh census differs')
    exceptions=[hardware_census(checkpoint(rom,root/run,'exception-settled')) for run in ('exception-a','exception-b')]
    require(exceptions[0]==exceptions[1],'independent walking diagnostic differs')
    e=exceptions[0]
    require(e['map']==15 and e['entities'][[a['slot'] for a in e['entities']].index(0x1000)]['position']==[441,112] and e['event_bits']==[32,251],'exceptional-exit walking diagnostic')
    ranges=[(0xf3f1,0xf800),(0xeae6,0xed51),(0x58_008,0x58_0d3),(0x58656,0x58748),(0x59095,0x590f9),(0x88e4b,0x88f0c),(0x8a837,0x8a896),(0x8a9af,0x8a9bd),(0x8aae9,0x8ab50),(0x8aca9,0x8acd3),(0x8d60d,0x8d65a),(0x4a129,0x4a1a0),(0xdb841,0xdb920)]
    return {'rom_sha256':ROM_SHA,'graph':source_graph(rom),'source_roster':roster,'captures':runs[0],'exceptional_exit_diagnostic_sha256':sha(canonical(e)),'sources':[{'start':a,'end':b,'sha256':sha(rom[a:b])} for a,b in ranges],'itinerary_sha256':sha(Path('tools/house-scene-qualification/route.jsonl').read_bytes()),'exception_itinerary_sha256':sha(Path('tools/house-scene-qualification/exception-route.jsonl').read_bytes())}

if __name__=='__main__':
    result=report(sys.argv[1],sys.argv[2])
    reference=Path('tools/house-scene-qualification/reference.json')
    if len(sys.argv)==4 and sys.argv[3]=='--record':reference.write_text(json.dumps(result,indent=2)+'\n')
    else:
        require(result==json.loads(reference.read_text()),'committed census reference mismatch')
        print('House census: nine source scenes, six witnessed rooms, nine residents; two fresh runs exact')
