// Headless check of a built site: node crates/crysta-web/smoke.mjs <site> <rom>.
// Runs frames and renders sound into memory; nothing is played.
import { readFileSync } from 'node:fs';

const site = new URL(`file://${process.argv[2].startsWith('/') ? '' : process.cwd() + '/'}${process.argv[2]}/`);
const { initSync, WebGame } = await import(new URL('crysta_web.js', site));
initSync({ module: readFileSync(new URL('crysta_web_bg.wasm', site)) });
const rom = readFileSync(process.argv[3]);
let t = performance.now();
const game = new WebGame(new Uint8Array(rom));
console.log('new', (performance.now() - t).toFixed(0), 'ms', game.width(), WebGame.height());
t = performance.now(); game.start_audio(); console.log('audio boot', (performance.now() - t).toFixed(0), 'ms');
t = performance.now();
let px;
for (let i = 0; i < 600; i++) { game.frame(0); px = game.draw(); }
console.log('600 frames', (performance.now() - t).toFixed(0), 'ms', px.length, 'fault', game.fault());
const buf = new Float32Array(1066);
t = performance.now();
let loud = 0;
for (let i = 0; i < 600; i++) { game.audio(buf); for (const s of buf) loud = Math.max(loud, Math.abs(s)); }
console.log('10 s audio', (performance.now() - t).toFixed(0), 'ms, peak', loud.toFixed(3));
let lit = 0; for (let i = 0; i < px.length; i += 4) if (px[i] | px[i+1] | px[i+2]) lit++;
console.log('lit pixels', lit);
if (game.fault() || lit === 0 || loud === 0) process.exit(1);
