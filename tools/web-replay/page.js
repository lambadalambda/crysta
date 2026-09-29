// Installed into the page before its scripts run. It replaces
// requestAnimationFrame with a queue that the replay pumps one PAL frame at a
// time, silences sound, and records what the page's loop passes to WebGame.
// The page's own key handlers and frame loop run unchanged.
(() => {
  const qa = { now: 0, period: 0, queue: [], calls: [], game: null };
  window.requestAnimationFrame = (callback) => qa.queue.push(callback);

  // Sound is rendered (as on the real page) but never played.
  class SilentAudio {
    constructor() { this.destination = {}; }
    get currentTime() { return qa.now / 1000; }
    resume() { return Promise.resolve(); }
    suspend() { return Promise.resolve(); }
    createBuffer(channels, length) {
      const data = Array.from({ length: channels }, () => new Float32Array(length));
      return { getChannelData: (index) => data[index] };
    }
    createBufferSource() { return { buffer: null, connect() {}, start() {} }; }
  }
  window.AudioContext = SilentAudio;

  // The page's module instance: same URL, so the same WebGame class.
  qa.hook = async () => {
    const { WebGame } = await import(new URL('crysta_web.js', document.baseURI));
    const { frame_ms: rate, frame_with_presses: frame } = WebGame.prototype;
    // The loop asks for the console's rate every callback.
    WebGame.prototype.frame_ms = function () {
      qa.game = this;
      return (qa.period = rate.call(this));
    };
    WebGame.prototype.frame_with_presses = function (held, pressed) {
      qa.calls.push([held, pressed]);
      return frame.call(this, held, pressed);
    };
  };

  const status = () => document.getElementById('status');
  qa.tick = (ms) => {
    if (qa.queue.length !== 1) throw new Error(`page loop stopped (${qa.queue.length} RAF callbacks)`);
    qa.now += ms;
    qa.queue.shift()(qa.now);
    if (status().className === 'error') throw new Error(`page says: ${status().textContent}`);
  };

  // The first callback only starts the page's clock.
  qa.begin = () => {
    qa.tick(0);
    if (qa.calls.length || !qa.period) throw new Error('the page clock did not start cleanly');
    return qa.period;
  };

  // `count` callbacks of one PAL frame each; returns the page's inputs.
  qa.frames = (count) => {
    for (let i = 0; i < count; i++) {
      const before = qa.calls.length;
      qa.tick(qa.period);
      // IEEE rounding can leave a frame an epsilon short of its boundary.
      if (qa.calls.length === before) qa.tick(0.001);
      if (qa.calls.length !== before + 1) throw new Error(`${qa.calls.length - before} frames in one step`);
    }
    return qa.calls.splice(0);
  };

  // What a player can see: the fault, the status line and the canvas bytes.
  qa.view = () => {
    const canvas = document.getElementById('view');
    const pixels = canvas.getContext('2d').getImageData(0, 0, canvas.width, canvas.height).data;
    let binary = '';
    for (let i = 0; i < pixels.length; i += 0x8000) {
      binary += String.fromCharCode(...pixels.subarray(i, i + 0x8000));
    }
    return { fault: qa.game?.fault() ?? null, status: status().textContent, rgba: btoa(binary) };
  };

  window.__replay = qa;
})();
