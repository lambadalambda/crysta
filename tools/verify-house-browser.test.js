'use strict';
// Bounded synthetic verifier tests. No browser, host, ROM or production oracle.
const {test} = require('node:test');
const assert = require('node:assert/strict');
const {compileRoute, validateArt, expectedScene, compareCanvas, createControls, createTickDriver, requireCoverage, compositionInputs, expandMasks} = require('./verify-house-browser.js');

function fixture() {
  // Independently transcribed docs/house-scene.md setup roster, not verifier output.
  const rows = [
    [0x838b96,11,120,112,6,0,false,208,0],
    [0x838c0a,12,88,416,4,1,false,208,3],
    [0x838c14,12,56,384,5,3,false,208,2],
    [0x838c1e,12,72,368,3,0,false,192,1],
    [0x838c28,12,104,368,3,0,false,192,0],
    [0x838cb4,13,72,672,3,0,false,192,0],
    [0x838d7c,16,424,416,2,3,false,208,1],
    [0x838d86,16,440,416,2,2,true,192,0],
    [0x838de2,17,440,640,4,1,true,192,0],
    [0x88d618,15,472,144,66,3,false,160,0],
  ];
  const actors = rows.map(([source_id,map_id,x,y,selector,facing,hflip,palette_base,tie_rank]) => ({
    id:`house:${source_id.toString(16)}`, key:`house:${source_id.toString(16)}`,
    source_id,map_id,position:[x,y],selector,facing,hflip,palette_base,tie_rank,
    policy:'frozen-fresh-setup',setup_record:0,graphics_packet:0xd00000,source_ranges:[[0,10]],
    pose:source_id===0x88d618 ? {kind:'direct',address:0xa2f0ac} : {kind:'compressed',packet:0xd80000,decoded_offset:16},
  }));
  const frame = {width:1,height:1,offset:[0,0],rgba:[12,34,56,255]};
  const keys = ['0:0','1:0','2:0','2:1',
    ...Array.from({length:18},(_,i)=>`${i+3}:0`), ...Array.from({length:6},(_,i)=>`${i+15}:1`)];
  return {schema_version:1,actors,
    frames:Object.fromEntries([...keys,...actors.map(a=>a.key)].map(k=>[k,structuredClone(frame)])),
    scene_ids:{10:['ark'],11:['ark','house:838b96'],12:['ark','house:838c0a','house:838c14','house:838c1e','house:838c28'],
      13:['ark','house:838cb4'],15:['ark','house:88d618'],16:['ark','house:838d7c','house:838d86'],17:['ark','house:838de2']},
    foreground:Object.fromEntries([10,11,12,13,15,16,17].map(id=>[id,{width:id===10?1024:512,height:id===10?1280:1024,runs:[]}])),
    backgrounds:{house:{url:'/map.bmp',width:512,height:1024},exterior:{url:'/exterior.bmp',width:1024,height:1280}},
    background_keys:{10:'exterior',11:'house',12:'house',13:'house',15:'house',16:'house',17:'house'},door_background:'house',
    door_patches:[304,320].map(y=>({position:[128,y],rgba:Array(1024).fill(255),high:Array(256).fill(false)})),
  };
}

test('default511 retains all checkpoints; opt-in JSONL ends at2244', () => {
  const route=compileRoute();
  assert.equal(route.inputs.length,511);
  assert.deepEqual([...route.expected.keys()],[100,167,202,266,301,361,411,461,511]);
  assert.deepEqual(route.expected.get(511),[15,392,191,'walking',false]);
  const lines=require('node:fs').readFileSync(`${__dirname}/house-navigation-qualification/core-route.jsonl`,'utf8').trim().split('\n').map(JSON.parse);
  const full=compileRoute(lines);
  assert.equal(full.inputs.length,2244);
  assert.equal(full.inputs.filter(b=>b===5).length,1);
  assert.deepEqual(full.expected.get(2244),[16,360,463,'walking',true]);
  assert.equal(full.full,true);
  assert.deepEqual(compileRoute([{button:5,steps:2,expect:[12,136,352,'Walking',true]}]).inputs,[5,5]);
  for(const input of [[], '[]', [{button:6,steps:1}], [{button:1,steps:0}], [{button:0,steps:1,expect:[12,0,0,'Walking','false']}], [{finish:true},{button:0,steps:1}]]) {
    assert.throws(()=>compileRoute(input),/route/i);
  }
});

test('complete independent roster required even when running default511', () => {
  validateArt(fixture());
  const mutations = [
    a=>a.actors.pop(), a=>a.actors.push(a.actors[0]),
    a=>a.actors[0].position[0]++, a=>a.actors[1].tie_rank=0,
    a=>a.actors[5].facing=1, a=>a.actors[9].pose.kind='compressed',
    a=>a.actors[0].setup_record=1, a=>a.actors[0].source_ranges=[],
    a=>a.actors[0].id='house:838B96', a=>a.actors[0].key='npc:house',
    a=>a.scene_ids[12].pop(), a=>a.scene_ids[20]=['ark'],
    a=>a.scene_ids[12][1]='house:838b96', a=>a.scene_ids[11].push('ark'),
    a=>delete a.frames['house:838de2'], a=>a.frames.extra=a.frames['0:0'],
    a=>a.frames['0:0'].rgba[3]=128, a=>delete a.foreground[11],
    a=>a.foreground[11].runs=[524287,2],
    a=>a.door_patches[0].position=[128,305], a=>a.door_patches[1].high[0]=1,
    a=>delete a.scene_ids[10], a=>a.scene_ids[10].push('house:838b96'),
    a=>a.foreground[10].height=1024, a=>a.foreground[10].runs=[1024*1280-1,2],
    a=>a.backgrounds.exterior.url='/map.bmp', a=>a.background_keys[10]='house',
    a=>a.background_keys[11]='exterior', a=>a.door_background='exterior',
  ];
  for(const mutate of mutations) { const art=fixture(); mutate(art); assert.throws(()=>validateArt(art)); }
});

test('world Y then fixed native ties, Ark last at equal Y', () => {
  const scene=expectedScene(12,[88,368],'0:0');
  assert.deepEqual(scene.map(a=>a.id),['house:838c28','house:838c1e','ark','house:838c14','house:838c0a']);
  assert.deepEqual(scene[2],{id:'ark',key:'0:0',position:[88,368]});
  assert.deepEqual(expectedScene(16,[424,416],'2:1').map(a=>a.id),['house:838d86','house:838d7c','ark']);
});

const rgba = (...reds) => reds.flatMap(r=>[r,0,0,255]);
const sprite = (id,red,x=0) => ({id,position:[x,0],frame:{width:2,height:1,offset:[0,0],rgba:rgba(red,red)}});
test('full pixel oracle: overlap, transparency, offsets, high mask and door replacement', () => {
  const options={width:2,height:1,sheetWidth:2,camera:[0,0],background:rgba(1,2),mask:[0,1],doorPatches:[],doorOpen:false,
    sprites:[sprite('resident',10),sprite('ark',20,1)]};
  let counts=compareCanvas({...options,actual:rgba(10,2)});
  assert.deepEqual(counts.resident,{viewportPixels:2,highOccludedPixels:1,actorOccludedPixels:0,pixels:1});
  assert.equal(counts.ark.pixels,0);
  assert.throws(()=>compareCanvas({...options,actual:rgba(10,20)}),/pixel 1,0/);
  // A full metatile patch REPLACES high as well as RGBA, including clearing old high bits.
  const patch={position:[0,0],rgba:rgba(...Array(256).fill(7)),high:Array(256).fill(false)};
  patch.high[0]=true;
  counts=compareCanvas({...options,doorPatches:[patch],doorOpen:true,actual:rgba(7,20)});
  assert.equal(counts.ark.pixels,1);
  assert.equal(counts.resident.actorOccludedPixels,1);
  const transparent=sprite('ark',20); transparent.frame.rgba[3]=0;
  compareCanvas({...options,mask:[0,0],sprites:[sprite('resident',10),transparent],actual:rgba(10,20)});
  const offset=sprite('ark',30); offset.frame.offset=[-1,0];
  compareCanvas({...options,mask:[0,0],sprites:[offset],actual:rgba(30,2)});
});

test('controls discard transition input, reassert on walking, and resume only after Interact ack', () => {
  const events=[];
  const nodes={pause:{disabled:false,textContent:'Resume',click(){events.push(this.textContent);this.textContent=this.textContent==='Resume'?'Pause':'Resume';}},
    interact:{disabled:false,click(){events.push('Interact');nodes.pause.textContent='Resume';}}};
  const controls=createControls(id=>nodes[id],(type,key)=>events.push(`${type}:${key}`));
  const state=(map,phase)=>({map,phase});
  controls.resume();
  controls.apply(3,state(12,'walking'));
  controls.apply(3,state(12,'departing'));
  controls.apply(3,state(11,'arriving'));
  controls.apply(3,state(11,'walking'));
  controls.apply(5,state(11,'walking'));
  assert.deepEqual(events,['Resume','keydown:ArrowUp','keyup:ArrowUp','keydown:ArrowUp','keyup:ArrowUp','Interact']);
  controls.resume(); // only called by route observer after the interaction's sequential tick
  controls.apply(3,state(11,'walking'));
  controls.stop();
  assert.deepEqual(events.slice(-4),['Resume','keydown:ArrowUp','keyup:ArrowUp','Pause']);
});

test('full coverage is nonvacuous, with only measured all-high table occlusion exempt', () => {
  const evidence=Object.fromEntries(fixture().actors.map(a=>[a.id,{pixels:1,viewportPixels:1,highOccludedPixels:0}]));
  evidence.ark={pixels:0};
  assert.throws(()=>requireCoverage(evidence),/Ark/);
  evidence.ark.pixels=1;
  requireCoverage(evidence);
  evidence['house:838b96'].pixels=0;
  assert.throws(()=>requireCoverage(evidence),/838b96/);
  evidence['house:838b96'].pixels=1;
  evidence['house:88d618']={pixels:0,viewportPixels:8,highOccludedPixels:8};
  assert.match(requireCoverage(evidence)[0].reason,/high/i);
  evidence['house:88d618'].viewportPixels=0;
  assert.throws(()=>requireCoverage(evidence),/88d618/);
});

test('tick driver waits for sequential command ack before Resume and next input', () => {
  const events=[], view={map:12,phase:'walking'};
  const controls={resume:()=>events.push('Resume'),apply:b=>events.push(`input:${b}`)};
  const drive=createTickDriver({route:{inputs:[0,5,5,3]},controls,
    checkTick:tick=>{events.push(`check:${tick}`);return view;},finish:()=>events.push('finish')});
  drive(0); assert.deepEqual(events,[]); // Busy/other repeated render is not an ack.
  drive(1); assert.deepEqual(events,['check:1','input:5']);
  drive(1); assert.equal(events.length,2); // Pending Interact remains paused.
  drive(2); assert.deepEqual(events.slice(-3),['check:2','Resume','input:5']);
  drive(3); assert.deepEqual(events.slice(-3),['check:3','Resume','input:3']);
  drive(4); assert.deepEqual(events.slice(-2),['check:4','finish']);
  const skipped=createTickDriver({route:{inputs:[5,3]},controls,
    checkTick:()=>assert.fail('must not check/drive a skipped tick'),finish:()=>assert.fail('must not finish')});
  const count=events.length;
  assert.throws(()=>skipped(2),/Non-sequential UI tick 0 -> 2/);
  assert.equal(events.length,count);
});

test('A is Ark-only with its own full-size mask/sheet; open indoor door never patches outdoors', () => {
  const art=fixture(); art.foreground[10].runs=[1024*1280-1,1];
  validateArt(art);
  assert.deepEqual(expectedScene(10,[538,815],'0:0'),[{id:'ark',key:'0:0',position:[538,815]}]);
  const masks=expandMasks(art);
  assert.equal(masks[10].length,1024*1280);
  assert.equal(masks[10].at(-1),1);
  const backgrounds={house:{width:512,height:1024,rgba:[1]},exterior:{width:1024,height:1280,rgba:[2]}};
  const outdoor=compositionInputs(art,backgrounds,masks,{map:10,door:true,camera:[410,703]});
  assert.equal(outdoor.doorOpen,false); assert.equal(outdoor.sheetWidth,1024);
  assert.equal(outdoor.background,backgrounds.exterior.rgba);
  assert.equal(outdoor.mask,masks[10]);
  assert.equal(compositionInputs(art,backgrounds,masks,{map:12,door:true,camera:[0,0]}).doorOpen,true);
  assert.throws(()=>compositionInputs(art,backgrounds,masks,{map:12,door:true,camera:[410,703]}),/camera/);
});
