// Narrow typed overlay admission: synthetic fixtures are independent of ROM export.
'use strict';
const assert=require('node:assert/strict');
const vm=require('node:vm');
const {fixture:baseFixture,html}=require('./pandora-render-check.js');
function fixture() {
  const f=baseFixture(), b=f.bundle;
  const poses={}, flight={};
  for(const [motion,art,ark,pot] of [
    ['lifting','80a261',[24,25,26,26],[43,44,45,45]],
    ['standing','80a24f',[3,4,5,5],[25,26,26,26]],
    ['walking','80a255',[9,10,11,11],[29,29,30,30]],
    ['throwing','80a261',[15,16,17,17],[46,47,48,48]]]) {
    for(let d=0;d<4;d++) {
      const key=`${motion}:${d}`, mirror=Number(d===2);
      poses[key]={ark:`pandora:${art}:${ark[d]}:0:${mirror}`,pots:{}};
      for(const [slot,id] of [[2442,'96e1a6'],[2447,'96e1ab']]) poses[key].pots[slot]=`pandora:${id}:${pot[d]}:0:${mirror}`;
    }
  }
  for(const [slot,id] of [[2442,'96e1a6'],[2447,'96e1ab']]) flight[slot]=`pandora:${id}:60:0:0`;
  b.pandora_carry={map_id:12,policy:'core-semantic-record0-not-native-timing',priority:2,tie_policy:'world-y-pot-after-ark',poses,flight};
  for(const p of Object.values(poses)) for(const key of [p.ark,...Object.values(p.pots)]) {
    const mirror=key.endsWith(':1');
    b.frames[key]={width:2,height:1,offset:[mirror?-3:-1,-20],rgba:[20,40,60,255,70,80,90,255]};
  }
  for(const key of Object.values(flight)) b.frames[key]={width:2,height:1,offset:[-1,-10],rgba:[200,100,50,255,0,0,0,0]};
  const actor={...b.pandora_scenes['tour-start'].actors[0],position:[184,368],priority:2};
  b.pandora_scenes['c-direct']={map_id:12,ark_tie_rank:1,policy:'source-endpoints-not-script-timing',limits:[],actors:[actor]};
  f.state={map_id:12,scene_phase:'c-direct',actor_key:'ark',x:184,y:368,camera:[80,240],scene:[
    {id:actor.id,key:actor.key,position:actor.position,priority:2}, {id:'ark',key:'ark',position:[184,368],priority:2}]};
  return f;
}
function stateFor(bundle,state,pose,slot=2442,tick=0,x=184) {
  const flight=pose==='throwing:1'&&tick>=18&&tick<(x===184?22:27);
  const hand=!pose.startsWith('throwing:')||tick<18;
  const reserved=hand||flight;
  const key=bundle.pandora_carry.poses[pose].ark;
  return {...state,x,actor_key:key,carry:{pose,phase_tick:tick,held_slot:hand?slot:null,reserved_slot:reserved?slot:null,flight:flight?[x,357-3*(tick-18)]:null},
    scene:state.scene.map(e=>e.id==='ark'?{...e,key,position:[x,state.y]}:e)};
}
function check(api) {
  const {bundle,pixels,state}=fixture(), make=(width,height,data)=>({width,height,data});
  const art=api.prepareArt(bundle,pixels,make);
  let cases=0;
  for(const pose of Object.keys(bundle.pandora_carry.poses)) for(const slot of [2442,2447]) {
    const s=stateFor(bundle,state,pose,slot), actors=api.selectActors(art,s);
    assert.equal(actors.length,3);
    assert.deepEqual(Array.from(actors[2].position),[s.x,s.y], 'no extra carry elevation');
    assert.equal(actors[2].image,art.actors[bundle.pandora_carry.poses[pose].pots[slot]].image);
    assert.equal(actors[1].image,art.actors[s.actor_key].image);
    assert(actors.every(a=>a.priority===2&&!a.flipX));
    cases++;
  }
  for(const x of [136,184]) for(const slot of [2442,2447]) for(let tick=0;tick<32;tick++) {
    const s=stateFor(bundle,state,'throwing:1',slot,tick,x), actors=api.selectActors(art,s);
    assert.equal(actors.length,s.carry.reserved_slot===null?2:3);
    if(s.carry.flight) {
      assert.equal(actors[0].image,art.actors[bundle.pandora_carry.flight[slot]].image);
      assert.deepEqual(Array.from(actors[0].position),s.carry.flight);
    }
    cases++;
  }
  const good=stateFor(bundle,state,'throwing:1',2447,18);
  const reject=s=>assert.throws(()=>api.selectActors(art,s),/carry|scene|phase|sprite/i);
  for(const change of [{pose:'standing:1'},{pose:'throwing:2'},{phase_tick:17},{phase_tick:19},{phase_tick:32},{phase_tick:18.1},{held_slot:2447},{reserved_slot:null},{reserved_slot:2443},{flight:null},{flight:[184,356]},{flight:[183,357]},{flight:[184,357,0]},{priority:3},{position:[184,357]},{key:'ark'},{type:'npc'}]) reject({...good,carry:{...good.carry,...change}});
  for(const change of [{x:183},{y:367},{actor_key:'ark'},{carry:null},{carry:undefined},{carry:[]},{carry:{}},{map_id:14},{scene_phase:undefined}]) reject({...good,...change});
  for(const change of [
    {scene:[...good.scene].reverse()}, {scene:good.scene.slice(1)}, {scene:[...good.scene,{id:'pot',key:'ark',position:[184,357],priority:2}]},
    {scene:good.scene.map(e=>e.id==='ark'?e:{...e,position:[184,369]})},
    {scene:good.scene.map(e=>({...e,priority:3}))}, {scene:good.scene.map(e=>e.id==='ark'?e:{...e,key:'ark'})}
  ]) reject({...good,...change});
  for(const pose of ['lifting:0','standing:0','walking:0','throwing:0']) {
    const s=stateFor(bundle,state,pose);
    for(const change of [{held_slot:null},{held_slot:2447},{flight:[184,357]},{phase_tick:pose==='lifting:0'?23:pose==='throwing:0'?18:1}]) reject({...s,carry:{...s.carry,...change}});
  }
  const broken=stateFor(bundle,state,'throwing:1',2442,22);
  reject({...broken,carry:{...broken.carry,reserved_slot:2442}});
  reject({...broken,carry:{...broken.carry,flight:[184,345]}});
  for(const mutate of [
    c=>c.priority=3,c=>c.map_id=14,c=>c.policy='native-timing',c=>c.tie_policy='priority',c=>c.extra=1,
    c=>delete c.poses['lifting:0'],c=>c.poses.evil=c.poses['lifting:0'],c=>c.poses['lifting:0'].ark='ark',
    c=>c.poses['lifting:0'].pots[2442]=c.poses['lifting:0'].pots[2447],
    c=>c.poses['lifting:2'].pots[2442]='pandora:96e1a6:45:0:0',
    c=>c.poses['lifting:0'].pots[2443]=c.poses['lifting:0'].pots[2442],
    c=>c.poses['lifting:0'].priority=3,c=>c.flight[2442]='pandora:96e1a6:60:0:1',c=>delete c.flight[2447]
  ]) {
    const b=structuredClone(bundle);mutate(b.pandora_carry);
    assert.throws(()=>api.prepareArt(b,pixels,make),/carry|art/i);
  }
  const missing=structuredClone(bundle);delete missing.frames[bundle.pandora_carry.flight[2442]];
  assert.throws(()=>api.prepareArt(missing,pixels,make),/carry|art/i);
  const old=api.prepareArt({...bundle,pandora_carry:undefined},pixels,make);
  assert.throws(()=>api.selectActors(old,good),/carry/i);
  assert.equal(api.selectActors(old,state).length,2);
  reject({...state,actor_key:good.actor_key,scene:good.scene}); // no bypass without typed overlay
  // Prepared carry catalog must not alias mutable input JSON.
  const held=stateFor(bundle,state,'throwing:1',2447);
  bundle.pandora_carry.poses['throwing:1'].pots[2447]='ark';
  assert.equal(api.selectActors(art,held)[2].image,art.actors['pandora:96e1ab:47:0:0'].image);
  bundle.pandora_carry.flight[2447]='ark';
  assert.equal(api.selectActors(art,good)[0].image,art.actors['pandora:96e1ab:60:0:0'].image);
  return {cases,negativeControls:'slots, lifetime, lane, pair, mirror, roster, priority, catalog'};
}
module.exports={fixture,stateFor,check};
if(require.main===module) {
  const sandbox={};vm.runInNewContext(html.match(/<script>([\s\S]*?)<\/script>/)[1],sandbox);
  console.log(check(sandbox.RoomSlice));
}
