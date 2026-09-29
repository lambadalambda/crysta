#!/usr/bin/env node
// Replays a crysta-web route trace through a real page in headless Chromium:
//   node tools/web-replay/replay.mjs --url URL --rom ROM --trace TRACE [--shots DIR] [--chrome PATH]
// The ROM goes to the page's file input from disk; the page never uploads it.
// Screenshots (--shots) show copyrighted art: they must stay under local/.
import { spawn } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync, existsSync } from 'node:fs';
import { homedir, tmpdir } from 'node:os';
import { join, resolve, sep } from 'node:path';
import { parseArgs } from 'node:util';
import { KEYS, parseTrace, steps, keyEvents, sameInput, fnv64 } from './trace.mjs';

const here = new URL('.', import.meta.url).pathname;
const repo = resolve(here, '../..');

function findChrome() {
  if (process.env.CHROME) return process.env.CHROME;
  const cache = join(homedir(), 'Library/Caches/ms-playwright');
  const shells = existsSync(cache) ? readdirSync(cache).filter((d) => d.startsWith('chromium_headless_shell-')).sort() : [];
  for (const dir of shells.reverse()) {
    for (const arch of ['mac-arm64', 'mac-x64', 'linux64']) {
      const path = join(cache, dir, `chrome-headless-shell-${arch}`, 'chrome-headless-shell');
      if (existsSync(path)) return path;
    }
  }
  throw new Error('no headless Chromium: set CHROME');
}

function launch(chrome) {
  const profile = mkdtempSync(join(tmpdir(), 'web-replay-'));
  // Chromium's own sandbox cannot start inside a sandboxed agent; the page is
  // still isolated by its origin, and this profile is thrown away.
  const child = spawn(chrome, ['--headless', '--no-sandbox', '--disable-gpu', '--mute-audio', '--no-first-run',
    `--user-data-dir=${profile}`, '--remote-debugging-port=0', 'about:blank'], { stdio: ['ignore', 'ignore', 'pipe'] });
  const close = () => { child.kill(); rmSync(profile, { recursive: true, force: true, maxRetries: 5 }); };
  const port = new Promise((ok, fail) => {
    let log = '';
    child.stderr.on('data', (chunk) => {
      log += chunk;
      const match = log.match(/DevTools listening on ws:\/\/[^:]+:(\d+)\//);
      if (match) ok(match[1]);
    });
    child.on('exit', (code) => fail(new Error(`Chromium exited (${code}): ${log.slice(-500)}`)));
  });
  return { port, close };
}

async function connect(port) {
  const pages = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
  const socket = new WebSocket(pages.find((t) => t.type === 'page').webSocketDebuggerUrl);
  await new Promise((ok, fail) => { socket.onopen = ok; socket.onerror = fail; });
  let id = 0;
  const pending = new Map();
  const listeners = [];
  socket.onmessage = ({ data }) => {
    const message = JSON.parse(data);
    const waiter = pending.get(message.id);
    if (waiter) {
      pending.delete(message.id);
      if (message.error) waiter.fail(new Error(`${waiter.method}: ${message.error.message}`));
      else waiter.ok(message.result);
    } else if (message.method) for (const listen of listeners) listen(message);
  };
  socket.onclose = () => {
    for (const { fail, method } of pending.values()) fail(new Error(`${method}: the browser went away`));
    pending.clear();
  };
  const send = (method, params = {}) => new Promise((ok, fail) => {
    pending.set(++id, { ok, fail, method });
    socket.send(JSON.stringify({ id, method, params }));
  });
  const on = (listen) => listeners.push(listen);
  return { send, on, close: () => socket.close() };
}

async function main() {
  const { values: args } = parseArgs({ options: {
    url: { type: 'string', default: 'https://lambadalambda.github.io/crysta/' },
    rom: { type: 'string', default: join(repo, 'local/Terranigma (E) [!].smc') },
    trace: { type: 'string' }, shots: { type: 'string' }, chrome: { type: 'string' },
  } });
  if (!args.trace) throw new Error('--trace is required');
  const shots = args.shots && resolve(args.shots);
  if (shots && !shots.startsWith(join(repo, 'local') + sep)) throw new Error('--shots must be under local/');
  if (shots) mkdirSync(shots, { recursive: true });
  const trace = parseTrace(readFileSync(args.trace, 'utf8'));
  const browser = launch(args.chrome ?? findChrome());
  try {
    const cdp = await connect(await browser.port);
    const errors = [];
    cdp.on(({ method, params }) => {
      if (method === 'Runtime.exceptionThrown') errors.push(params.exceptionDetails.exception?.description ?? params.exceptionDetails.text);
    });
    const evaluate = async (expression) => {
      const { result, exceptionDetails } = await cdp.send('Runtime.evaluate',
        { expression, awaitPromise: true, returnByValue: true, userGesture: true });
      if (exceptionDetails) throw new Error(exceptionDetails.exception?.description ?? exceptionDetails.text);
      return result.value;
    };
    const until = async (expression, what) => {
      for (let i = 0; i < 600; i++) {
        if (await evaluate(expression)) return;
        await new Promise((ok) => setTimeout(ok, 50));
      }
      throw new Error(`timed out: ${what} (status: ${await evaluate("document.getElementById('status')?.textContent")})`);
    };

    await cdp.send('Runtime.enable');
    await cdp.send('Page.enable');
    await cdp.send('Emulation.setFocusEmulationEnabled', { enabled: true });
    await cdp.send('Page.addScriptToEvaluateOnNewDocument', { source: readFileSync(join(here, 'page.js'), 'utf8') });
    await cdp.send('Page.navigate', { url: args.url });
    await until("document.readyState === 'complete' && !!document.getElementById('rom')", 'page load');
    await evaluate('__replay.hook()');
    const { root } = await cdp.send('DOM.getDocument');
    const { nodeId } = await cdp.send('DOM.querySelector', { nodeId: root.nodeId, selector: '#rom' });
    await cdp.send('DOM.setFileInputFiles', { nodeId, files: [resolve(args.rom)] });
    await until("document.getElementById('status').textContent === 'Ready.'", 'ROM accepted');
    await evaluate("document.getElementById('start').click()");
    await until('__replay.queue.length === 1', 'Start');
    const period = await evaluate('__replay.begin()');
    console.log(`page ${args.url}: ${period.toFixed(6)} ms frames, ${trace.frames} to replay`);

    const codes = Object.fromEntries(KEYS.map(([, code, vk]) => [code, vk]));
    const keyName = (code) => (code.startsWith('Key') ? code.slice(3).toLowerCase() : code);
    let frame = 0, pageHeld = 0, routeHeld = 0, failures = 0, checked = 0;
    for (const step of steps(trace)) {
      for (const [type, code] of keyEvents(routeHeld, step.held, step.pressed)) {
        await cdp.send('Input.dispatchKeyEvent', { type: type === 'keyDown' ? 'rawKeyDown' : 'keyUp',
          code, key: keyName(code), windowsVirtualKeyCode: codes[code], nativeVirtualKeyCode: codes[code] });
      }
      let calls;
      try {
        calls = await evaluate(`__replay.frames(${step.count})`);
      } catch (error) {
        throw new Error(`frame ${frame + 1}: ${error.message}`);
      }
      calls.forEach(([held, pressed], i) => {
        const expected = { held: step.held, pressed: i === 0 ? step.pressed : 0 };
        if (!sameInput({ held, pressed }, pageHeld, expected, i === 0 ? routeHeld : step.held)) {
          throw new Error(`frame ${frame + i + 1}: page sent ${held}/${pressed}, route ${expected.held}/${expected.pressed}`);
        }
        pageHeld = held;
      });
      routeHeld = step.held;
      frame += step.count;
      if (!step.checkpoint) continue;
      checked++;
      const { name, hash } = step.checkpoint;
      const view = await evaluate('__replay.view()');
      const seen = fnv64(Buffer.from(view.rgba, 'base64'));
      const same = seen === hash && !view.fault && !errors.length;
      failures += !same;
      console.log(`${same ? 'ok  ' : 'FAIL'} ${name} @${frame}: view ${seen.toString(16).padStart(16, '0')}` +
        `${seen === hash ? '' : ` expected ${hash.toString(16).padStart(16, '0')}`}` +
        `${view.fault ? ` fault: ${view.fault}` : ''}${errors.length ? ` errors: ${errors.join('; ')}` : ''}`);
      if (shots) {
        const clip = await evaluate(`(({ x, y, width, height }) => ({ x: x + scrollX, y: y + scrollY, width, height, scale: 1 }))(
          document.getElementById('view').getBoundingClientRect())`);
        const { data } = await cdp.send('Page.captureScreenshot', { format: 'png', clip, captureBeyondViewport: true });
        writeFileSync(join(shots, `${String(frame).padStart(6, '0')}-${name}.png`), Buffer.from(data, 'base64'));
      }
    }
    if (checked !== trace.checkpoints.length) throw new Error(`checked ${checked} of ${trace.checkpoints.length} checkpoints`);
    if (errors.length) throw new Error(`page errors: ${errors.join('; ')}`);
    console.log(`${failures ? 'FAILED' : 'passed'}: ${frame} frames, ${checked} checkpoints, ${failures} failed`);
    cdp.close();
    return failures ? 1 : 0;
  } finally {
    browser.close();
  }
}

main().then((code) => process.exit(code), (error) => {
  console.error(`FAILED: ${error.message}`);
  process.exit(2);
});
