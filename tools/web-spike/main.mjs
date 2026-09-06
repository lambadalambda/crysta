import init, { probe, replay } from './web_spike.js';
import { runProbe } from './run.mjs';

const fileInput = document.querySelector('#rom');
const output = document.querySelector('#result');

try {
  const start = performance.now();
  const wasm = await init();
  const response = await fetch('./native.json', { cache: 'no-store' });
  if (!response.ok) throw new Error('Generate native.json with the native harness first');
  const expected = await response.json();
  const startupMs = performance.now() - start;
  if (JSON.stringify(JSON.parse(replay())) !== JSON.stringify(expected.replay)) {
    throw new Error('Startup synthetic replay differs from native');
  }
  output.textContent = 'Ready. Native/Wasm synthetic replay matches. Select a local JP ROM.';
  fileInput.disabled = false;
  fileInput.addEventListener('change', async () => {
    const file = fileInput.files[0];
    if (!file) return;
    fileInput.disabled = true;
    output.textContent = 'Reading local file…';
    try {
      const evidence = await runProbe({
        file, probe, expected,
        memoryBytes: () => wasm.memory.buffer.byteLength,
        now: () => performance.now(),
        jsHeapBytes: () => performance.memory?.usedJSHeapSize ?? null,
      });
      output.textContent = JSON.stringify({
        status: 'pass', startup_ms: startupMs,
        user_agent: navigator.userAgent,
        cross_origin_isolated: crossOriginIsolated,
        ...evidence,
      }, null, 2);
    } catch (error) {
      output.textContent = JSON.stringify({ status: 'fail', error: String(error) });
    } finally {
      // Drop the File reference from the control and allow reselecting the same file.
      fileInput.value = '';
      fileInput.disabled = false;
    }
  });
} catch (error) {
  output.textContent = `Startup failed: ${error}`;
}
