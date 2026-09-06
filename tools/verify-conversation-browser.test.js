'use strict';
// ROM-free, independent source fixtures. These are not generated from host metadata.
const {test}=require('node:test');
const assert=require('node:assert/strict');
const {validateDialogueArt,conversationPlan,checkConversationState,compareRaster,createAckLatch,createUICommands,motionRecipe,checkExterior}=require('./verify-conversation-browser.js');
const rows=[
  ['888fda',[[0x888fef,19,'end']]],
  ['888ff0',[[0x889027,37,'next'],[0x889059,36,'none']]],
  ['88905a',[[0x889083,39,'next'],[0x8890b0,33,'next'],[0x8890d8,36,'end']]],
  ['8890d9',[[0x8890f8,29,'next'],[0x889126,39,'next'],[0x889155,39,'end']]],
  ['889156',[[0x88918b,35,'none']]],
  ['88918c',[[0x8891aa,27,'next'],[0x8891d5,36,'end']]],
  ['8891d6',[[0x8891eb,20,'next'],[0x889217,38,'end']]],
];
function raster(width,height) {
  return {width,height,rgba:Array.from({length:width*height},(_,i)=>[i%2?240:0,0,0,255]).flat()};
}
function artFixture() {
  const dialogue_pages={},dialogue_requests={};
  for (const [source,pages] of rows) dialogue_requests[source]=pages.map(([boundary_source,glyph_count,acknowledgement],index)=> {
    const key=`text:${source}:${index}`;
    dialogue_pages[key]=raster(224,48);
    return {key,page_id:(parseInt(source,16)<<4)|index,boundary_source,glyph_count,acknowledgement};
  });
  for (let c=0;c<2;c++) for(let r=1;r<=2;r++) dialogue_pages[`choice:${c}:${r}`]=raster(212,16);
  return {dialogue_pages,dialogue_requests,choice_catalogs:{0:['choice:0:1','choice:0:2'],1:['choice:1:1','choice:1:2']}};
}
function state(key=null,choice=null,granted=true) {
  return {map_id:11,x:120,y:128,phase:key?'dialogue':'walking',dialogue:key?{key,choice}:null,
    events:granted?[32,38,251]:[32,251],choice_interaction:true,dialogue_acknowledgement:true,error:null};
}
test('all fourteen independent boundaries/logical IDs plus four exact source crops',()=> {
  validateDialogueArt(artFixture());
  for (const mutate of [
    a=>a.dialogue_requests['888ff0'][1].acknowledgement='end',
    a=>a.dialogue_requests['888ff0'][0].page_id=0x888ff0,
    a=>a.dialogue_requests['889156'][0].boundary_source=0x88917e,
    a=>a.dialogue_requests['8890d9'][1].glyph_count--,
    a=>a.dialogue_requests['88905a'].pop(),
    a=>a.choice_catalogs[0].reverse(),
    a=>a.dialogue_pages['choice:1:1'].rgba[0]=4,
    a=>a.dialogue_pages['text:88918c:1'].rgba.fill(0),
    a=>delete a.dialogue_pages['text:888fda:0'],
  ]) {const a=artFixture();mutate(a);assert.throws(()=>validateDialogueArt(a));}
});
test('fixed input/source flow grants before choice0, closes all followups, repeats cancel then option1',()=> {
  const plan=conversationPlan();
  assert.deepEqual(plan.map(p=>p.command),[5,6,8,6,6,6,5,7,6,6,5,8,6,6]);
  assert.deepEqual(plan.map(p=>p.key),[
    'text:888ff0:0','text:888ff0:1','text:8890d9:0','text:8890d9:1','text:8890d9:2',null,
    'text:889156:0','text:8891d6:0','text:8891d6:1',null,
    'text:889156:0','text:88918c:0','text:88918c:1',null]);
  for(const p of plan) checkConversationState(state(p.key,p.choice,p.granted),p);
  assert.throws(()=>checkConversationState(state(plan[0].key,null,true),plan[0]),/events/);
  assert.throws(()=>checkConversationState(state(plan[1].key,0,false),plan[1]),/events/);
  assert.throws(()=>checkConversationState(state(plan[1].key,null),plan[1]),/choice/);
  for(const mutate of [s=>s.x++,s=>s.y++,s=>s.map_id=12,s=>s.phase='walking',s=>s.events.push(39),s=>s.events.reverse(),s=>s.choice_interaction=false,s=>s.dialogue_acknowledgement=false]) {
    const s=state(plan[6].key,1);mutate(s);assert.throws(()=>checkConversationState(s,plan[6]));
  }
  const page=state(plan[0].key,null,false);delete page.dialogue.choice;
  checkConversationState(page,plan[0]); // absent is a page wait, not catalog0
});
test('raster checks detect actual pixel mismatch, empty/blank evidence and wrong dimensions',()=> {
  const frame=raster(2,1);
  assert.equal(compareRaster({width:2,height:1,data:frame.rgba},frame),1);
  assert.throws(()=>compareRaster({width:1,height:2,data:frame.rgba},frame),/dimensions/);
  assert.throws(()=>compareRaster({width:2,height:1,data:[...frame.rgba.slice(0,4),0,0,0,255]},frame),/pixel/);
  const blank={width:1,height:1,rgba:[0,0,0,255]};
  assert.throws(()=>compareRaster({width:1,height:1,data:blank.rgba},blank),/nonvacuous/);
  for (const color of [[36,47,55,255],[240,244,248,255]]) {
    const flat={width:2,height:1,rgba:[...color,...color]};
    assert.throws(()=>compareRaster({width:2,height:1,data:flat.rgba},flat),/nonvacuous/);
    const art=artFixture();
    for (const frame of Object.values(art.dialogue_pages)) frame.rgba=Array.from({length:frame.width*frame.height},()=>color).flat();
    assert.throws(()=>validateDialogueArt(art),/nonvacuous/); // matching uniform pages AND exact uniform crops
  }
});
test('one-shot latch stores last tick; duplicate busy renders do not ack; skips fail',()=> {
  const latch=createAckLatch(4);
  assert.equal(latch.observe(4),false);
  latch.arm(); assert.throws(()=>latch.arm(),/pending/);
  assert.equal(latch.observe(4),false);assert.equal(latch.observe(5),true);
  assert.equal(latch.lastTick,5);assert.equal(latch.observe(5),false);
  assert.throws(()=>latch.observe(6),/Unsolicited/);
  latch.arm();assert.throws(()=>latch.observe(7),/Non-sequential/);
});
test('motion preserves first15 house chunks and pins resident/exterior recipe',()=> {
  const recipe=motionRecipe();
  const original=require('node:fs').readFileSync(`${__dirname}/house-navigation-qualification/core-route.jsonl`,'utf8').trim().split('\n').slice(0,15).map(JSON.parse);
  assert.deepEqual(recipe.toB,original.map(({button,steps})=>[button,steps]));
  assert.deepEqual(recipe.target,[[3,100],[0,20]]);
  assert.deepEqual(recipe.down,[[4,32],[0,60]]);assert.deepEqual(recipe.right,[[2,24],[0,90]]);
  const s={map_id:10,x:538,y:815,camera:[410,703],phase:'walking'};
  checkExterior(s);
  for(const mutate of [s=>s.x=577,s=>s.y=849,s=>s.map_id=13,s=>s.camera[1]++,s=>s.x=487]) {
    const bad=structuredClone(s);mutate(bad);assert.throws(()=>checkExterior(bad));
  }
});

test('actual controls: movement gets one Resume, manual callbacks stay paused through page end',()=> {
  const events=[];
  const nodes=Object.fromEntries(['pause','interact','continue','choice-cancel','choice1','choice2'].map(id=>[id,
    {disabled:false,hidden:false,textContent:id==='pause'?'Resume':id,click(){events.push(id);if(id==='pause')this.textContent=this.textContent==='Pause'?'Resume':'Pause';}}]));
  const controls=createUICommands(id=>nodes[id],(type,key)=>events.push(`${type}:${key}`));
  controls.send(3,{phase:'walking'});
  assert.deepEqual(events,['pause','keydown:ArrowUp']);
  controls.pause();assert.equal(nodes.pause.textContent,'Resume');
  controls.send(0,{phase:'arriving'});controls.pause();
  controls.send(5,{phase:'walking'});
  nodes.pause.disabled=true;
  controls.send(6,{phase:'dialogue'});controls.pause();
  controls.send(8,{phase:'dialogue'});controls.send(7,{phase:'dialogue'});controls.send(9,{phase:'dialogue'});
  assert.deepEqual(events.slice(-5),['interact','continue','choice1','choice-cancel','choice2']);
  assert.throws(()=>controls.send(3,{phase:'dialogue'}),/dialogue/);
  nodes.continue.disabled=true;assert.throws(()=>controls.send(6,{phase:'dialogue'}),/Unavailable/);
  nodes.pause.disabled=false;controls.send(4,{phase:'walking'}); // explicit resume only after final ack
  assert.deepEqual(events.slice(-2),['pause','keydown:ArrowDown']);
  assert.throws(()=>controls.send(10,{phase:'walking'}),/command/);
});

test('browser script refuses to touch old backend without explicit readiness',()=> {
  const vm=require('node:vm'),fs=require('node:fs');
  const context={document:{},fetch(){assert.fail('must not fetch or drive before readiness');}};
  assert.throws(()=>vm.runInNewContext(fs.readFileSync(`${__dirname}/verify-conversation-browser.js`,'utf8'),context),/readiness/);
  assert.equal(context.CONVERSATION_BROWSER_RUN,undefined);
});
