// Pure parts of the web replay: the Rust route's input trace
// (crates/crysta-web/src/european_route.rs), the page's keys, and checks.

// www/main.js BUTTONS and one KEYS code per button.
export const KEYS = [
  [0x01, 'ArrowUp', 38], [0x02, 'ArrowDown', 40], [0x04, 'ArrowLeft', 37], [0x08, 'ArrowRight', 39],
  [0x10, 'KeyX', 88], [0x20, 'KeyZ', 90], [0x40, 'KeyQ', 81],
];
const DIRECTIONS = 0x0f;
const ACTIONS = 0x70;

// "count held pressed" runs, and "# checkpoint frame name fnv64" lines.
export function parseTrace(text) {
  const runs = [];
  const checkpoints = [];
  text.split('\n').forEach((line, index) => {
    const mark = line.match(/^# checkpoint (\d+) (\S+) ([0-9a-f]{1,16})$/);
    if (mark) {
      checkpoints.push({ frame: Number(mark[1]), name: mark[2], hash: BigInt(`0x${mark[3]}`) });
      return;
    }
    if (!line.trim() || line.startsWith('#')) return;
    const run = line.trim().split(/\s+/);
    if (run.length !== 3 || !/^\d+$/.test(run[0]) || run[0] === '0') {
      throw new Error(`trace line ${index + 1}: ${line}`);
    }
    runs.push([Number(run[0]), Number(run[1]), Number(run[2])]);
  });
  const frames = runs.reduce((sum, [count]) => sum + count, 0);
  checkpoints.sort((a, b) => a.frame - b.frame);
  checkpoints.forEach(({ frame, name }, index) => {
    if (frame < 1 || frame > frames) throw new Error(`checkpoint ${name} at ${frame} is past the end (${frames})`);
    if (frame === checkpoints[index - 1]?.frame) throw new Error(`two checkpoints at frame ${frame}`);
  });
  return { runs, checkpoints, frames };
}

// Runs as steps: the first frame of a run changes keys, the rest hold
// them; a checkpoint ends a step after its frame.
export function* steps({ runs, checkpoints }) {
  let frame = 0;
  let next = 0;
  for (const [count, held, pressed] of runs) {
    let left = count;
    let first = true;
    while (left > 0) {
      const size = first && pressed ? 1 : left;
      const mark = checkpoints[next];
      const take = mark && mark.frame <= frame + size ? mark.frame - frame : size;
      const checkpoint = mark && mark.frame === frame + take ? mark : null;
      if (checkpoint) next++;
      yield { held, pressed: first ? pressed : 0, count: take, checkpoint };
      frame += take;
      left -= take;
      first = false;
    }
  }
}

// Key events that turn the keys held for `before` into `held`, with each
// extra `pressed` action as a fresh keydown between two frames.
export function keyEvents(before, held, pressed) {
  const directions = held & DIRECTIONS;
  if (directions & (directions - 1)) throw new Error(`two directions held: ${held.toString(16)}`);
  const events = [];
  for (const [bit, code] of KEYS) if (before & bit && !(held & bit)) events.push(['keyUp', code]);
  for (const [bit, code] of KEYS) if (!(before & bit) && held & bit) events.push(['keyDown', code]);
  for (const [bit, code] of KEYS) {
    if (!(pressed & bit)) continue;
    if (before & held & bit) events.push(['keyUp', code], ['keyDown', code]);
    else if (!(held & bit)) events.push(['keyDown', code], ['keyUp', code]);
  }
  return events;
}

// WebGame.frame_with_presses(held, pressed) as the page called it, against
// the route's frame: same held bits, same action edges.
export function sameInput(actual, actualBefore, expected, expectedBefore) {
  const edge = (held, before, pressed) => ((held & ~before) | pressed) & ACTIONS;
  return actual.held === expected.held &&
    edge(actual.held, actualBefore, actual.pressed) === edge(expected.held, expectedBefore, expected.pressed);
}

export function fnv64(bytes) {
  // Two 32-bit halves: BigInt per byte is slow for a 229 KB view.
  let hi = 0xcbf29ce4, lo = 0x84222325;
  for (const byte of bytes) {
    lo ^= byte;
    // (hi:lo) * 0x100000001b3 mod 2^64.
    const l0 = (lo & 0xffff) * 0x1b3, l1 = (lo >>> 16) * 0x1b3;
    const low = l0 + (l1 & 0xffff) * 0x10000;
    const carry = Math.floor(low / 0x100000000) + (l1 >>> 16);
    hi = (Math.imul(hi, 0x1b3) + (lo << 8) + carry) >>> 0;
    lo = low >>> 0;
  }
  return (BigInt(hi) << 32n) | BigInt(lo);
}

export class TimeoutError extends Error {}

// `promise`, or a TimeoutError once `ms` pass; the timer never outlives it.
export function withTimeout(promise, ms, what) {
  if (!(ms > 0)) throw new Error(`timeout for ${what} must be positive, not ${ms}`);
  let timer;
  const late = new Promise((_, fail) => {
    timer = setTimeout(() => fail(new TimeoutError(`${what} timed out after ${ms} ms`)), ms);
  });
  return Promise.race([promise, late]).finally(() => clearTimeout(timer));
}
