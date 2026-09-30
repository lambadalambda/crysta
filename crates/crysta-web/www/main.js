// The page: ROM selection, the frame loop, input and sound.
import init, { WebGame } from './crysta_web.js';

const RATE = 32000;
const AHEAD = 0.1; // Seconds of sound queued.
const BUTTONS = { UP: 1, DOWN: 2, LEFT: 4, RIGHT: 8, CONFIRM: 16, CANCEL: 32, DESCRIBE: 64 };
const DIRECTIONS = 15;
const ACTIONS = BUTTONS.CONFIRM | BUTTONS.CANCEL | BUTTONS.DESCRIBE;
const KEYS = {
  ArrowUp: 'UP', KeyW: 'UP', ArrowDown: 'DOWN', KeyS: 'DOWN',
  ArrowLeft: 'LEFT', KeyA: 'LEFT', ArrowRight: 'RIGHT', KeyD: 'RIGHT',
  KeyX: 'CONFIRM', Enter: 'CONFIRM', Space: 'CONFIRM',
  KeyZ: 'CANCEL', Backspace: 'CANCEL',
  KeyQ: 'DESCRIBE',
};

const status = document.getElementById('status');
const start = document.getElementById('start');
const canvas = document.getElementById('view');
const wide = document.getElementById('wide');
const saves = document.getElementById('saves');
const exportSave = document.getElementById('export');
const importSave = document.getElementById('import');
const context = canvas.getContext('2d');
let game = null;
let running = null; // The running loop's token; a new game ends the old loop.
let audio = null;
let muted = false;

function say(text, error = false) {
  status.className = error ? 'error' : '';
  status.textContent = text;
}

// Keys held by code, newest last: the newest direction wins, as in the
// native app. Action presses are latched until a frame takes them, so a
// tap between two frames still counts, even if the prior frame held the key.
const keys = [];
let latched = 0;
let lastPadActions = 0;
let padLatched = 0;
let blockedPadActions = 0;
let focused = true;
const inputPaused = () => !focused || document.hidden;

function suspendInput() {
  keys.length = 0;
  latched = 0;
  padLatched = 0;
  lastPadActions = 0;
  // A button held across blur/hide must be released before it is a new press.
  blockedPadActions = ACTIONS;
}

addEventListener('keydown', (event) => {
  if (!running || inputPaused()) return;
  if (event.code === 'KeyM' && !event.repeat) toggleSound();
  if (event.code === 'KeyV' && !event.repeat) {
    wide.checked = !wide.checked;
    applyView();
  }
  const name = KEYS[event.code];
  if (!name) return;
  event.preventDefault();
  if (!event.repeat && !keys.includes(event.code)) {
    keys.push(event.code);
    latched |= BUTTONS[name] & ~DIRECTIONS;
  }
});
addEventListener('keyup', (event) => {
  const index = keys.indexOf(event.code);
  if (index >= 0) keys.splice(index, 1);
});
addEventListener('blur', () => { focused = false; suspendInput(); });
addEventListener('focus', () => { focused = true; });

function keyboard() {
  let bits = 0;
  for (const code of keys) {
    const bit = BUTTONS[KEYS[code]];
    bits = bit & DIRECTIONS ? (bits & ~DIRECTIONS) | bit : bits | bit;
  }
  return bits;
}

// Standard-mapped pads, as the native app reads them: the D-pad, else the
// stick's stronger axis; A (South) confirms, B (East) cancels, LB
// describes a shop's item.
function pad() {
  let bits = 0;
  for (const gamepad of navigator.getGamepads?.() ?? []) {
    if (!gamepad || gamepad.mapping !== 'standard') continue;
    const button = (index) => gamepad.buttons[index]?.pressed;
    const [x = 0, y = 0] = gamepad.axes;
    if (button(12)) bits |= BUTTONS.UP;
    else if (button(13)) bits |= BUTTONS.DOWN;
    else if (button(14)) bits |= BUTTONS.LEFT;
    else if (button(15)) bits |= BUTTONS.RIGHT;
    else if (Math.max(Math.abs(x), Math.abs(y)) > 0.5) {
      if (Math.abs(x) > Math.abs(y)) bits |= x < 0 ? BUTTONS.LEFT : BUTTONS.RIGHT;
      else bits |= y < 0 ? BUTTONS.UP : BUTTONS.DOWN;
    }
    if (button(0)) bits |= BUTTONS.CONFIRM;
    if (button(1)) bits |= BUTTONS.CANCEL;
    if (button(4)) bits |= BUTTONS.DESCRIBE;
  }
  return bits;
}

// The pad wins over the keyboard, as in the native app.
function buttons() {
  if (inputPaused()) return { held: 0, padActions: 0 };
  const keys = keyboard();
  const gamepad = pad();
  const rawActions = gamepad & ACTIONS;
  blockedPadActions &= rawActions;
  const padActions = rawActions & ~blockedPadActions;
  const direction = (gamepad & DIRECTIONS) || (keys & DIRECTIONS);
  return {
    held: direction | (keys & ACTIONS) | padActions,
    padActions,
  };
}

// Saves: the SRAM stays in this browser, one for each ROM revision, as the
// bytes of a native `.srm` (base64). It never leaves the page.
const sramKey = () => `crysta-sram:${game.revision()}`;
function storage() {
  try { return globalThis.localStorage ?? null; } catch { return null; }
}
function toBase64(bytes) {
  let text = '';
  for (const byte of bytes) text += String.fromCharCode(byte);
  return btoa(text);
}
function fromBase64(text) {
  return Uint8Array.from(atob(text), (char) => char.charCodeAt(0));
}
// A kept save that does not load is never written over.
let keepable = true;
function restoreSram() {
  keepable = true;
  const kept = storage()?.getItem(sramKey());
  if (!kept) return null;
  try {
    game.load_sram(fromBase64(kept));
    return null;
  } catch (error) {
    keepable = false;
    return `The save kept in this browser does not load, and is left as it is: ${error.message ?? error}`;
  }
}
function keepSram() {
  if (!keepable) return false;
  try {
    storage()?.setItem(sramKey(), toBase64(game.sram()));
    return true;
  } catch (error) {
    say(`The save is not kept in this browser: ${error.message ?? error}`, true);
    return false;
  }
}

function stop() {
  running = null;
  game?.free();
  game = null;
}

document.getElementById('rom').addEventListener('change', async (event) => {
  const file = event.target.files[0];
  if (!file) return;
  stop();
  start.classList.add('hidden');
  say('Checking the ROM…');
  try {
    await init();
    game = new WebGame(new Uint8Array(await file.arrayBuffer()));
    const problem = restoreSram();
    saves.classList.remove('hidden');
    applyView();
    say(problem ?? 'Ready.', !!problem);
    start.classList.remove('hidden');
  } catch (error) {
    say(`This ROM does not work: ${error.message ?? error}`, true);
  }
});

start.addEventListener('click', async () => {
  start.classList.add('hidden');
  if (keepable) say(''); // A kept save that did not load stays reported.
  // One context for the page; a new game starts its queue afresh.
  try {
    audio ??= new AudioContext({ sampleRate: RATE });
    game.start_audio();
    muted = false;
    await audio.resume();
  } catch (error) {
    say(`No sound: ${error.message ?? error}`, true);
  }
  queued = 0;
  last = null;
  owed = 0;
  keys.length = 0;
  latched = 0;
  lastPadActions = 0;
  padLatched = 0;
  blockedPadActions = 0;
  canvas.focus();
  const token = {};
  running = token;
  requestAnimationFrame((now) => loop(now, token));
});

// A native `.srm` in and out: the file stays on this computer.
exportSave.addEventListener('click', () => {
  if (!game) return;
  const link = document.createElement('a');
  link.href = URL.createObjectURL(new Blob([game.sram()], { type: 'application/octet-stream' }));
  link.download = `terranigma-${game.revision()}.srm`;
  link.click();
  setTimeout(() => URL.revokeObjectURL(link.href), 1000);
});
importSave.addEventListener('change', async (event) => {
  const file = event.target.files[0];
  event.target.value = '';
  if (!file || !game) return;
  const kept = storage()?.getItem(sramKey());
  if (kept && !confirm('Replace the save kept in this browser? The old one stays as a backup.')) return;
  try {
    game.load_sram(new Uint8Array(await file.arrayBuffer()));
  } catch (error) {
    say(`This save does not load: ${error.message ?? error}`, true);
    return;
  }
  try {
    if (kept) storage().setItem(`${sramKey()}:previous`, kept);
  } catch { /* The backup is best effort; the import goes on. */ }
  keepable = true;
  if (keepSram()) say('Save imported.');
});

// The canvas follows the view: 256 or 400 pixels across, three times over
// at most.
function applyView() {
  game?.set_wide(wide.checked);
  canvas.width = game?.width() ?? (wide.checked ? 400 : 256);
  canvas.height = 224;
  canvas.style.maxWidth = `${canvas.width * 3}px`;
  canvas.style.aspectRatio = `${canvas.width} / ${canvas.height}`;
}
wide.addEventListener('change', applyView);

function toggleSound() {
  if (!audio) return;
  muted = !muted;
  if (muted) audio.suspend(); else audio.resume();
}

let last = null;
let owed = 0;
let queued = 0;
function loop(now, token) {
  if (running !== token) return;
  try {
    if (last === null) last = now;
    // The console's rate: 60.1 Hz for the Japanese ROM, 50 Hz for the European.
    const frameMs = game.frame_ms();
    owed = Math.min(owed + (now - last), frameMs * 4);
    last = now;
    const { held, padActions } = buttons();
    // Poll every render callback, including those without a PAL game frame.
    // Keep a sampled gamepad re-press until the next game frame consumes it.
    padLatched |= padActions & ~lastPadActions;
    lastPadActions = padActions;
    while (owed >= frameMs) {
      game.frame_with_presses(held, latched | padLatched);
      if (game.take_sram_write()) keepSram();
      latched = 0;
      padLatched = 0;
      owed -= frameMs;
    }
    const fault = game.fault();
    if (fault) say(`The game stopped: ${fault}`, true);
    const image = new ImageData(new Uint8ClampedArray(game.draw()), game.width(), WebGame.height());
    context.putImageData(image, 0, 0);
    if (audio && !muted) queueSound();
  } catch (error) {
    say(`The page stopped: ${error.message ?? error}`, true);
    running = null;
    return;
  }
  requestAnimationFrame((next) => loop(next, token));
}

function queueSound() {
  const frames = 1024;
  queued = Math.max(queued, audio.currentTime);
  while (queued - audio.currentTime < AHEAD) {
    const samples = new Float32Array(frames * 2);
    game.audio(samples);
    const buffer = audio.createBuffer(2, frames, RATE);
    const left = buffer.getChannelData(0);
    const right = buffer.getChannelData(1);
    for (let i = 0; i < frames; i++) {
      left[i] = samples[2 * i];
      right[i] = samples[2 * i + 1];
    }
    const source = audio.createBufferSource();
    source.buffer = buffer;
    source.connect(audio.destination);
    source.start(queued);
    queued += frames / RATE;
  }
}

document.addEventListener('visibilitychange', () => {
  last = null;
  if (document.hidden) suspendInput();
});
