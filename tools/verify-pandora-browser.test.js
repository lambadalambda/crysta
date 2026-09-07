'use strict';
const {test}=require('node:test');
const assert=require('node:assert/strict');
const {projectRoute,checkState,createCommands,bounded,createInspection,checkLocalBootstrap,compareComposition,requireCoverage,indexedPage,checkSemanticCheckpoint,blockingDialogue,checkDialogueControls}=require('./verify-pandora-browser.js');
const clone=structuredClone;
function fixture() {
  const state=(tick,dialogue=null,x=304)=>({tick,dialogue,x,y:112,map_id:15,owner:dialogue?'dialogue':'player',events:[32,251]});
  const record=(tick,dialogue,continuation,x)=>({state:state(tick,dialogue,x),continuation:continuation.repeat(64)});
  const page={key:'text:888ff0:0'};
  const offline=[record(0,null,'a'),record(1,page,'b'),record(2,page,'b'),record(3,page,'b'),record(4,null,'c'),record(5,null,'d',306)];
  const projected=[offline[0],offline[1],{...offline[4],state:{...offline[4].state,tick:2}},{...offline[5],state:{...offline[5].state,tick:3}}];
  return {route:{actions:[[5,1],[2,1],[0,1],[6,1],[2,1]]},proof:{schema:1,offline,projected}};
}
test('browser-local inspection is read-only and never falls back to fetch',async()=>{
  const calls=[],runtime={
    request(...args){calls.push(['state',...args]);return Promise.resolve({tick:0});},
    loadArt(){calls.push(['art']);return new TextEncoder().encode('{"frames":{}}');},
    loadBackground(path){calls.push(['background',path]);return Promise.resolve(new Blob([path]));},
  };
  let fetches=0;
  const inspection=createInspection({runtime,fetchFn:async()=>{fetches++;throw new Error('fetch forbidden');}});
  assert.equal(inspection.kind,'browser-local-wasm-worker');
  assert.deepEqual(await inspection.state(),{tick:0});
  assert.deepEqual(await inspection.art(),{frames:{}});
  assert.equal((await inspection.background('/map.bmp')) instanceof Blob,true);
  assert.deepEqual(Object.keys(inspection).sort(),['art','background','kind','state']);
  assert.deepEqual(calls,[['state','/state'],['art'],['background','/map.bmp']]);
  assert.equal(fetches,0);
});
test('native inspection retains exact GET behavior',async()=>{
  const calls=[],fetchFn=async(path,options)=>{calls.push([path,options]);return {ok:true,json:async()=>({path})};};
  const inspection=createInspection({fetchFn,timeout:()=>({})});
  assert.equal(inspection.kind,'native-http');
  assert.deepEqual(await inspection.state(),{path:'/state'});
  assert.deepEqual(await inspection.art(),{path:'/art.json'});
  assert.equal(await inspection.background('/map.bmp'),'/map.bmp');
  assert.deepEqual(calls.map(([path])=>path),['/state','/art.json']);
});
test('requested local inspection never downgrades and operations are bounded',async()=>{
  for(const runtime of [undefined,null,{}, {request(){},loadArt(){},loadBackground:null}])
    assert.throws(()=>createInspection({runtime,localRequested:true}),/unavailable/);
  const never=new Promise(()=>{}),runtime={request:()=>never,loadArt:()=>never,loadBackground:()=>never};
  const inspection=createInspection({runtime,deadlineMs:1});
  await assert.rejects(inspection.state(),/timeout/);
  await assert.rejects(inspection.art(),/timeout/);
  await assert.rejects(inspection.background('/map.bmp'),/timeout/);
  await assert.rejects(bounded(never,'runtime readiness',1),/readiness timeout/);
});

test('local bootstrap rejects every unready or non-checkpoint state',()=>{
  const ready={status:'ready',read_ms:1,compile_ms:2,wasm_memory_bytes:3};
  const state={start_kind:'saved-checkpoint',tick:0,map_id:15,x:472,y:176};
  const $=id=>id==='local-rom-status'?{dataset:{kind:'ready'}}:id==='error'?{textContent:''}:id==='new-game'?{disabled:false}:null;
  checkLocalBootstrap(ready,$,state);
  for(const [preview,lookup,changed] of [
    [{...ready,status:'error'},$,state],
    [ready,id=>id==='local-rom-status'?{dataset:{kind:'error'}}:$(id),state],
    [ready,id=>id==='error'?{textContent:'bad'}:$(id),state],
    [ready,id=>id==='new-game'?{disabled:true}:$(id),state],
    [ready,$,{...state,start_kind:'new-game'}],
    [ready,$,{...state,x:471}],
  ]) assert.throws(()=>checkLocalBootstrap(preview,lookup,changed));
});
test('projection omits only proven paused no-ops; keeps original and UI identities',()=>{
  const {route,proof}=fixture(),p=projectRoute(route,proof);
  assert.deepEqual(p.steps.map(s=>[s.command,s.offlineTick,s.state.tick]),[[5,1,1],[6,4,2],[2,5,3]]);
  assert.deepEqual(p.omissions.map(s=>s.offlineTick),[2,3]);
  assert.equal(p.offlineTicks,5);assert.equal(p.steps.length,3);
});
test('projection fails on hidden progression, position, flags, missing proofs or wrong fresh replay',()=>{
  for(const mutate of [
    p=>p.offline[2].continuation='e'.repeat(64), p=>p.offline[2].state.x++,
    p=>p.offline[2].state.events.push(0x292),p=>delete p.offline[2].continuation,
    p=>p.projected[2].continuation='e'.repeat(64),p=>p.projected[2].state.tick=4,
    p=>p.projected[2].state.owner='presentation',p=>p.offline.pop(),
    p=>p.offline[0].state.tick=1,
  ]) {const {route,proof}=fixture();mutate(proof);assert.throws(()=>projectRoute(route,proof));}
});
test('manual inputs can never be omitted, nor can dialogue neutral that advances recovery',()=>{
  for(const command of [5,6,7,8,9,10]) {
    const {route,proof}=fixture();route.actions[1][0]=command;
    assert.throws(()=>projectRoute(route,proof));
  }
  const {route,proof}=fixture();proof.offline[2].state.owner='potrecovery';
  assert.throws(()=>projectRoute(route,proof),/paused|no-op/);
});
test('non-dialogue identical neutral ticks remain ordinary ticks',()=>{
  const {proof}=fixture();const a=proof.offline[0];
  const records=[a,{...a,state:{...a.state,tick:1}}];
  assert.equal(projectRoute({actions:[[0,1]]},{schema:1,offline:records,projected:records}).steps.length,1);
});
test('invalid commands/counts/empty or malformed routes fail closed',()=>{
  for(const actions of [[],[[11,1]],[[0,0]],[[0,1.2]],[[0,1,2]],[[0,100001]],[[null,1]]]) {
    assert.throws(()=>projectRoute({actions},fixture().proof));
  }
});
test('state checks compare every field except global tick, with exact projected tick',()=>{
  const s=fixture().proof.offline[0].state;
  checkState({...s,tick:4},s,4);
  assert.throws(()=>checkState({...s,tick:5},s,4));
  assert.throws(()=>checkState({...s,events:[32,38,251],tick:4},s,4));
  assert.throws(()=>checkState({...s,extra:true,tick:4},s,4));
});
test('projection uses normalized identity across clocks but browser checks exact raw snapshot',()=>{
  const {route,proof}=fixture();
  proof.offline.forEach((r,i)=>r.state.snapshot_sha256=String(i).repeat(64));
  proof.projected=clone(proof.projected);
  proof.projected.forEach((r,i)=>r.state.snapshot_sha256=String(i).repeat(64));
  const p=projectRoute(route,proof);
  assert.equal(p.steps[1].state.snapshot_sha256,'2'.repeat(64));
  assert.throws(()=>checkState({...p.steps[1].state,snapshot_sha256:'4'.repeat(64)},p.steps[1].state,2));
});
test('UI commands use actual native A button, manual ack/choices and Resume neutral',()=>{
  const log=[],buttons=Object.fromEntries(['pause','interact','continue','choice-cancel','choice1','choice2','pot-action'].map(id=>[id,{textContent:'Resume',click(){log.push(id);}}]));
  const motion={resume(){log.push('resume');},apply(c,s){log.push([c,s.phase]);},stop(){log.push('stop');}};
  const c=createCommands(id=>buttons[id],motion);
  for(let i=5;i<=10;i++)c.send(i,{dialogue:i===10?null:{}});
  assert.deepEqual(log,['interact','continue','choice-cancel','choice1','choice2','pot-action']);
  c.send(0,{dialogue:null,phase:'dialogue'});assert.deepEqual(log.slice(-2),['resume',[0,'dialogue']]);
  assert.throws(()=>c.send(0,{dialogue:{}}),/dialogue/);
  assert.throws(()=>c.send(10,{dialogue:{}}),/dialogue/);
  buttons['pot-action'].disabled=true;assert.throws(()=>c.send(10,{dialogue:null}),/Unavailable/);
  assert.throws(()=>c.send(11,{}),/command/);
});
function composition() {
  return {width:2,height:1,camera:[0,0],background:{width:2,height:1,data:[10,20,30,255,10,20,30,255]},
    base:{width:2,height:1,data:[0,0,0,255,10,20,30,255]},high:{width:2,height:1,data:[0,0,0,255,0,0,0,0]},
    patches:[{cell:0,tile:1}],sprites:[],actual:[10,20,30,255,10,20,30,255]};
}
const sprite=(id,priority,color)=>({id,priority,position:[0,0],offset:[0,0],source:[0,0,2,1],rgba:[...color,255,...color,255],flipX:false});
test('independent composition resolves OBJ winner before high BG and counts only visible differences',()=>{
  const f=composition();f.sprites=[sprite('rear',3,[4,5,6]),sprite('carry',2,[1,2,3])];f.actual=[10,20,30,255,1,2,3,255];
  const e=compareComposition(f);assert.equal(e.sprites.carry,1);assert.equal(e.sprites.rear,undefined);
  assert.equal(e.background,1);assert.equal(e.patches[0],1);
  f.actual[0]=4;assert.throws(()=>compareComposition(f),/pixel/);
  f.actual=[10,20,30,255,1,2,3,255];f.sprites[1].rgba.fill(0);assert.throws(()=>compareComposition(f),/pixel/);
});
test('offscreen/occluded/identical patch and carry are not visual evidence',()=>{
  const f=composition();f.base=clone(f.background);f.sprites=[{...sprite('carry',2,[1,2,3]),position:[10,10]}];
  const e=compareComposition(f);assert.deepEqual(e.patches,{});assert.deepEqual(e.sprites,{});
  assert.throws(()=>requireCoverage({backgrounds:{house:1},patches:{},carry:{}}));
});
test('source text RGBA maps back to exact indexed palette, rejecting unknown colors',()=>{
  const page={width:2,height:1,rgba:[0,0,0,0,240,244,248,255]};
  assert.deepEqual(Array.from(indexedPage(page,{width:2,height:1,background_index:3})),[3,1]);
  page.rgba[4]=241;assert.throws(()=>indexedPage(page,{width:2,height:1,background_index:3}),/palette/);
});
test('independent fixed semantic checkpoints retain precise grants and final movement square',()=>{
  const s={map_id:65,x:120,y:208,events:[38,40,39,46,658,34,579,580],owner:'player',phase:'walking',dialogue:null};
  checkSemanticCheckpoint(11314,s);
  for(const change of [{x:136},{events:s.events.filter(v=>v!==580)},{owner:'presentation'},{dialogue:{key:'text:89d735:3'}}])
    assert.throws(()=>checkSemanticCheckpoint(11314,{...s,...change}));
  assert.throws(()=>checkSemanticCheckpoint(964,{events:[38]}));
  checkSemanticCheckpoint(965,{events:[38]});
});
test('browser bootstrap forbids8765, requires explicit origin and retains async failure without inputs',async()=>{
  const fs=require('node:fs'),vm=require('node:vm'),code=fs.readFileSync(`${__dirname}/verify-pandora-browser.js`,'utf8');
  const base={document:{},location:{origin:'http://127.0.0.1:8765',port:'8765'},PANDORA_BROWSER_READY:'http://127.0.0.1:8765'};
  assert.throws(()=>vm.runInNewContext(code,base),/8765/);
  base.location={origin:'http://127.0.0.1:11599',port:'11599'};
  assert.throws(()=>vm.runInNewContext(code,base),/origin/);
  let inputs=0;
  const context={...base,PANDORA_BROWSER_READY:base.location.origin,
    HouseBrowserHelpers:{createControls:()=>({stop:()=>inputs++})},ConversationBrowserHelpers:{},RoomSlice:{},TextEncoder,crypto:globalThis.crypto};
  const result=vm.runInNewContext(code,context);
  assert.equal(typeof result,'string');assert.equal(context.PANDORA_BROWSER_RUN.status,'running');
  await context.PANDORA_BROWSER_RUN.promise;
  assert.equal(context.PANDORA_BROWSER_RUN.status,'failed');assert.equal(inputs,0);
  const incomplete={...context,PANDORA_BROWSER_RUN:undefined,HouseBrowserHelpers:{}};
  vm.runInNewContext(code,incomplete);
  await incomplete.PANDORA_BROWSER_RUN.promise;
  assert.equal(incomplete.PANDORA_BROWSER_RUN.status,'failed');
  assert.match(incomplete.PANDORA_BROWSER_RUN.error,/createControls/);assert.equal(inputs,0);
});
test('only explicit false readiness unblocks visible text; legacy undefined still blocks',()=>{
  for(const dialogue of [null,{key:'text:889e9c:0'}])for(const dialogue_ready of [undefined,true,false])
    assert.equal(blockingDialogue({dialogue,dialogue_ready}),dialogue!==null && dialogue_ready!==false);
  for(const dialogue_ready of [null,0,1,'false'])assert.throws(()=>blockingDialogue({dialogue:{},dialogue_ready}),/readiness/);
});
test('visible unready arrival/motion and identical ordinary neutral ticks are never omitted',()=>{
  const record=(tick,ready,x,identity)=>({state:{tick,dialogue:{key:'text:889e9c:0'},dialogue_ready:ready,x,phase:'arriving'},continuation:identity.repeat(64)});
  const records=[record(0,false,120,'a'),record(1,false,121,'b'),record(2,false,121,'b'),record(3,true,122,'c')];
  const route={actions:[[3,1],[0,2]]},proof={schema:1,offline:records,projected:clone(records)};
  const p=projectRoute(route,proof);
  assert.deepEqual(p.steps.map(s=>s.command),[3,0,0]);assert.deepEqual(p.omissions,[]);
  proof.projected.splice(2,1);assert.throws(()=>projectRoute(route,proof));
  const f=fixture();for(const records of [f.proof.offline,f.proof.projected])for(const r of records)r.state.dialogue_ready=true;
  assert.equal(projectRoute(f.route,f.proof).omissions.length,2);
  f.proof.offline[2].continuation='f'.repeat(64);assert.throws(()=>projectRoute(f.route,f.proof),/no-op/);
});
test('unready dialogue uses real Resume and transition-neutral controls; never sends ack/choice',()=>{
  const H=require('./verify-house-browser.js'),log=[];
  const buttons=Object.fromEntries(['pause','continue','choice-cancel','choice1','choice2','pot-action'].map(id=>[id,
    {textContent:'Resume',click(){log.push(id);}}]));
  const commands=createCommands(id=>buttons[id],H.createControls(id=>buttons[id],(type,key)=>log.push([type,key])));
  for(const phase of ['arriving','dialogue'])for(let command=0;command<=4;command++) {
    commands.send(command,{dialogue:{},dialogue_ready:false,phase,map_id:12});
    commands.stop();
  }
  assert.deepEqual(log,Array(10).fill('pause'),'only Resume: no arrow injected during transition/motion');
  for(let command=6;command<=10;command++)assert.throws(()=>commands.send(command,{dialogue:{},dialogue_ready:false}),/dialogue|ready/);
  assert.equal(log.length,10,'even incorrectly enabled manual buttons cannot bypass readiness');
});
test('visible unready text stays painted, disables ack/choices, and permits paused Resume/neutral',()=>{
  for(const ready of [true,false,undefined])for(const choice of [null,1]) {
    const blocked=ready!==false,s={dialogue:{key:'text:889e9c:0',...(choice===null?{}:{choice})},dialogue_ready:ready};
    const buttons={'dialogue-panel':{hidden:false},pause:{disabled:blocked},step:{disabled:blocked},interact:{disabled:true},'pot-action':{disabled:true},
      continue:{disabled:!blocked || choice!==null,hidden:choice!==null},'dialogue-choices':{hidden:choice===null}};
    for(const id of ['choice1','choice2','choice-cancel'])buttons[id]={disabled:!blocked || choice===null};
    checkDialogueControls(id=>buttons[id],s);
    for(const id of ['pause','step','continue','choice1','choice2','choice-cancel']) {
      buttons[id].disabled=!buttons[id].disabled;
      assert.throws(()=>checkDialogueControls(id=>buttons[id],s));
      buttons[id].disabled=!buttons[id].disabled;
    }
    buttons['dialogue-panel'].hidden=true;assert.throws(()=>checkDialogueControls(id=>buttons[id],s));
  }
});
