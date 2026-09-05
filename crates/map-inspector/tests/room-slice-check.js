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
const {createController, bindInputs, drawScene} = sandbox.RoomSlice;
const initial = () => ({map_id: 15, x: 472, y: 176, tick: 0, phase: 'walking', error: null,
  policy: 'semantic-preview', camera: [256, 0]});
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
async function main() {
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
  assert(html.includes('Experimental semantic preview — reference-qualified walking; doorway timing simplified; no original CPU'));
  assert(/id="new-game"[^>]*>New Game<\/button>/.test(html));
  assert(html.includes("$('new-game').addEventListener('click', () => controller.newGame())"));
  assert(html.includes('Checkpoint Reset')); assert(html.includes('Checkpoint doorway demo'));
  assert(html.includes('default name')); assert(html.includes('intro'));
  assert(html.includes('/map.bmp')); assert(!/https?:\/\/|<script[^>]+src=/i.test(html));
  console.log('PASS: New Game, serialized start precedence, paused input cleanup, checkpoint reset/demo, pacing, bindings, errors, and drawing');
}
main().catch(error => { console.error(error); process.exitCode = 1; });
