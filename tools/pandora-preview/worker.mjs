import init, { PandoraPreview } from './pandora_web.js';

const wasm = await init();
let session = null;
const reply = (id, ok, value, transfer = []) => postMessage({id, ok, value}, transfer);

onmessage = ({data:{id, type, value}}) => {
  try {
    if (type === 'dispose') {
      session?.free(); session = null;
      return reply(id, true, null);
    }
    if (type === 'replace') {
      session?.free(); session = null;
      session = new PandoraPreview(new Uint8Array(value));
      return reply(id, true, {memory_bytes:wasm.memory.buffer.byteLength});
    }
    if (!session) throw new Error('no authenticated local ROM session');
    if (type === 'request') {
      const {path, body} = value;
      let json;
      if (path === '/state' && body === undefined) json = session.state();
      else if (path === '/new-game' && body === '') json = session.new_game();
      else if (path === '/reset' && body === '') json = session.reset();
      else if (path === '/step' && typeof body === 'string' && /^(?:0|[1-9]|10)$/.test(body)) json = session.step(Number(body));
      else throw new Error('unsupported preview request');
      return reply(id, true, JSON.parse(json));
    }
    if (type === 'parity') {
      if (!Array.isArray(value) || value.length > 128) throw new Error('unsupported parity fixture');
      session.new_game();
      let commands = 0, inputs = [];
      for (const run of value) {
        if (!Array.isArray(run) || run.length !== 2 || !Number.isInteger(run[0]) || run[0] < 0 || run[0] > 10 ||
            !Number.isInteger(run[1]) || run[1] < 1 || commands + run[1] > 6000) throw new Error('unsupported parity fixture');
        inputs.push(...Array(run[1]).fill(run[0]));
        commands += run[1];
      }
      return reply(id, true, {commands,state:JSON.parse(session.runParityInputs(new Uint8Array(inputs)))});
    }
    if (type === 'art') {
      const bytes = session.art();
      return reply(id, true, bytes.buffer, [bytes.buffer]);
    }
    if (type === 'background') {
      const bytes = session.background(value);
      return reply(id, true, bytes.buffer, [bytes.buffer]);
    }
    throw new Error('unsupported preview worker command');
  } catch (error) {
    reply(id, false, String(error?.message || error));
  }
};
