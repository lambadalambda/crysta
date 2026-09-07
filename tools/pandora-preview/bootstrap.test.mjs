import test from 'node:test';
import assert from 'node:assert/strict';
import { createSessionTransport, createWorkerTransport } from './bootstrap.mjs';

class Preview {
  static created = 0;
  constructor(bytes) {
    if (bytes[0] !== 7) throw new Error('bad rom');
    this.tick = 0; this.freed = false; Preview.created++;
  }
  state() { return JSON.stringify({tick:this.tick}); }
  step(input) { this.tick += input; return this.state(); }
  new_game() { this.tick = 100; return this.state(); }
  reset() { this.tick = 0; return this.state(); }
  art() { return new Uint8Array([1,2,3]); }
  background(key) { if(key==='house') return new Uint8Array([4,5]); throw new Error('bad key'); }
  free() { this.freed = true; }
}

const make = () => createSessionTransport({Preview, BlobCtor: Blob});

test('replacement clears stale session and rejects malformed bytes', async () => {
  const transport = make();
  transport.replace(new Uint8Array([7]));
  assert.equal(transport.hasSession(), true);
  assert.throws(() => transport.replace(new Uint8Array([0])), /bad rom/);
  assert.equal(transport.hasSession(), false);
  await assert.rejects(transport.request('/state'), /no authenticated/i);
});

test('closed endpoint protocol never fetches and preserves command identities', async () => {
  const transport = make(); transport.replace(new Uint8Array([7]));
  assert.deepEqual(await transport.request('/state'), {tick:0});
  assert.deepEqual(await transport.request('/step','5'), {tick:5});
  assert.deepEqual(await transport.request('/new-game',''), {tick:100});
  assert.deepEqual(await transport.request('/reset',''), {tick:0});
  for (const call of [['/step','11'],['/step','05'],['/step',5],['/other',undefined],['/state','']]) {
    await assert.rejects(transport.request(...call), /unsupported/i);
  }
});

test('art and backgrounds are owned values and URL admission is exact', async () => {
  const transport = make(); transport.replace(new Uint8Array([7]));
  const art = transport.loadArt(); art[0] = 99;
  assert.deepEqual([...transport.loadArt()], [1,2,3]);
  const blob = transport.loadBackground('/map.bmp');
  assert.deepEqual([...new Uint8Array(await blob.arrayBuffer())], [4,5]);
  assert.throws(() => transport.loadBackground('/map.bmp?x'), /unsupported/i);
});

test('synchronous worker transfer failure becomes terminal', async () => {
  const worker = {postMessage(){throw new Error('transfer failed');},terminate(){}};
  const transport = createWorkerTransport({worker, BlobCtor:Blob});
  await assert.rejects(transport.replace(new Uint8Array([7])), /transfer failed/);
  await assert.rejects(transport.replace(new Uint8Array([7])), /transfer failed/);
});

test('worker terminal failures reject pending work and future calls', async () => {
  class FailedWorker {
    postMessage(message) { this.last = message; }
    terminate() {}
  }
  const worker = new FailedWorker();
  const transport = createWorkerTransport({worker, BlobCtor:Blob});
  const replacing = transport.replace(new Uint8Array([7]));
  worker.onerror(new Error('worker crashed'));
  await assert.rejects(replacing, /worker crashed/);
  await assert.rejects(transport.request('/state'), /no authenticated/i);
  await assert.rejects(transport.replace(new Uint8Array([7])), /worker crashed/);
});

test('worker transport invalidates in-flight results before replacement', async () => {
  class FakeWorker {
    sent = [];
    postMessage(message) { this.sent.push(message); }
    respond(message, ok = true, value = null) { this.onmessage({data:{id:message.id,ok,value}}); }
    terminate() {}
  }
  const worker = new FakeWorker();
  const transport = createWorkerTransport({worker, BlobCtor:Blob});
  const replacing = transport.replace(new Uint8Array([7]));
  assert.deepEqual(worker.sent.map(({type})=>type), ['dispose','replace']);
  worker.respond(worker.sent[0]);
  worker.respond(worker.sent[1], true, {memory_bytes:42});
  assert.deepEqual(await replacing, {memory_bytes:42});
  const pending = transport.request('/state');
  transport.invalidate();
  await assert.rejects(pending, /stale/i);
  worker.respond(worker.sent.find(({type})=>type==='request'), true, {tick:0});
  assert.equal(transport.hasSession(), false);
});

test('generation rejects a result retained across replacement', async () => {
  const transport = make(); transport.replace(new Uint8Array([7]));
  const pending = transport.request('/state');
  transport.invalidate();
  await assert.rejects(pending, /stale/i);
});
