import { test } from 'node:test';
import assert from 'node:assert/strict';
import { parseTrace, steps, keyEvents, sameInput, fnv64 } from './trace.mjs';

const TEXT = `# count held pressed (web button bits, hex)
3 0x0 0x0
2 0x11 0x0
1 0x0 0x10
# checkpoint 4 door 00000000000000ab
# checkpoint 6 end 0000000000000001
`;

test('parses runs and named checkpoints', () => {
  const trace = parseTrace(TEXT);
  assert.deepEqual(trace.runs, [[3, 0, 0], [2, 0x11, 0], [1, 0, 0x10]]);
  assert.deepEqual(trace.checkpoints, [
    { frame: 4, name: 'door', hash: 0xabn },
    { frame: 6, name: 'end', hash: 1n },
  ]);
  assert.equal(trace.frames, 6);
});

test('rejects malformed lines and checkpoints past the end', () => {
  assert.throws(() => parseTrace('3 0x0'), /line 1/);
  assert.throws(() => parseTrace('1 0x0 0x0\n# checkpoint 2 late 00'), /past the end/);
  assert.throws(() => parseTrace('1 0x0 0x0\n# checkpoint 1 a 00\n# checkpoint 1 b 00'), /two checkpoints/);
});

test('splits runs at checkpoints; a step changes keys once, then holds', () => {
  const trace = parseTrace(TEXT);
  assert.deepEqual([...steps(trace)], [
    { held: 0, pressed: 0, count: 3, checkpoint: null },
    { held: 0x11, pressed: 0, count: 1, checkpoint: trace.checkpoints[0] },
    { held: 0x11, pressed: 0, count: 1, checkpoint: null },
    { held: 0, pressed: 0x10, count: 1, checkpoint: trace.checkpoints[1] },
  ]);
});

test('key events release, press, then re-press or tap extra presses', () => {
  assert.deepEqual(keyEvents(0x04, 0x11, 0), [
    ['keyUp', 'ArrowLeft'], ['keyDown', 'ArrowUp'], ['keyDown', 'KeyX'],
  ]);
  assert.deepEqual(keyEvents(0x10, 0x10, 0x10), [['keyUp', 'KeyX'], ['keyDown', 'KeyX']]);
  assert.deepEqual(keyEvents(0, 0, 0x20), [['keyDown', 'KeyZ'], ['keyUp', 'KeyZ']]);
  assert.throws(() => keyEvents(0, 0x03, 0), /directions/);
});

test('page input matches when held bits and action edges agree', () => {
  const expected = { held: 0x10, pressed: 0 };
  assert.ok(sameInput({ held: 0x10, pressed: 0x10 }, 0, expected, 0));
  assert.ok(sameInput({ held: 0x10, pressed: 0 }, 0, expected, 0));
  assert.ok(!sameInput({ held: 0x10, pressed: 0 }, 0x10, expected, 0), 'lost edge');
  assert.ok(!sameInput({ held: 0x11, pressed: 0 }, 0, expected, 0), 'extra direction');
});

test('fnv64 matches the Rust route', () => {
  assert.equal(fnv64(new Uint8Array()), 0xcbf29ce484222325n);
  assert.equal(fnv64(new TextEncoder().encode('a')), 0xaf63dc4c8601ec8cn);
});
