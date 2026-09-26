// The page's real RAF/input wiring, with only the DOM, audio and WebGame mocked.
// Run: node --test crates/crysta-web/input.test.mjs
import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';

const source = readFileSync(new URL('./www/main.js', import.meta.url), 'utf8')
  .replace(/^import init, \{ WebGame \} from '.\/crysta_web\.js';/m, '');

async function page() {
  const elements = new Map();
  const listeners = new Map();
  const frames = [];
  const callbacks = [];
  const pad = { mapping: 'standard', buttons: Array.from({ length: 16 }, () => ({ pressed: false })), axes: [0, 0] };
  const element = (id) => {
    if (!elements.has(id)) elements.set(id, {
      checked: false, classList: { add() {}, remove() {} },
      addEventListener(name, fn) { listeners.set(`${id}:${name}`, fn); },
      focus() {}, getContext: () => ({ putImageData() {} }),
    });
    return elements.get(id);
  };
  const document = {
    hidden: false,
    getElementById: element,
    addEventListener(name, fn) { listeners.set(`document:${name}`, fn); },
  };
  class WebGame {
    constructor() { this.frames = frames; }
    static height() { return 224; }
    frame_ms() { return 20; }
    width() { return 256; }
    set_wide() {}
    start_audio() {}
    frame_with_presses(held, pressed) { frames.push({ held, pressed }); }
    fault() { return null; }
    draw() { return new Uint8Array(256 * 224 * 4); }
  }
  const context = vm.createContext({
    document, WebGame, init: async () => {},
    navigator: { getGamepads: () => [pad] },
    addEventListener(name, fn) { listeners.set(name, fn); },
    requestAnimationFrame(fn) { callbacks.push(fn); },
    AudioContext: class { async resume() {} },
    ImageData: class {}, Uint8Array, Uint8ClampedArray,
  });
  vm.runInContext(`${source}\nqueueSound = () => {};`, context);
  await listeners.get('rom:change')({ target: { files: [{ arrayBuffer: async () => new ArrayBuffer(1) }] } });
  await listeners.get('start:click')();
  const raf = (time) => { assert.ok(callbacks.length); callbacks.shift()(time); };
  const key = (name, code) => listeners.get(name)({ code, preventDefault() {} });
  return { frames, pad, raf, key, document, fire: (name) => listeners.get(name)(),
    last: () => frames.at(-1) };
}

const CONFIRM = 16;

test('X held and sampled gamepad A each generate separate edges, including a sub-frame re-press', async () => {
  const p = await page();
  p.raf(0);
  p.key('keydown', 'KeyX');
  p.raf(20);
  assert.deepEqual(p.last(), { held: CONFIRM, pressed: CONFIRM });
  p.pad.buttons[0].pressed = true;
  p.raf(32); // no PAL frame yet: the observed edge must wait
  assert.equal(p.frames.length, 1);
  p.raf(40);
  assert.deepEqual(p.last(), { held: CONFIRM, pressed: CONFIRM });
  p.pad.buttons[0].pressed = false;
  p.raf(52);
  p.pad.buttons[0].pressed = true;
  p.raf(60);
  assert.deepEqual(p.last(), { held: CONFIRM, pressed: CONFIRM });
  p.raf(80);
  assert.deepEqual(p.last(), { held: CONFIRM, pressed: 0 });
  p.key('keyup', 'KeyX');
  p.raf(100);
  p.key('keydown', 'KeyX');
  p.raf(120);
  assert.deepEqual(p.last(), { held: CONFIRM, pressed: CONFIRM }, 'keyboard is independent too');
});

test('blur or hidden page discards unsampled gamepad and keyboard presses until fresh input', async () => {
  for (const event of ['blur', 'visibilitychange']) {
    const p = await page();
    p.raf(0);
    p.pad.buttons[0].pressed = true;
    p.key('keydown', 'KeyX');
    p.raf(8); // A and X sampled, but neither has reached a PAL game frame
    if (event === 'visibilitychange') p.document.hidden = true;
    p.fire(event === 'blur' ? 'blur' : 'document:visibilitychange');
    p.key('keydown', 'KeyX');
    p.raf(20);
    if (event === 'visibilitychange') p.raf(40); // visibility reset the PAL clock
    assert.deepEqual(p.last(), { held: 0, pressed: 0 }, `${event}: neutral while inactive`);
    if (event === 'visibilitychange') {
      p.document.hidden = false;
      p.fire('document:visibilitychange');
      p.raf(60); // rebase the clock again on return
    } else p.fire('focus');
    const offset = event === 'visibilitychange' ? 40 : 0;
    p.raf(40 + offset);
    assert.deepEqual(p.last(), { held: 0, pressed: 0 }, `${event}: no ghost on return`);
    p.pad.buttons[0].pressed = false;
    p.raf(52 + offset); // observed release re-arms A
    p.pad.buttons[0].pressed = true;
    p.raf(60 + offset);
    if (event === 'visibilitychange') p.raf(72 + offset);
    assert.deepEqual(p.last(), { held: CONFIRM, pressed: CONFIRM }, `${event}: new press works`);
  }
});
