// No network, filesystem, clock, or browser globals in the measured pipeline.
// Dependencies are injected for deterministic tests; File.arrayBuffer is async.
export async function runProbe({ file, probe, expected, memoryBytes, now, jsHeapBytes }) {
  if (![4194304, 4194816].includes(file.size)) {
    throw new Error('Unsupported file size: expected 4 MiB, optionally +512-byte header');
  }
  const before = memoryBytes();
  const heapBefore = jsHeapBytes();
  const start = now();
  const bytes = new Uint8Array(await file.arrayBuffer());
  const readEnd = now();
  // wasm-bindgen copies bytes into Wasm. Rom::load makes a normalized owned copy.
  // No decoded content is returned, only metadata. Synchronous, no threads/WASI.
  const report = JSON.parse(probe(bytes));
  const probeEnd = now();
  const after = memoryBytes();
  const heapAfter = jsHeapBytes();
  if (JSON.stringify(report) !== JSON.stringify(expected)) {
    throw new Error('Wasm result differs from the native probe');
  }
  return {
    report,
    native_match: true,
    input_bytes: file.size,
    file_read_ms: readEnd - start,
    probe_ms: probeEnd - readEnd,
    file_read_and_probe_ms: probeEnd - start,
    wasm_linear_before_bytes: before,
    wasm_linear_capacity_high_water_bytes: after,
    js_heap_before_bytes: heapBefore,
    js_heap_after_bytes: heapAfter,
    browser_total_peak_bytes: null,
  };
}
