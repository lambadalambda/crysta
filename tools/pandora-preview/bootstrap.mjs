const ROM_SIZE = 4 * 1024 * 1024;
const HEADER_SIZE = 512;
const BACKGROUNDS = new Map([
  ['/map.bmp', 'house'], ['/exterior.bmp', 'exterior'],
  ['/town13.bmp', 'town13'], ['/cellars.bmp', 'cellars'],
  ['/box.bmp', 'box'], ['/tour.bmp', 'tour'],
]);

/** Create the closed endpoint/asset adapter around one replaceable Wasm session. */
export function createSessionTransport({Preview, BlobCtor}) {
  let session = null, generation = 0;
  const invalidate = () => {
    generation++;
    session?.free();
    session = null;
  };
  const current = () => {
    if (!session) throw new Error('no authenticated local ROM session');
    return session;
  };
  return {
    invalidate,
    hasSession: () => session !== null,
    replace(bytes) {
      invalidate();
      const next = new Preview(bytes);
      session = next;
    },
    async request(path, body) {
      const selected = current(), selectedGeneration = generation;
      await Promise.resolve();
      if (selectedGeneration !== generation || selected !== session) throw new Error('stale preview generation');
      let json;
      if (path === '/state' && body === undefined) json = selected.state();
      else if (path === '/new-game' && body === '') json = selected.new_game();
      else if (path === '/reset' && body === '') json = selected.reset();
      else if (path === '/step' && typeof body === 'string' && /^(?:0|[1-9]|10)$/.test(body)) json = selected.step(Number(body));
      else throw new Error('unsupported preview request');
      if (selectedGeneration !== generation || selected !== session) throw new Error('stale preview generation');
      return JSON.parse(json);
    },
    loadArt() { return new Uint8Array(current().art()); },
    loadBackground(path) {
      const key = BACKGROUNDS.get(path);
      if (!key) throw new Error('unsupported preview background URL');
      return new BlobCtor([new Uint8Array(current().background(key))], {type:'image/bmp'});
    },
  };
}

/** Create an asynchronous generation-checked adapter around a dedicated worker. */
export function createWorkerTransport({worker, BlobCtor}) {
  let nextId = 0, generation = 0, session = false, terminalError = null;
  const pending = new Map();
  const failTerminal = failure => {
    terminalError = failure instanceof Error ? failure : new Error(String(failure?.message || failure || 'preview worker failed'));
    generation++; session = false;
    for (const {reject} of pending.values()) reject(terminalError);
    pending.clear();
  };
  worker.onerror = failTerminal;
  worker.onmessageerror = failTerminal;
  worker.onmessage = ({data:{id, ok, value}}) => {
    const entry = pending.get(id);
    if (!entry) return;
    pending.delete(id);
    if (entry.generation !== generation) entry.reject(new Error('stale preview generation'));
    else if (ok) entry.resolve(value);
    else entry.reject(new Error(value));
  };
  const call = (type, value, transfer = []) => new Promise((resolve, reject) => {
    if (terminalError) { reject(terminalError); return; }
    const id = ++nextId;
    pending.set(id, {generation, resolve, reject});
    try { worker.postMessage({id, type, value}, transfer); }
    catch (error) {
      pending.delete(id);
      failTerminal(error);
      reject(terminalError);
    }
  });
  const invalidate = () => {
    generation++; session = false;
    for (const {reject} of pending.values()) reject(new Error('stale preview generation'));
    pending.clear();
    if (!terminalError) void call('dispose', null).catch(() => {});
  };
  return {
    invalidate,
    hasSession: () => session,
    async replace(bytes) {
      invalidate();
      const buffer = bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength);
      const report = await call('replace', buffer, [buffer]);
      session = true;
      return report;
    },
    request(path, body) {
      if (!session) return Promise.reject(new Error('no authenticated local ROM session'));
      return call('request', {path, body});
    },
    replayForParity(actions) {
      if (!session) return Promise.reject(new Error('no authenticated local ROM session'));
      return call('parity', actions);
    },
    async loadArt() {
      if (!session) throw new Error('no authenticated local ROM session');
      return new Uint8Array(await call('art', null));
    },
    async loadBackground(path) {
      const key = BACKGROUNDS.get(path);
      if (!key) throw new Error('unsupported preview background URL');
      if (!session) throw new Error('no authenticated local ROM session');
      return new BlobCtor([await call('background', key)], {type:'image/bmp'});
    },
    terminate() { failTerminal(new Error('preview worker terminated')); worker.terminate(); },
  };
}

/** Install local-ROM lifecycle controls and connect them to the existing controller. */
export function installLocalBootstrap({transport, controller, loadArt, invalidatePresentation, document, performance, memoryBytes}) {
  const panel = document.createElement('section');
  panel.className = 'panel';
  panel.innerHTML = '<h2>Browser-local ROM</h2><p><label>Select owned Japanese ROM <input id="local-rom" type="file" accept=".sfc,.smc,application/octet-stream"></label></p><p id="local-rom-status" role="status">Select a local ROM. Bytes stay in this browser tab.</p>';
  document.querySelector('main').prepend(panel);
  const input = panel.querySelector('#local-rom'), status = panel.querySelector('#local-rom-status');
  let initialized = false, operation = 0;
  const setStatus = (text, kind = '') => { status.textContent = text; status.dataset.kind = kind; };
  input.addEventListener('change', async () => {
    const selected = input.files?.[0];
    if (!selected) return;
    const token = ++operation;
    transport.invalidate();
    invalidatePresentation('Local ROM replacement — input paused.');
    input.disabled = true;
    setStatus('Reading local file…');
    try {
      if (selected.size !== ROM_SIZE && selected.size !== ROM_SIZE + HEADER_SIZE) {
        throw new Error('expected a 4 MiB ROM, optionally with a 512-byte copier header');
      }
      await new Promise(resolve => requestAnimationFrame(resolve));
      const readStart = performance.now();
      const bytes = new Uint8Array(await selected.arrayBuffer());
      if (token !== operation) throw new Error('stale file selection');
      const compileStart = performance.now();
      const compiled = await transport.replace(bytes);
      const compileMs = performance.now() - compileStart;
      await transport.request('/state');
      await loadArt();
      if (token !== operation || !transport.hasSession()) throw new Error('stale file selection');
      if (!initialized) { initialized = true; controller.init(); }
      controller.reset();
      const report = {read_ms: performance.now()-readStart-compileMs, compile_ms:compileMs,
        wasm_memory_bytes:compiled?.memory_bytes ?? memoryBytes(), note:'Wasm linear-memory capacity, not total browser peak'};
      globalThis.PandoraLocalPreview = {status:'ready', ...report};
      setStatus(`Authenticated checkpoint ready. Compile ${compileMs.toFixed(1)} ms; choose New Game explicitly.`, 'ready');
    } catch (error) {
      transport.invalidate();
      invalidatePresentation(`Local ROM load failed: ${error.message || error}`);
      globalThis.PandoraLocalPreview = {status:'error', error:String(error)};
      setStatus(`Load failed: ${error.message || error}`, 'error');
    } finally {
      input.value = '';
      input.disabled = false;
    }
  });
  return {input, status, dispose() { operation++; transport.invalidate(); invalidatePresentation('Local ROM session closed.'); }};
}
