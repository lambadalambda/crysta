// Dependency-free offline regression: node crates/map-inspector/tests/room-slice-check.js
// Exercises the actual inline controller with a synthetic host; no browser/ROM/server needed.
'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const html = fs.readFileSync(path.join(__dirname, '../web/room-slice.html'), 'utf8');
const scripts = [...html.matchAll(/<script>([\s\S]*?)<\/script>/g)];
assert.equal(scripts.length, 1);
const sandbox = {console};
vm.runInNewContext(scripts[0][1], sandbox);
const {createController, bindInputs, drawScene, prepareArt, selectActor, selectActors, selectBackground, selectDialogue, selectChoices} = sandbox.RoomSlice;
const initial = () => ({map_id: 15, x: 472, y: 176, tick: 0, phase: 'walking', error: null,
  policy: 'semantic-preview', camera: [256, 0], door_interaction:true});
const newGameState = () => ({...initial(), x: 304, y: 112, start_kind: 'new-game'});
const flush = async () => { for (let i = 0; i < 8; i++) await Promise.resolve(); };
function harness() {
  let nextId = 0, active = 0, maximum = 0, clock = 0;
  const timers = new Map(), calls = [], pending = [], views = [];
  const controller = createController({
    request(url, body) {
      calls.push({url, body, at: clock}); active++; maximum = Math.max(maximum, active);
      return new Promise((resolve, reject) => pending.push({resolve, reject}));
    },
    render: view => views.push(view),
    now: () => clock,
    schedule(fn, delay) { assert(Number.isInteger(delay) && delay >= 0); timers.set(++nextId, {fn, due: clock + delay}); return nextId; },
    cancel(id) { timers.delete(id); },
  });
  return {controller, calls, views, timers, maximum: () => maximum,
    async reply(state = initial()) { const p = pending.shift(); assert(p); active--; p.resolve(state); await flush(); },
    async fail() { const p = pending.shift(); active--; p.reject(new Error('offline')); await flush(); },
    async fire(earlyBy = 0) { assert.equal(timers.size, 1); const [id, task] = [...timers][0]; timers.delete(id); clock = task.due - earlyBy; task.fn(); await flush(); },
    async start() { controller.init(); await this.reply(); },
  };
}
class Target {
  constructor() { this.listeners = {}; this.hidden = false; this.dataset = {}; this.captured = new Set(); }
  addEventListener(name, fn) { (this.listeners[name] ||= []).push(fn); }
  emit(name, data = {}) {
    const event = {key: '', repeat: false, preventDefault() {}, ...data};
    for (const fn of this.listeners[name] || []) fn(event);
  }
  setPointerCapture(id) { this.captured.add(id); }
  hasPointerCapture(id) { return this.captured.has(id); }
  releasePointerCapture(id) { this.captured.delete(id); }
  getBoundingClientRect() { return {left: 0, top: 0, right: 50, bottom: 50}; }
}
// Run browser initialization too: helper-only tests cannot catch load/recovery bugs.
function browserHarness({stallBitmap = false, actorKey = '0:0', invalidScene = false, dialogueKey = null, dialogueChoice = null, mutateBundle = () => {}, badDimensions = false, failBitmap = false} = {}) {
  const elements = new Map(), timers = new Map(), requests = [], images = [], draws = []; let timerId = 0, key = actorKey, clock = 0;
  const context = {fillRect(){},drawImage(...args){draws.push(args);},save(){},restore(){},translate(){},putImageData(){},
    getImageData(x,y,width,height){return {width,height,data:new Uint8ClampedArray(width*height*4)};}};
  const element = id => {
    if (!elements.has(id)) {
      const target = new Target(); target.textContent=''; target.getContext=()=>context;
      elements.set(id,target);
    }
    return elements.get(id);
  };
  const doc = new Target(); doc.getElementById=element; doc.querySelectorAll=()=>[];
  doc.createElement=()=>({getContext:()=>context});
  const win = new Target();
  const bundle = {schema_version:1,scene_ids:{15:['ark']},frames:{'0:0':{width:1,height:1,offset:[0,0],rgba:[1,2,3,255]}},
    backgrounds:{house:{url:'/map.bmp',width:512,height:1024},exterior:{url:'/exterior.bmp',width:1024,height:1280}},
    background_keys:{10:'exterior',15:'house'},door_background:'house',
    foreground:{'10':{width:1024,height:1280,runs:[]},'15':{width:512,height:1024,runs:[]}},
    dialogue_pages:{'page:1':{width:2,height:1,rgba:[255,255,255,255,0,0,0,0]},'option:2':{width:1,height:1,rgba:[1,2,3,255]}},choice_catalogs:{0:['page:1','option:2']}};
  bundle.scene_ids[10]=['ark'];mutateBundle(bundle);
  const browser = {console,document:doc,window:win,performance:{now:()=>clock},AbortController,AbortSignal,Uint8ClampedArray,
    ImageData:class {constructor(data,width,height){Object.assign(this,{data,width,height});}},
    Image:class {set src(value){
      this.url=value;images.push(this);this.naturalWidth=value==='/exterior.bmp'?1024:512;
      this.naturalHeight=value==='/exterior.bmp'?1280:1024;if(badDimensions)this.naturalWidth=1;
      if(stallBitmap!==true && stallBitmap!==value) Promise.resolve().then(()=>failBitmap===true || failBitmap===value?this.onerror():this.onload());
    }},
    setTimeout(fn,ms){timers.set(++timerId,{fn,ms});return timerId;},clearTimeout(id){timers.delete(id);},
    async fetch(url,options){requests.push({url,...options});return {ok:true,async json(){return url==='/art.json'?bundle:{...(url==='/reset'?initial():newGameState()),actor_key:key,
      ...(url==='/step'&&dialogueKey?{phase:'dialogue',dialogue:{key:dialogueKey,...(dialogueChoice===null?{}:{choice:dialogueChoice})},dialogue_acknowledgement:true,choice_interaction:true}:{}),
      scene:[{id:'ark',key,position:invalidScene?[0,0]:url==='/reset'?[472,176]:[304,112]}]};}};},
  };
  vm.runInNewContext(scripts[0][1], browser);
  return {element,timers,requests,images,draws,advance(ms){clock+=ms;},setDialogueKey(value){dialogueKey=value;},setKey(value){key=value;},fixScene(){invalidScene=false;}};
}
async function main() {
  const loaded = browserHarness(); await flush(); await flush();
  assert.deepEqual(loaded.images.map(image=>image.url).sort(),['/exterior.bmp','/map.bmp']);
  assert.equal(loaded.element('error').textContent,'');
  loaded.element('reset').emit('click'); await flush();
  loaded.element('new-game').emit('click'); await flush();
  assert.equal(loaded.images.length,2,'starts reuse both preloaded sheets');
  for(const url of ['/other.bmp','//evil.test/map.bmp','/map.bmp?x=1','/../map.bmp']) {
    const invalid=browserHarness({mutateBundle:b=>b.backgrounds.house.url=url});await flush();await flush();
    assert.match(invalid.element('error').textContent,/background/i);
    assert.equal(invalid.images.length,0,'validate entire manifest before requesting images');
  }
  for(const options of [{badDimensions:true},{failBitmap:'/exterior.bmp'}]) {
    const failed=browserHarness(options);await flush();await flush();
    assert.match(failed.element('error').textContent,/Art load failed/);
    assert.equal(failed.element('pause').disabled,true);
    failed.element('reset').emit('click');await flush();
    assert.match(failed.element('error').textContent,/Art load failed/);
  }
  const recovery = browserHarness({actorKey:'unsupported'}); await flush(); await flush();
  assert.match(recovery.element('error').textContent, /sprite/i);
  recovery.element('new-game').emit('click'); await flush();
  assert.equal(recovery.element('pause').disabled, true, 'a repeated bad key must still reject a new start');
  recovery.element('demo').emit('click'); await flush();
  assert.equal(recovery.element('pause').disabled, true);
  assert.equal(recovery.timers.size, 0, 'invalid art cannot start demo stepping');
  recovery.setKey('0:0'); recovery.element('new-game').emit('click'); await flush();
  assert.equal(recovery.element('error').textContent, '', 'valid New Game reuses cached art after a bad key');
  assert.equal(recovery.element('pause').disabled, false);
  const recoveredDemo = browserHarness({actorKey:'unsupported'}); await flush(); await flush();
  recoveredDemo.setKey('0:0'); recoveredDemo.element('demo').emit('click'); await flush();
  assert.equal(recoveredDemo.element('error').textContent, '');
  assert.equal(recoveredDemo.timers.size, 1, 'valid demo replacement must retain autoplay intent');
  for (const validStart of ['new-game','demo']) {
    const badScene = browserHarness({invalidScene:true}); await flush(); await flush();
    assert.match(badScene.element('error').textContent, /scene/i);
    for (const control of ['new-game','demo']) {
      badScene.element(control).emit('click'); await flush();
      assert.equal(badScene.element('pause').disabled, true);
      assert.equal(badScene.timers.size, 0, 'invalid scene cannot start stepping');
    }
    badScene.fixScene(); badScene.element(validStart).emit('click'); await flush();
    assert.equal(badScene.element('error').textContent, '');
    assert.equal(badScene.element('pause').disabled, false);
    assert.equal(badScene.timers.size, validStart==='demo'?1:0);
  }
  for(const control of ['new-game','reset']) {
    const loading=browserHarness({stallBitmap:'/exterior.bmp'});await flush();await flush();
    assert.equal(loading.element('pause').disabled,true,'house alone cannot make art ready');
    loading.element(control).emit('click');await flush();
    assert.equal(loading.element('pause').disabled,true);
    loading.images.find(image=>image.url==='/exterior.bmp').onload();await flush();await flush();
    assert.equal(loading.element('pause').disabled,false);
    assert.equal(loading.element('error').textContent,'');
    assert.equal(loading.images.length,2);
    assert.equal(loading.requests.filter(request=>request.url==='/step').length,0,'loading/start never autosteps');
  }
  const stalled = browserHarness({stallBitmap:'/exterior.bmp'}); await flush();
  const deadlines=[...stalled.timers.values()].filter(t => t.ms===5000);
  assert.equal(deadlines.length,1); deadlines[0].fn(); await flush();
  assert.match(stalled.element('error').textContent, /timed out/i);
  stalled.images.forEach(image=>image.onload());await flush();await flush();
  assert.match(stalled.element('error').textContent,/timed out/i,'late image completion cannot resurrect timed-out art');
  assert.equal(stalled.element('pause').disabled,true);
  for(const key of ['page:1','missing']) {
    const ui=browserHarness({dialogueKey:key}); await flush(); await flush();
    ui.element('interact').emit('click');
    const stepTimer=[...ui.timers].find(([,timer])=>timer.ms<100); assert(stepTimer);
    ui.timers.delete(stepTimer[0]);stepTimer[1].fn();await flush();await flush();
    if(key==='page:1') {
      assert.equal(ui.element('dialogue-panel').hidden,false);
      assert.equal(ui.element('dialogue-page').width,2);assert.equal(ui.element('dialogue-page').height,1);
      assert.equal(ui.element('dialogue-page').dataset.key,'page:1');
      assert.equal(ui.element('continue').disabled,false);assert.equal(ui.element('pause').disabled,true);
      ui.setDialogueKey('missing');ui.element('continue').emit('click');
      const nextTimer=[...ui.timers].find(([,timer])=>timer.ms<100);assert(nextTimer);
      ui.timers.delete(nextTimer[0]);ui.advance(nextTimer[1].ms);nextTimer[1].fn();await flush();await flush();
      assert.match(ui.element('error').textContent,/dialogue/i);
      assert.equal(ui.element('dialogue-panel').hidden,true,'invalid next page hides previously visible text');
      assert.equal(ui.element('continue').disabled,true);
    } else {
      assert.match(ui.element('error').textContent,/dialogue/i);
      assert.equal(ui.element('dialogue-panel').hidden,true);
      assert.equal(ui.element('continue').disabled,true);
    }
  }
  // Dialogue owns control; each acknowledgement is one paced command, never autoplay.
  const talk=harness(); await talk.start();
  const page = (key,tick) => ({...initial(),tick,phase:'dialogue',dialogue:{key,choice:null},dialogue_acknowledgement:true});
  talk.controller.resume(); talk.controller.press('prior-held',2); await talk.fire();
  await talk.reply({...page('test:1',1),phase:'walking'});
  assert.equal(talk.views.at(-1).paused,true);
  assert.equal(talk.views.at(-1).walking,false);
  assert.equal(talk.timers.size,0);
  talk.controller.resume(); talk.controller.press('held',2); talk.controller.stepOnce();
  assert.equal(talk.timers.size,0,'movement and neutral stepping cannot advance dialogue');
  talk.controller.acknowledge(); talk.controller.acknowledge();
  assert.equal(talk.timers.size,1); await talk.fire();
  assert.equal(talk.calls.at(-1).body,'6');
  talk.controller.acknowledge(); assert.equal(talk.timers.size,0,'busy ack cannot replay');
  await talk.reply(page('test:2',2));
  assert.equal(talk.timers.size,0); assert.equal(talk.views.at(-1).paused,true);
  talk.controller.interact(); await talk.fire(); assert.equal(talk.calls.at(-1).body,'6');
  await talk.reply({...initial(),tick:3,dialogue:null,dialogue_acknowledgement:true});
  assert.equal(talk.views.at(-1).paused,true); assert.equal(talk.views.at(-1).walking,true);
  talk.controller.acknowledge(); assert.equal(talk.timers.size,0);
  talk.controller.resume(); await talk.fire(); assert.equal(talk.calls.at(-1).body,'0','held movement from dialogue was discarded');
  await talk.reply(page('test:3',4)); talk.controller.acknowledge(); talk.controller.newGame();
  assert.equal(talk.timers.size,0,'New Game cancels a pending page acknowledgement'); await talk.reply(newGameState());
  const noAck=harness(); await noAck.start(); noAck.controller.resume(); await noAck.fire();
  await noAck.reply({...page('test:1',1),dialogue_acknowledgement:undefined});
  noAck.controller.acknowledge(); noAck.controller.interact(); assert.equal(noAck.timers.size,0,'ack capability is required');
  const resetAck=harness(); await resetAck.start(); resetAck.controller.resume(); await resetAck.fire();
  await resetAck.reply(page('test:1',1)); resetAck.controller.acknowledge(); await resetAck.fire();
  resetAck.controller.reset(); await resetAck.reply(page('test:2',2));
  assert.equal(resetAck.calls.at(-1).url,'/reset'); await resetAck.reply(initial());
  assert.equal(resetAck.timers.size,0); assert.equal(resetAck.views.at(-1).dialogue,false);
  const pageBindings = new Target(), pageWindow = new Target();
  bindInputs(talk.controller,pageBindings,pageWindow,[]);
  for(const id of ['continue','choice1','choice2','choice-cancel']) for(const repeat of [false,true]) {
    let prevented=false;
    pageBindings.emit('keydown',{key:'Enter',repeat,target:{closest:()=>({id})},preventDefault(){prevented=true;}});
    assert.equal(prevented,repeat,'only repeated native button clicks are suppressed');
  }
  for(const contextual of [false,true]) for(const catalog of [0,1]) {
    const ui=browserHarness({dialogueKey:'page:1',dialogueChoice:catalog,mutateBundle:bundle=> {
      if(contextual) {
        bundle.dialogue_choice_contexts={'page:1':{catalog:0,options:['option:2','page:1']}};
        bundle.choice_catalogs[1]=['page:1','option:2']; // A valid fallback must not hide a context mismatch.
      }
    }});await flush();await flush();
    ui.element('interact').emit('click');
    const timer=[...ui.timers].find(([,timer])=>timer.ms<100);assert(timer);
    ui.timers.delete(timer[0]);ui.advance(timer[1].ms);timer[1].fn();await flush();await flush();
    if(catalog===0) {
      assert.equal(ui.element('continue').hidden,true);assert.equal(ui.element('dialogue-choices').hidden,false);
      assert.equal(ui.element('choice2').disabled,false);assert.equal(ui.element('choice1-label').dataset.key,contextual?'option:2':'page:1');assert.equal(ui.element('choice2-label').dataset.key,contextual?'page:1':'option:2');
      ui.element('choice2').emit('click');
      for(const id of ['choice1','choice2','choice-cancel']) assert.equal(ui.element(id).disabled,true);
      const next=[...ui.timers].find(([,timer])=>timer.ms<100);assert(next);
      ui.timers.delete(next[0]);ui.advance(next[1].ms);next[1].fn();await flush();await flush();
      assert.equal(ui.requests.at(-1).body,'9');
    } else {
      assert.match(ui.element('error').textContent,/choice/i);assert.equal(ui.element('dialogue-panel').hidden,true);
      assert.equal(ui.element('choice2').disabled,true);
    }
  }
  for(const invalid of [-1,65536,0.5,'0']) {
    const bad=harness();await bad.start();bad.controller.resume();await bad.fire();
    await bad.reply({...page('prompt',1),dialogue:{key:'prompt',choice:invalid}});
    assert.match(bad.views.at(-1).error,/Invalid host state/);assert.equal(bad.timers.size,0);
  }
  // A source choice is not an acknowledgement or an implicit first option.
  for(const selection of [0,1,2]) {
    const choice=harness();await choice.start();choice.controller.resume();await choice.fire();
    const pendingChoice={...page('prompt:0',1),dialogue:{key:'prompt:0',choice:0},choice_interaction:true};
    await choice.reply(pendingChoice);
    choice.controller.acknowledge();choice.controller.interact();choice.controller.choose(3);
    const keys=new Target();bindInputs(choice.controller,keys,new Target(),[]);keys.emit('keydown',{key:'Enter'});
    assert.equal(choice.timers.size,0,'choice needs an explicit valid selection');
    choice.controller.choose(selection);choice.controller.choose(2);await choice.fire();
    assert.equal(choice.calls.at(-1).body,String(7+selection));
    choice.controller.choose(2);assert.equal(choice.timers.size,0,'in-flight selection is not replayed');
    await choice.reply(page('followup:0',2));
    choice.controller.choose(selection);assert.equal(choice.timers.size,0,'choice cannot acknowledge a follow-up');
  }
  const noChoice=harness();await noChoice.start();noChoice.controller.resume();await noChoice.fire();
  await noChoice.reply({...page('prompt:0',1),dialogue:{key:'prompt:0',choice:0}});
  noChoice.controller.choose(1);assert.equal(noChoice.timers.size,0,'choice capability required');
  // Door interaction is one paced command, never a held direction or autoplay.
  const door = harness(); await door.start(); door.controller.interact();
  assert.equal(door.calls.length,1); door.controller.interact(); door.controller.stepOnce();
  assert.equal(door.timers.size,1); await door.fire();
  assert.equal(door.calls.at(-1).body,'5'); await door.reply({...initial(),tick:1});
  assert.equal(door.views.at(-1).paused,true); assert.equal(door.timers.size,0);
  door.controller.interact(); door.controller.newGame();
  assert.equal(door.timers.size,0); await door.reply(newGameState());
  assert.equal(door.calls.filter(call=>call.body==='5').length,1,'new start cancels undispatched action');
  door.controller.resume(); door.controller.press('held',3); door.controller.interact();
  assert.match(door.views.at(-1).note,/release/i); assert.equal(door.views.at(-1).error,null);
  door.controller.release('held'); door.controller.pause();
  door.controller.interact(); door.controller.setHidden(true);
  assert.equal(door.timers.size,0,'hidden tab cancels action');
  const incapable=harness(); incapable.controller.init(); await incapable.reply({...initial(),door_interaction:undefined});
  incapable.controller.interact(); assert.equal(incapable.timers.size,0);
  const transitionDoor=harness(); await transitionDoor.start(); transitionDoor.controller.resume(); await transitionDoor.fire();
  await transitionDoor.reply({...initial(),phase:'departing'}); transitionDoor.controller.pause();
  transitionDoor.controller.interact(); assert.equal(transitionDoor.timers.size,0);
  const busyDoor=harness(); await busyDoor.start(); busyDoor.controller.resume(); await busyDoor.fire();
  busyDoor.controller.interact(); assert.equal(busyDoor.timers.size,0);
  await busyDoor.reply({...initial(),tick:1}); await busyDoor.fire();
  assert.equal(busyDoor.calls.at(-1).body,'0'); await busyDoor.reply({...initial(),tick:2}); busyDoor.controller.pause();
  assert(!busyDoor.calls.some(call=>call.body==='5'),'busy action must not replay later');
  const demoDoor=harness(); await demoDoor.start(); demoDoor.controller.demo(); demoDoor.controller.interact();
  await demoDoor.reply(initial()); demoDoor.controller.interact(); await demoDoor.fire();
  assert.equal(demoDoor.calls.at(-1).body,'1'); await demoDoor.reply({...initial(),tick:1});
  assert.equal(demoDoor.views.at(-1).demoIndex,1); assert.equal(demoDoor.views.at(-1).mode,'demo');
  assert(!demoDoor.calls.some(call=>call.body==='5')); demoDoor.controller.pause();
  const actionKeys=harness(); await actionKeys.start();
  const actionDoc=new Target(), actionWin=new Target(); bindInputs(actionKeys.controller,actionDoc,actionWin,[]);
  actionDoc.emit('keydown',{key:'Enter',target:{closest:()=>({})}});
  actionDoc.emit('keydown',{key:' ',repeat:true}); assert.equal(actionKeys.timers.size,0);
  actionDoc.emit('keydown',{key:' '}); await actionKeys.fire();
  assert.equal(actionKeys.calls.at(-1).body,'5'); await actionKeys.reply(initial());
  actionDoc.emit('keydown',{key:'Enter'}); actionKeys.controller.pause();
  assert.equal(actionKeys.timers.size,0);

  // Start paused; requests and timers never overlap or accumulate a catch-up queue.
  const h = harness(); await h.start();
  assert.equal(h.calls[0].url, '/state'); assert.equal(h.timers.size, 0);
  h.controller.resume(); h.controller.press('key:a', 1); await h.fire();
  assert.equal(h.calls.at(-1).body, '1'); assert.equal(h.timers.size, 0);
  h.controller.stepOnce(); h.controller.resume();
  assert.equal(h.calls.length, 2, 'busy step/resume cannot send another request');
  h.controller.release('key:a'); await h.reply({...initial(), tick: 1}); await h.fire();
  assert.equal(h.calls.at(-1).body, '0', 'release cannot leave a buffered direction');
  await h.reply({...initial(), tick: 2}); h.controller.pause();
  assert.equal(h.timers.size, 0); const beforeManual = h.calls.length; h.controller.stepOnce();
  assert.equal(h.calls.length, beforeManual, 'single-step must obey the same pacing gate');
  await h.fire(); assert.equal(h.calls.at(-1).body, '0'); await h.reply({...initial(), tick: 3});
  assert.equal(h.timers.size, 0, 'single step stays paused');
  assert.equal(h.calls.filter(c => c.url === '/state').length, 1, 'no polling GET flood');
  assert.equal(h.maximum(), 1);
  for (let i = 0; i < 4; i++) { h.controller.stepOnce(); await h.fire(); await h.reply(); }
  const times = h.calls.filter(c => c.url === '/step').map(c => c.at);
  for (let i = 1; i < times.length; i++) assert(times[i] - times[i - 1] >= 1000 / 60, 'all /step dispatches respect 60 Hz, including manual clicks');

  const early = harness(); await early.start(); early.controller.resume(); await early.fire(); await early.reply();
  const beforeEarly = early.calls.length; await early.fire(1);
  assert.equal(early.calls.length, beforeEarly, 'an early timer cannot dispatch before the rate limit');
  await early.fire(); await early.reply(); early.controller.pause();
  early.controller.stepOnce(); early.controller.stepOnce(); early.controller.stepOnce();
  assert.equal(early.timers.size, 1, 'repeated pending manual clicks coalesce');
  await early.fire(); await early.reply(); assert.equal(early.timers.size, 0);

  // Phase changes discard held input, and every transition tick is neutral.
  h.controller.resume(); h.controller.press('key:s', 4); await h.fire();
  await h.reply({...initial(), phase: 'transition', tick: 80});
  h.controller.press('key:a', 1); await h.fire();
  assert.equal(h.calls.at(-1).body, '0');
  await h.reply({...initial(), map_id: 16, phase: 'walking', tick: 115, camera: [256, 256]});
  await h.fire(); assert.equal(h.calls.at(-1).body, '0', 'no held input across handoff');
  await h.reply({...initial(), map_id: 16});

  // Unsupported host steps pause, expose the error and require an explicit reset.
  await h.fire(); await h.reply({...initial(), error: 'flagged cell unsupported'});
  assert.equal(h.timers.size, 0); assert.match(h.views.at(-1).error, /flagged cell/);
  h.controller.resume(); h.controller.stepOnce(); assert.equal(h.timers.size, 0);
  const blockedCalls = h.calls.length; h.controller.press('bad', 9);
  assert.equal(h.calls.length, blockedCalls);
  h.controller.reset(); assert.equal(h.calls.at(-1).url, '/reset');
  assert.equal(h.calls.at(-1).body, ''); await h.reply();
  assert.equal(h.views.at(-1).error, null); assert.equal(h.views.at(-1).paused, true);

  // Reset queues behind an in-flight tick, discards held keys and never races it.
  h.controller.resume(); h.controller.press('key:a', 1); await h.fire();
  h.controller.reset(); assert.equal(h.calls.at(-1).url, '/step');
  await h.reply({...initial(), tick: 1}); assert.equal(h.calls.at(-1).url, '/reset');
  await h.reply(); assert.equal(h.maximum(), 1); assert.equal(h.timers.size, 0);

  // New Game is an explicit empty-body POST, clears errors/inputs and stays paused.
  const game = harness(); await game.start(); game.controller.resume();
  game.controller.press('held-before-start', 1); await game.fire();
  await game.reply({...initial(), error: 'unsupported action'});
  game.controller.newGame();
  assert.equal(game.calls.at(-1).url, '/new-game'); assert.equal(game.calls.at(-1).body, '');
  assert.equal(game.views.at(-1).error, null, 'starting clears the previous error');
  game.controller.press('held-during-start', 2); game.controller.resume();
  await game.reply(newGameState());
  assert.equal(game.views.at(-1).state.x, 304); assert.equal(game.views.at(-1).state.y, 112);
  assert.equal(game.views.at(-1).canDrive, true); assert.equal(game.views.at(-1).paused, true);
  assert.equal(game.views.at(-1).mode, 'live'); assert.equal(game.timers.size, 0);
  game.controller.resume(); await game.fire();
  assert.equal(game.calls.at(-1).body, '0', 'New Game cannot retain old or mid-request controls');
  await game.reply({...newGameState(), tick: 1}); game.controller.pause();
  game.controller.stepOnce(); game.controller.newGame();
  assert.equal(game.timers.size, 0, 'New Game cancels an undispatched single-step');
  await game.reply(newGameState()); assert.equal(game.timers.size, 0);

  // Latest pending start intent wins; in-flight ticks never race reset/new-game/demo.
  for (const first of ['reset', 'newGame', 'demo']) {
    for (const last of ['reset', 'newGame', 'demo']) {
      const queued = harness(); await queued.start(); queued.controller.resume();
      queued.controller.press('held', 1); await queued.fire();
      queued.controller[first](); queued.controller[last]();
      assert.equal(queued.calls.at(-1).url, '/step');
      queued.controller.resume(); queued.controller.stepOnce();
      await queued.fail(); // Obsolete failure must not cancel the newer demo intent.
      assert.equal(queued.calls.at(-1).url, last === 'newGame' ? '/new-game' : '/reset');
      assert.equal(queued.calls.at(-1).body, ''); assert.equal(queued.views.at(-1).error, null);
      await queued.reply(last === 'newGame' ? newGameState() : initial());
      assert.equal(queued.views.at(-1).paused, last !== 'demo');
      assert.equal(queued.views.at(-1).mode, last === 'demo' ? 'demo' : 'live');
      assert.equal(queued.maximum(), 1);
      if (last !== 'demo') queued.controller.resume();
      await queued.fire(); assert.equal(queued.calls.at(-1).body, last === 'demo' ? '1' : '0');
      await queued.reply(); queued.controller.pause();
    }
  }

  // A start response cannot borrow the autoplay intent of a newer queued start.
  for (const [first, last] of [['demo', 'newGame'], ['newGame', 'demo'], ['newGame', 'reset'], ['reset', 'newGame']]) {
    const queued = harness(); await queued.start(); queued.controller[first](); queued.controller[last]();
    await queued.reply(first === 'newGame' ? newGameState() : initial());
    assert.equal(queued.calls.at(-1).url, last === 'newGame' ? '/new-game' : '/reset');
    assert.equal(queued.timers.size, 0, 'no stale autoplay between start requests');
    await queued.reply(last === 'newGame' ? newGameState() : initial());
    assert.equal(queued.views.at(-1).paused, last !== 'demo');
    assert.equal(queued.maximum(), 1); queued.controller.pause();
  }
  const pausedStart = harness(); await pausedStart.start(); pausedStart.controller.newGame();
  pausedStart.controller.demo(); pausedStart.controller.pause(); await pausedStart.reply(newGameState());
  await pausedStart.reply(); assert.equal(pausedStart.timers.size, 0, 'pause cancels queued demo autoplay');

  // Learn the normal phase only at known starts, with optional provenance metadata.
  for (const start of [newGameState(), {...newGameState(), start_kind: undefined}, initial()]) {
    const known = harness(); known.controller.init(); await known.reply(start);
    assert.equal(known.views.at(-1).canDrive, true); assert.equal(known.views.at(-1).paused, true);
  }
  const unknown = harness(); unknown.controller.init();
  await unknown.reply({...newGameState(), x: 305}); unknown.controller.resume();
  assert.equal(unknown.views.at(-1).canDrive, false); assert.equal(unknown.timers.size, 0);
  assert.match(unknown.views.at(-1).note, /New Game/);
  unknown.controller.newGame(); await unknown.reply({...newGameState(), tick: 1});
  assert.match(unknown.views.at(-1).error, /start|checkpoint/i);
  assert.equal(unknown.timers.size, 0);
  unknown.controller.newGame(); await unknown.reply({...newGameState(), start_kind: 7});
  assert.match(unknown.views.at(-1).error, /state|policy/i);
  unknown.controller.newGame(); await unknown.reply({...newGameState(), start_kind: undefined});
  assert.equal(unknown.views.at(-1).error, null); assert.equal(unknown.views.at(-1).canDrive, true);

  const duringInit = harness(); duringInit.controller.init(); duringInit.controller.newGame();
  assert.equal(duringInit.calls.length, 1, 'New Game waits for the initial GET too');
  await duringInit.reply({...initial(), tick: 81, phase: 'FadeOut'});
  assert.equal(duringInit.calls.at(-1).url, '/new-game');
  await duringInit.reply(newGameState()); assert.equal(duringInit.views.at(-1).paused, true);
  duringInit.controller.newGame(); await duringInit.fail();
  assert.match(duringInit.views.at(-1).error, /offline/); assert.equal(duringInit.timers.size, 0);
  assert.equal(duringInit.calls.filter(call => call.url === '/reset').length, 0, 'failed New Game never falls back to a checkpoint');

  // Demo always resets first; exactly 56 Left + 24 Down + 35 neutral requests.
  const d = harness(); await d.start(); d.controller.demo();
  assert.equal(d.calls.at(-1).url, '/reset'); await d.reply();
  for (let i = 0; i < 115; i++) {
    d.controller.press('noise', 2); // live input is ignored throughout the demo
    await d.fire();
    assert.equal(d.calls.at(-1).body, i < 56 ? '1' : i < 80 ? '4' : '0');
    await d.reply({...initial(), tick: i + 1, phase: i >= 79 && i < 114 ? 'transition' : 'walking'});
  }
  assert.equal(d.calls.filter(c => c.url === '/step').length, 115);
  assert.equal(d.timers.size, 0); assert.equal(d.views.at(-1).paused, true);
  assert.equal(d.maximum(), 1);

  // Interruptions do not auto-resume a reset/demo or queue stale commands.
  const interrupted = harness(); await interrupted.start(); interrupted.controller.demo();
  interrupted.controller.pause('blur'); await interrupted.reply();
  assert.equal(interrupted.timers.size, 0, 'blur during demo reset cancels autoplay');
  interrupted.controller.demo(); await interrupted.reply(); await interrupted.fire();
  interrupted.controller.pause('Paused explicitly'); await interrupted.reply({...initial(), tick: 1});
  assert.equal(interrupted.views.at(-1).note, 'Paused explicitly');
  assert.equal(interrupted.timers.size, 0, 'pause during demo request survives its response');
  interrupted.controller.resume(); await interrupted.fire();
  assert.equal(interrupted.calls.at(-1).body, '1');
  interrupted.controller.reset(); await interrupted.reply({...initial(), tick: 2});
  assert.equal(interrupted.calls.at(-1).url, '/reset'); await interrupted.reply();
  assert.equal(interrupted.timers.size, 0); assert.equal(interrupted.views.at(-1).mode, 'live');
  interrupted.controller.stepOnce(); interrupted.controller.pause();
  assert.equal(interrupted.timers.size, 0, 'pause cancels a paced single-step before dispatch');
  assert.equal(interrupted.maximum(), 1);

  // Real input bindings: key release/repeat, blur/hidden, touch outside/cancel/up.
  const k = harness(); await k.start(); const doc = new Target(), win = new Target();
  const button = new Target(); button.dataset.direction = '1';
  bindInputs(k.controller, doc, win, [button]);
  k.controller.resume(); doc.emit('keydown', {key: 'a'}); doc.emit('keyup', {key: 'a'});
  await k.fire(); assert.equal(k.calls.at(-1).body, '0'); await k.reply();
  doc.emit('keydown', {key: 'ArrowLeft'}); win.emit('blur');
  assert.equal(k.timers.size, 0); k.controller.resume();
  doc.emit('keydown', {key: 'ArrowLeft', repeat: true});
  await k.fire(); assert.equal(k.calls.at(-1).body, '0'); await k.reply();
  for (const ending of ['pointerup', 'pointercancel', 'pointerleave', 'lostpointercapture', 'outside']) {
    button.emit('pointerdown', {pointerId: 7, button: 0});
    if (ending === 'outside') button.emit('pointermove', {pointerId: 7, clientX: 99, clientY: 20});
    else button.emit(ending, {pointerId: 7});
    await k.fire(); assert.equal(k.calls.at(-1).body, '0', ending); await k.reply();
  }
  button.emit('pointerdown', {pointerId: 8, button: 0}); await k.fire();
  assert.equal(k.calls.at(-1).body, '1'); await k.reply();
  doc.hidden = true; doc.emit('visibilitychange'); assert.equal(k.timers.size, 0);
  doc.hidden = false; doc.emit('visibilitychange'); assert.equal(k.timers.size, 0, 'visibility does not auto-resume');
  k.controller.resume(); doc.emit('keydown', {key: 'a'}); doc.emit('keydown', {key: 'w'});
  assert.match(k.views.at(-1).error, /one direction/i); assert.equal(k.timers.size, 0);

  // Phase labels are protocol strings, not a guessed enum; learn only at reset.
  const p = harness(); p.controller.init(); await p.reply({...initial(), phase: 'Ready'});
  p.controller.resume(); p.controller.press('key:a', 1); await p.fire();
  assert.equal(p.calls.at(-1).body, '1'); await p.reply({...initial(), phase: 'FadeOut', tick: 80});
  await p.fire(); assert.equal(p.calls.at(-1).body, '0'); await p.reply({...initial(), phase: 'Ready', tick: 115});
  const reload = harness(); reload.controller.init(); await reload.reply({...initial(), tick: 81, phase: 'FadeOut'});
  reload.controller.resume(); assert.equal(reload.timers.size, 0);
  assert.match(reload.views.at(-1).note, /Reset/);
  reload.controller.reset(); await reload.reply({...initial(), phase: 'Ready'});
  reload.controller.resume(); reload.controller.press('key:d', 2); await reload.fire();
  assert.equal(reload.calls.at(-1).body, '2'); await reload.reply({...initial(), phase: 'Ready'});

  // Transport/schema failures are visible and cannot keep sending ticks.
  const n = harness(); await n.start(); n.controller.resume(); await n.fire(); await n.fail();
  assert.match(n.views.at(-1).error, /offline/); assert.equal(n.timers.size, 0);
  n.controller.reset(); await n.reply({...initial(), policy: 'not-the-preview'});
  assert.match(n.views.at(-1).error, /state|policy/i);

  // Actual drawing helper uses host camera, fixed viewport and cyan feet-anchored box.
  const draws = []; const ctx = {fillRect() {}, drawImage(...args) { draws.push(args); },
    strokeRect(...args) { draws.push(args); }};
  const image = {}; drawScene(ctx, image, {...initial(), x: 392, y: 353, camera: [256, 256]});
  assert.deepEqual(draws[0], [image, 256, 256, 256, 224, 0, 0, 256, 224]);
  assert.deepEqual(draws[1], [128, 81, 16, 16]); assert.equal(ctx.strokeStyle, '#58f5ff');
  assert.equal(ctx.imageSmoothingEnabled, false);
  // A supplied player raster replaces (not overlays) the diagnostic marker.
  // Its source rectangle and signed anchor offset must survive camera movement.
  const spriteCalls = [], texture = {};
  const spriteCtx = {fillRect(){}, drawImage(...args){spriteCalls.push(['draw',...args]);},
    save(){spriteCalls.push(['save']);}, restore(){spriteCalls.push(['restore']);},
    translate(...args){spriteCalls.push(['translate',...args]);}, scale(...args){spriteCalls.push(['scale',...args]);},
    strokeRect(){throw new Error('sprite rendering must replace the marker');}};
  const actor = {image:texture, source:[16,32,24,32], offset:[-10,-28], flipX:false};
  drawScene(spriteCtx, image, {...initial(),x:392,y:353,camera:[256,256]}, actor);
  assert.deepEqual(spriteCalls, [
    ['draw',image,256,256,256,224,0,0,256,224], ['save'], ['translate',136,97],
    ['draw',texture,16,32,24,32,-10,-28,24,32], ['restore'],
  ]);
  spriteCalls.length=0;
  drawScene(spriteCtx, null, {...initial(),x:304,y:112}, {...actor,flipX:true});
  assert.deepEqual(spriteCalls, [
    ['save'], ['translate',48,112], ['scale',-1,1],
    ['draw',texture,16,32,24,32,-10,-28,24,32], ['restore'],
  ]);

  // Ordered scene entries have independent world anchors, not player-relative positions.
  spriteCalls.length = 0;
  const npcTexture = {}, foregroundTexture = {};
  drawScene(spriteCtx, image, {...initial(),x:392,y:353,camera:[256,256]}, [
    {...actor,image:npcTexture,position:[320,336]}, actor,
  ], foregroundTexture);
  assert.deepEqual(spriteCalls, [
    ['draw',image,256,256,256,224,0,0,256,224],
    ['save'], ['translate',64,80], ['draw',npcTexture,16,32,24,32,-10,-28,24,32], ['restore'],
    ['save'], ['translate',136,97], ['draw',texture,16,32,24,32,-10,-28,24,32], ['restore'],
    ['draw',foregroundTexture,256,256,256,224,0,0,256,224],
  ]);

  // Real asset adapter: pre-mirrored bounds, transparent gaps, and high-only occlusion.
  const rgba = [10,20,30,255, 40,50,60,255, 70,80,90,255, 100,110,120,255];
  const bundle = {schema_version:1,scene_ids:{15:['ark']}, frames:{'2:1':{width:2,height:2,offset:[-9,-31],rgba}},
    foreground:{'15':{width:2,height:2,runs:[1,1,3,1]}}};
  const rasterCalls = [];
  const art = prepareArt(bundle, {width:2,height:2,data:rgba}, (width,height,data) => {
    const result = {width,height,data:Array.from(data)}; rasterCalls.push(result); return result;
  });
  assert.deepEqual(rasterCalls[1].data, [0,0,0,0,40,50,60,255,0,0,0,0,100,110,120,255]);
  const selected = selectActor(art, {...initial(),actor_key:'2:1'});
  assert.deepEqual(Array.from(selected.offset), [-9,-31]);
  assert.equal(selected.flipX, false, 'native alternate mirror anchors are already composed');
  assert.throws(() => selectActor(art, {...initial(),actor_key:'unknown'}), /sprite/i);
  assert.throws(() => prepareArt({...bundle,schema_version:99}, {width:2,height:2,data:rgba}, () => {}), /art/i);
  for (const ids of [undefined, [], ['other'], ['ark','ark'], ['ark',7], ['ark',''],
    ['ark',...Array.from({length:128},(_,i)=>`resident-${i}`)]]) {
    assert.throws(()=>prepareArt({...bundle,scene_ids:{15:ids}}, {width:2,height:2,data:rgba},()=>({})), /membership/i);
  }
  spriteCalls.length = 0;
  drawScene(spriteCtx, image, initial(), selected, art.foreground['15']);
  assert.equal(spriteCalls.at(-1)[1], art.foreground['15'], 'opaque high background is drawn after Ark');
  const sceneArt = {...art,sceneIds:{15:['ark','resident-a']},actors:{...art.actors,'npc:house':selected}};
  const sceneState = {...initial(),actor_key:'2:1',scene:[
    {id:'resident-a',key:'npc:house',position:[320,336]}, {id:'ark',key:'2:1',position:[472,176]},
  ]};
  const entries = selectActors(sceneArt,sceneState);
  assert.deepEqual(Array.from(entries[0].position), [320,336]);
  assert.deepEqual(Array.from(entries[1].position), [472,176]);
  for(const scene of [[],null,[{id:'ark',key:'2:1',position:[0,0]}],
    [{id:'ark',key:'2:1',position:[472,176]}, {id:'resident-a',key:'npc:house',position:[-1,336]}],
    [{id:'ark',key:'2:1',position:[472,176]}, {id:'resident-a',key:'npc:house',position:[320,65536]}],
    [{id:'ark',key:'2:1',position:[472,176]}, {id:'resident-a',key:'npc:house',position:[320]}],
    [sceneState.scene[0],sceneState.scene[1],sceneState.scene[0]],
    [{id:'ark',key:'missing',position:[472,176]}],
    [{id:'ark',key:'2:1',position:[472,176]},{id:'ark',key:'2:1',position:[472,176]}]]) {
    assert.throws(()=>selectActors(sceneArt,{...sceneState,scene}),/scene|sprite/i);
  }
  // NPC instances can share art; membership is keyed by stable actor identity.
  const sharedSceneArt = {...sceneArt,sceneIds:{15:['ark','resident-a','resident-b']}};
  const sharedScene = {...sceneState,scene:[
    {id:'resident-a',key:'npc:house',position:[320,336]},
    {id:'resident-b',key:'npc:house',position:[440,416]},
    {id:'ark',key:'2:1',position:[472,176]},
  ]};
  const shared = selectActors(sharedSceneArt,sharedScene);
  assert.equal(shared.length,3);
  assert.equal(shared[0].image,shared[1].image,'shared raster is not duplicate actor identity');
  assert.deepEqual(Array.from(shared[1].position),[440,416]);
  for (const scene of [sharedScene.scene.slice(1),
    sharedScene.scene.map((entry,i)=>({...entry,id:i===1?'resident-a':entry.id})),
    sharedScene.scene.map((entry,i)=>({...entry,id:i===1?'unknown':entry.id}))]) {
    assert.throws(()=>selectActors(sharedSceneArt,{...sharedScene,scene}),/scene/i);
  }
  assert(!html.includes('Cyan box ='));

  assert(html.includes('Experimental semantic preview — reference-qualified walking; doorway timing simplified; no original CPU'));
  assert(/id="new-game"[^>]*>New Game<\/button>/.test(html));
  assert(html.includes("$('new-game').addEventListener('click', () => controller.newGame())"));
  assert(html.includes('Checkpoint Reset')); assert(html.includes('Checkpoint doorway demo'));
  assert(html.includes('default name')); assert(html.includes('intro'));
  assert(html.includes('/map.bmp')); assert(!/https?:\/\/|<script[^>]+src=/i.test(html));
  console.log('PASS: New Game, serialized start precedence, paused input cleanup, checkpoint reset/demo, pacing, bindings, errors, sprite anchors/mirroring, and drawing');
}
main().catch(error => { console.error(error); process.exitCode = 1; });

// An open door replaces every background pixel AND clears the old foreground.
{
  const width=48,height=48,data=new Uint8ClampedArray(width*height*4).fill(255);
  const patches=[[8,8],[24,24]].map((position,i)=>({position,rgba:Array(256).fill([3+i,4,5,255]).flat(),high:Array(256).fill(false)}));
  patches[1].high[0]=true;
  const bundle={schema_version:1,frames:{},scene_ids:{11:['ark'],12:['ark']},
    foreground:{11:{width,height,runs:[0,width*height]},12:{width,height,runs:[0,width*height]}},door_patches:patches};
  const make=(width,height,rgba)=>({width,height,rgba:Array.from(rgba)});
  const background={width,height,data},art=prepareArt(bundle,background,make),image={};
  for(const map_id of [12,11]) {
    const closed=selectBackground(art,{map_id,wooden_door_open:false},image);
    assert.equal(closed.image,image);
    const open=selectBackground(art,{map_id,wooden_door_open:true},image);
    for(let y=0;y<height;y++) for(let x=0;x<width;x++) {
      const patch=patches.find(p=>x>=p.position[0]&&x<p.position[0]+16&&y>=p.position[1]&&y<p.position[1]+16);
      const i=patch?(y-patch.position[1])*16+x-patch.position[0]:0,at=(y*width+x)*4;
      const pixel=patch?patch.rgba.slice(i*4,i*4+4):[255,255,255,255];
      assert.deepEqual(open.image.rgba.slice(at,at+4),pixel);
      assert.deepEqual(open.foreground.rgba.slice(at,at+4),patch&&!patch.high[i]?[0,0,0,0]:pixel);
      assert.deepEqual(closed.foreground.rgba.slice(at,at+4),[255,255,255,255]);
    }
    assert.equal(selectBackground(art,{map_id,wooden_door_open:false},image).foreground,closed.foreground,'reset selects closed foreground');
  }
  assert.throws(()=>selectBackground(art,{map_id:12,wooden_door_open:1},image),/door/i);
  for(const mutate of [p=>p.pop(),p=>p[0].position=[-1,0],p=>p[1].position=[8,8],p=>p[0].rgba.pop(),p=>p[0].high[0]=1]) {
    const bad=JSON.parse(JSON.stringify(patches));mutate(bad);
    assert.throws(()=>prepareArt({...bundle,door_patches:bad},background,make),/door/i);
  }
  const legacy=prepareArt({...bundle,door_patches:undefined},background,make);
  assert.throws(()=>selectBackground(legacy,{map_id:12,wooden_door_open:true},image),/door/i);
}

// Dialogue pages are immutable source rasters; missing keys must fail visibly.
{
  const rgba=[255,255,255,255,0,0,0,0];
  const bundle={schema_version:1,frames:{},scene_ids:{15:['ark']},foreground:{15:{width:2,height:1,runs:[]}},
    dialogue_pages:{'page:1':{width:2,height:1,rgba}}};
  const background={width:2,height:1,data:rgba},make=(width,height,rgba)=>({width,height,rgba:Array.from(rgba)});
  const art=prepareArt(bundle,background,make);
  assert.equal(selectDialogue(art,{}),null);
  assert.equal(selectDialogue(art,{dialogue:null}),null);
  const page=selectDialogue(art,{dialogue:{key:'page:1'}});
  assert.equal(page.width,2);assert.equal(page.height,1);assert.deepEqual(page.image.rgba,rgba);
  assert.throws(()=>selectDialogue(art,{dialogue:{key:'missing'}}),/dialogue/i);
  for(const change of [{width:0},{width:513},{height:225},{rgba:[1]},{rgba:[255,255,255,255,0,0,0,256]}]) {
    assert.throws(()=>prepareArt({...bundle,dialogue_pages:{'page:1':{...bundle.dialogue_pages['page:1'],...change}}},background,make),/dialogue/i);
  }
}

// Choice labels reuse immutable dialogue rasters without an extra page wait.
{
  const frame={width:1,height:1,rgba:[255,255,255,255]};
  const bundle={schema_version:1,frames:{},scene_ids:{15:['ark']},foreground:{15:{width:1,height:1,runs:[]}},
    dialogue_pages:{prompt:frame,one:frame,two:frame},choice_catalogs:{0:['one','two']}};
  const background={width:1,height:1,data:frame.rgba},make=(width,height,rgba)=>({width,height,rgba:Array.from(rgba)});
  const art=prepareArt(bundle,background,make);
  assert.equal(selectChoices(art,{dialogue:{key:'prompt'}}),null);
  assert.equal(selectChoices(art,{dialogue:{key:'prompt',choice:0}}).length,2);
  assert.throws(()=>selectChoices(art,{dialogue:{key:'prompt',choice:1}}),/choice/i);
  for(const options of [[],['one'],['one','missing'],['one','two','one']]) {
    assert.throws(()=>prepareArt({...bundle,choice_catalogs:{0:options}},background,make),/choice/i);
  }
}

// The same native choice catalog may label different source prompt responses.
{
  const frame={width:1,height:1,rgba:[255,255,255,255]};
  const bundle={schema_version:1,frames:{},scene_ids:{15:['ark']},foreground:{15:{width:1,height:1,runs:[]}},
    dialogue_pages:{promptA:frame,promptB:frame,one:frame,two:frame,other:{...frame,rgba:[255,0,0,255]}},choice_catalogs:{0:['one','two']},
    dialogue_choice_contexts:{promptA:{catalog:0,options:['one','two']},promptB:{catalog:0,options:['other','one']}}};
  const background={width:1,height:1,data:frame.rgba},make=(width,height,rgba)=>({width,height,rgba:Array.from(rgba)});
  const art=prepareArt(bundle,background,make);
  assert.deepEqual(Array.from(selectChoices(art,{dialogue:{key:'promptB',choice:0}}),o=>o.key),['other','one']);
  assert.deepEqual(selectChoices(art,{dialogue:{key:'promptB',choice:0}})[0].image.rgba,[255,0,0,255]);
  assert.deepEqual(Array.from(selectChoices(art,{dialogue:{key:'promptA',choice:0}}),o=>o.key),['one','two']);
  assert.equal(selectChoices(art,{dialogue:{key:'promptA'}}),null);
  assert.throws(()=>selectChoices(art,{dialogue:{key:'promptB',choice:1}}),/choice/i);
  assert.throws(()=>selectChoices(art,{dialogue:{key:'one',choice:0}}),/choice/i); // No global fallback when contexts are declared.
  for(const contexts of [null,[],42,{missing:{catalog:0,options:['one','two']}},
      {promptA:{catalog:'0',options:['one','two']}},{promptA:{catalog:65536,options:['one','two']}},
      {promptA:{catalog:0,options:['one','missing']}},{promptA:{catalog:0,options:['one']}},
      {promptA:null}]) {
    assert.throws(()=>prepareArt({...bundle,dialogue_choice_contexts:contexts},background,make),/choice/i);
  }
}

// Different palettes/strides, shared indoor identity, and a persistent door flag on A.
{
  const backgrounds={house:{url:'/map.bmp',width:512,height:1024},exterior:{url:'/exterior.bmp',width:1024,height:1280}};
  const background_keys={10:'exterior',11:'house',12:'house',13:'house',15:'house',16:'house',17:'house'};
  const pixels=Object.fromEntries(Object.entries(backgrounds).map(([key,{width,height}],i)=>
    [key,{width,height,data:new Uint8ClampedArray(width*height*4).fill(i+20)}]));
  const patches=[[8,8],[24,24]].map(position=>({position,rgba:Array(1024).fill(99),high:Array(256).fill(false)}));
  patches[1].high[0]=true;
  const bundle={schema_version:1,backgrounds,background_keys,door_background:'house',door_patches:patches,frames:{},
    scene_ids:Object.fromEntries(Object.keys(background_keys).map(id=>[id,['ark']])),
    foreground:Object.fromEntries(Object.entries(background_keys).map(([id,key])=>[id,{width:backgrounds[key].width,height:backgrounds[key].height,runs:[0,1,8200,1]}]))};
  let copies=0;
  const make=(width,height,data)=>{copies++;return {width,height,data:new Uint8ClampedArray(data)};};
  const art=prepareArt(bundle,pixels,make);
  const closed=selectBackground(art,{map_id:12}),open=selectBackground(art,{map_id:12,wooden_door_open:true});
  const exterior=selectBackground(art,{map_id:10,wooden_door_open:true});
  assert.notEqual(exterior.image,closed.image);
  assert.equal(exterior.image.width,1024);assert.equal(exterior.image.height,1280);
  assert.equal(closed.image.width,512);assert.equal(closed.image.height,1024);
  assert.equal(exterior.foreground.data[8200*4],21,'exterior mask uses its own palette and stride');
  assert.equal(closed.foreground.data[8200*4],20);
  assert.equal(exterior.foreground.data[4],0,'low pixels stay transparent');
  const at=(8*512+8)*4;
  assert.equal(open.image.data[at],99);assert.equal(open.foreground.data[at],0);
  assert.equal(open.foreground.data[(24*512+24)*4],99);
  assert.equal(closed.image.data[at],20);assert.equal(pixels.house.data[at],20,'source remains closed');
  const prepared=copies;
  for(let i=0;i<20;i++) {
    assert.equal(selectBackground(art,{map_id:11}).image,closed.image);
    assert.equal(selectBackground(art,{map_id:11,wooden_door_open:true}).image,open.image);
    assert.equal(selectBackground(art,{map_id:10,wooden_door_open:!!(i%2)}).image,exterior.image);
    assert.equal(selectBackground(art,{map_id:10,wooden_door_open:!!(i%2)}).foreground,exterior.foreground);
  }
  assert.equal(copies,prepared,'selection never prepares/copies rasters');
  const draws=[],ctx={fillRect(){},drawImage(...args){draws.push(args);},strokeRect(){}};
  drawScene(ctx,exterior.image,{...initial(),map_id:10,camera:[376,640]});
  assert.deepEqual(draws[0],[exterior.image,376,640,256,224,0,0,256,224]);
  assert.throws(()=>selectBackground(art,{map_id:99}),/background/i);
  for(const change of [
    {backgrounds:{...backgrounds,exterior:{...backgrounds.exterior,url:'/other.bmp'}}},
    {backgrounds:{...backgrounds,exterior:{...backgrounds.exterior,width:512}}},
    {background_keys:{...background_keys,10:'house'}},
    {background_keys:{...background_keys,10:'missing'}},
    {background_keys:{11:'house'}},
    {door_background:'exterior'},
    {foreground:{...bundle.foreground,10:{width:512,height:1024,runs:[]}}},
  ]) assert.throws(()=>prepareArt({...bundle,...change},pixels,make),/background|foreground/i);
  assert.throws(()=>prepareArt(bundle,{house:pixels.house},make),/background/i);
  assert.throws(()=>prepareArt(bundle,{...pixels,exterior:{...pixels.exterior,data:[]}},make),/background/i);
  console.log('PASS: two-sheet preparation, identity, masks, door isolation, bounded manifest, fixed viewport');
}
