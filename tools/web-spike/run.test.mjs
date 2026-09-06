import test from 'node:test';
import assert from 'node:assert/strict';
import { runProbe } from './run.mjs';

function fixture() {
  let clock = 0;
  let capacity = 65536;
  const expected = { replay: { state_sha256: 'test', trace_sha256: 'trace' } };
  return {
    file: { size: 4194304, arrayBuffer: async () => new ArrayBuffer(4) },
    probe: () => { capacity = 131072; return JSON.stringify(expected); },
    expected,
    memoryBytes: () => capacity,
    now: () => ++clock,
    jsHeapBytes: () => null,
  };
}

test('measures stages and compares native metadata without claiming JS peak', async () => {
  const result = await runProbe(fixture());
  assert.equal(result.native_match, true);
  assert.equal(result.wasm_linear_capacity_high_water_bytes, 131072);
  assert.equal(result.wasm_linear_before_bytes, 65536);
  assert.equal(result.file_read_ms, 1);
  assert.equal(result.probe_ms, 1);
  assert.equal(result.js_heap_before_bytes, null);
  assert.equal(result.js_heap_after_bytes, null);
  assert.equal(result.browser_total_peak_bytes, null);
});

test('rejects oversized files before reading or copying', async () => {
  const args = fixture();
  args.file = { size: 4194817, arrayBuffer: () => assert.fail('must not read') };
  await assert.rejects(runProbe(args), /size/);
});

test('does not pass a divergent native replay', async () => {
  const args = fixture();
  args.probe = () => '{"replay":{"state_sha256":"wrong"}}';
  await assert.rejects(runProbe(args), /native/);
});

test('surfaces validation failure, never claims success', async () => {
  const args = fixture();
  args.probe = () => { throw new Error('unsupported ROM'); };
  await assert.rejects(runProbe(args), /unsupported ROM/);
});

test('accepts copier-header size and propagates file-read and JSON failures', async () => {
  const args = fixture();
  args.file.size = 4194816;
  assert.equal((await runProbe(args)).native_match, true);
  args.file.arrayBuffer = async () => { throw new Error('file unreadable'); };
  await assert.rejects(runProbe(args), /file unreadable/);
  const malformed = fixture();
  malformed.probe = () => 'not JSON';
  await assert.rejects(runProbe(malformed), SyntaxError);
});
