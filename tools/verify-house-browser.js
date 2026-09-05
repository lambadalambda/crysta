// Run on the real loopback room page with agent-browser eval --stdin.
// No mock host, direct movement API calls, memory injection or simulated core.
// Inputs use the page's actual button/keyboard bindings and request pump.
(async () => {
  const $ = id => document.getElementById(id);
  const start = $('new-game');
  if (!start || start.disabled) throw new Error('New Game control is unavailable');
  const until = async predicate => {
    const deadline = performance.now() + 5000;
    while (!predicate()) {
      if (performance.now() > deadline) throw new Error('UI startup timed out');
      await new Promise(resolve => setTimeout(resolve, 10));
    }
  };
  start.click();
  await until(() => $('position').textContent === '304, 112' && $('tick').textContent === '0' && !$('pause').disabled);
  const initial = await (await fetch('/state')).json();
  if (initial.start_kind !== 'new-game' || initial.tick !== 0 || initial.error !== null) {
    throw new Error('Host did not acknowledge a clean fresh New Game');
  }
  // Independent pixel composition verifies the actual canvas, not just state.
  const art = await (await fetch('/art.json')).json();
  const image = new Image();
  await new Promise((resolve,reject) => { image.onload=resolve; image.onerror=reject; image.src='/map.bmp'; });
  const sheet = document.createElement('canvas'); sheet.width=image.width; sheet.height=image.height;
  const sheetContext=sheet.getContext('2d'); sheetContext.drawImage(image,0,0);
  const background=sheetContext.getImageData(0,0,sheet.width,sheet.height).data;
  const masks=Object.fromEntries(Object.entries(art.foreground).map(([id, mask]) => {
    const bytes=new Uint8Array(mask.width*mask.height);
    for(let i=0;i<mask.runs.length;i+=2) bytes.fill(1,mask.runs[i],mask.runs[i]+mask.runs[i+1]);
    return [id,bytes];
  }));
  const visualKeys=new Set(); let visualChecks=0;
  function checkPixels() {
    const actorKey=$('actor-key').textContent, actor=art.frames[actorKey];
    if(!actor) throw new Error(`Missing actual sprite ${actorKey}`);
    const [px,py]=$('position').textContent.split(', ').map(Number);
    const [cx,cy]=$('camera').textContent.split(', ').map(Number);
    const map=parseInt($('map').textContent.slice(1),16), mask=masks[map];
    const actual=$('room').getContext('2d').getImageData(0,0,256,224).data;
    for(let y=0;y<224;y++) for(let x=0;x<256;x++) {
      const world=(y+cy)*sheet.width+x+cx, bg=world*4;
      const sx=x+cx-px-actor.offset[0], sy=y+cy-py-actor.offset[1];
      const src=(sy*actor.width+sx)*4;
      const opaque=sx>=0 && sy>=0 && sx<actor.width && sy<actor.height && actor.rgba[src+3]===255 && !mask[world];
      for(let c=0;c<4;c++) {
        const expected=opaque?actor.rgba[src+c]:background[bg+c];
        if(actual[(y*256+x)*4+c]!==expected) throw new Error(`Canvas mismatch tick ${$('tick').textContent}, key ${actorKey}, pixel ${x},${y}, channel ${c}`);
      }
    }
    visualKeys.add(actorKey); visualChecks++;
  }
  checkPixels();
  // Fresh control witness, then repeated directions to the covered doorway pair.
  // Reference input labels begin at6800. Doorway steps use explicit semantic
  // 17/load/17 timing, not the native loader's video-frame schedule.
  const chunks = [
    ['ArrowRight',62], ['',38], ['ArrowDown',67], ['',35],
    ['',50], ['ArrowUp',14], ['',35], ['',10],
    ['ArrowRight',20], ['',30], ['ArrowLeft',20], ['',30],
    ['ArrowRight',20], ['',30], ['ArrowLeft',20], ['',30],
  ];
  const inputs = chunks.flatMap(([key, count]) => Array(count).fill(key));
  const expected = new Map([
    [100, ['$000F','393, 112','walking']],
    [167, ['$000F','392, 208','departing']],
    [202, ['$0010','392, 353','walking']],
    [266, ['$0010','392, 336','departing']],
    [301, ['$000F','392, 191','walking']],
    [361, ['$000F','420, 191','walking']],
    [411, ['$000F','392, 191','walking']],
    [461, ['$000F','420, 191','walking']],
    [511, ['$000F','392, 191','walking']],
  ]);
  let held = '', seen = 0;
  const evidence = [];
  const key = (type, value) => document.dispatchEvent(new KeyboardEvent(type, {key:value, bubbles:true}));
  function input(next) {
    if (next === held) return;
    if (held) key('keyup', held);
    held = next;
    if (held) key('keydown', held);
  }
  await new Promise((resolve, reject) => {
    let observer;
    const timeout = setTimeout(() => finish(new Error('Exploration UI timed out')), 30000);
    function finish(error) {
      observer.disconnect(); clearTimeout(timeout); input(''); key('keydown','Escape');
      error ? reject(error) : resolve();
    }
    observer = new MutationObserver(() => {
      try {
        if ($('error').textContent) throw new Error($('error').textContent);
        const tick = Number($('tick').textContent);
        if (tick === seen) return;
        if (tick !== seen + 1) throw new Error(`Non-sequential UI tick ${seen} -> ${tick}`);
        seen = tick;
        checkPixels();
        const actual = [$('map').textContent, $('position').textContent, $('phase').textContent];
        if (expected.has(tick)) {
          if (JSON.stringify(actual) !== JSON.stringify(expected.get(tick))) {
            throw new Error(`Checkpoint ${tick}: ${JSON.stringify(actual)}`);
          }
          evidence.push({tick, map:actual[0], position:actual[1], phase:actual[2]});
        }
        if (tick === inputs.length) finish();
        else input(inputs[tick]);
      } catch (error) { finish(error); }
    });
    observer.observe($('tick'), {childList:true});
    $('pause').click();
    input(inputs[0]);
  });
  const final = await (await fetch('/state')).json();
  if (final.tick !== 511 || final.map_id !== 15 || final.x !== 392 || final.y !== 191 || final.error !== null) {
    throw new Error(`Final host state differs: ${JSON.stringify(final)}`);
  }
  return {kind:'real-browser-new-game-house-exploration', initial, checkpoints:evidence, final,
    visualChecks, spriteKeys:[...visualKeys].sort(),
    limits:'Default-name semantic start; intro/dialogue presentation omitted. ROM-derived ordinary Ark poses, static first background with high-pixel occlusion, passive cardinal walking and two endpoint-timed doorways only.'};
})()
