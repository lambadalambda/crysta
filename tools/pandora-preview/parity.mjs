const canonical = value => Array.isArray(value) ? value.map(canonical) : value && typeof value === 'object' ?
  Object.fromEntries(Object.keys(value).sort().map(key => [key, canonical(value[key])])) : value;
const sha256 = async bytes => [...new Uint8Array(await crypto.subtle.digest('SHA-256', bytes))]
  .map(byte => byte.toString(16).padStart(2,'0')).join('');

/** Execute the bounded actual-Wasm route and compare it to native output. */
export async function runParity(runtime) {
  const [fixture, expected] = await Promise.all([
    fetch('./parity-actions.json').then(response => response.json()),
    fetch('./native-parity.json').then(response => {
      if (!response.ok) throw new Error('build with an owned ROM to generate native parity');
      return response.json();
    }),
  ]);
  const replay = await runtime.replayForParity(fixture.actions);
  const commands = replay.commands;
  const visible_unready = replay.state;
  const after_ack = await runtime.request('/step', '6');
  const after_neutral = await runtime.request('/step', '0');
  const states = {visible_unready,after_ack,after_neutral};
  const background_sha256 = {};
  for (const [path,key] of [['/map.bmp','house'],['/exterior.bmp','exterior'],['/town13.bmp','town13'],
      ['/cellars.bmp','cellars'],['/box.bmp','box'],['/tour.bmp','tour']]) {
    background_sha256[key] = await sha256(await (await runtime.loadBackground(path)).arrayBuffer());
  }
  const actual = {schema:1,commands,states,art_sha256:await sha256((await runtime.loadArt()).buffer),background_sha256};
  if (JSON.stringify(canonical(actual)) !== JSON.stringify(canonical(expected))) throw new Error('native/Wasm focused parity differs');
  return actual;
}
