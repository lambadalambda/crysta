// Run on the real loopback room page with agent-browser eval --stdin.
// Optional globalThis.HOUSE_BROWSER_ROUTE is an already JSON-parsed JSONL array.
// No mock host, movement fetch, memory injection or simulated core. The only
// commands below go through actual keyboard/Interact/Pause DOM bindings.
(() => {
  'use strict';
  const same = (a,b) => JSON.stringify(a) === JSON.stringify(b);
  const insist = (ok,message) => { if (!ok) throw new Error(message); };
  const uint = (n,max=0xffffff) => Number.isInteger(n) && n >= 0 && n <= max;
  const pair = value => Array.isArray(value) && value.length === 2 && value.every(Number.isInteger);
  const MAPS = [11,12,13,15,16,17];
  // Independent source census: docs/house-scene.md setup table and native ties.
  // Never derive expected instances, positions or order from /art.json or state.scene.
  const ROSTER = [
    [0x838b96,11,120,112,6,0,false,208,0],
    [0x838c0a,12,88,416,4,1,false,208,3],
    [0x838c14,12,56,384,5,3,false,208,2],
    [0x838c1e,12,72,368,3,0,false,192,1],
    [0x838c28,12,104,368,3,0,false,192,0],
    [0x838cb4,13,72,672,3,0,false,192,0],
    [0x838d7c,16,424,416,2,3,false,208,1],
    [0x838d86,16,440,416,2,2,true,192,0],
    [0x838de2,17,440,640,4,1,true,192,0],
    [0x88d618,15,472,144,0x42,3,false,160,0],
  ].map(([source_id,map_id,x,y,selector,facing,hflip,palette_base,tie_rank]) => ({
    id:`house:${source_id.toString(16)}`,key:`house:${source_id.toString(16)}`,
    source_id,map_id,position:[x,y],selector,facing,hflip,palette_base,tie_rank,
    policy:'frozen-fresh-setup',setup_record:0,
  }));
  const ARK_KEYS = [];
  for (let index=0; index<21; index++) {
    ARK_KEYS.push(`${index}:0`);
    if (index === 2 || index >= 15) ARK_KEYS.push(`${index}:1`);
  }
  const membersEqual = (actual,expected) => Array.isArray(actual) && same([...actual].sort(),[...expected].sort());

  function compileRoute(supplied) {
    const full = supplied !== undefined;
    const chunks = full ? supplied : [
      [2,62],[0,38],[4,67],[0,35],[0,50],[3,14],[0,35],[0,10],
      [2,20],[0,30],[1,20],[0,30],[2,20],[0,30],[1,20],[0,30],
    ].map(([button,steps])=>({button,steps}));
    insist(Array.isArray(chunks) && chunks.length > 0,'Invalid route array');
    const inputs=[], expected=new Map();
    for (const [index,chunk] of chunks.entries()) {
      insist(chunk && typeof chunk === 'object','Invalid route chunk');
      if (chunk.finish === true) {
        insist(index === chunks.length-1 && Object.keys(chunk).length === 1,'Invalid route finish');
        continue;
      }
      insist(uint(chunk.button,5) && uint(chunk.steps,100000) && chunk.steps > 0 && inputs.length+chunk.steps <= 100000,'Invalid route button/steps');
      for (let i=0; i<chunk.steps; i++) inputs.push(chunk.button);
      if (chunk.expect !== undefined) {
        const e=chunk.expect;
        insist(Array.isArray(e) && e.length===5 && MAPS.includes(e[0]) && uint(e[1],65535) && uint(e[2],65535) &&
          typeof e[3]==='string' && ['walking','departing','arriving'].includes(e[3].toLowerCase()) && typeof e[4]==='boolean','Invalid route expect');
        expected.set(inputs.length,[e[0],e[1],e[2],e[3].toLowerCase(),e[4]]);
      }
    }
    insist(inputs.length > 0,'Empty route');
    if (!full) for (const [tick,map,x,y,phase] of [
      [100,15,393,112,'walking'],[167,15,392,208,'departing'],[202,16,392,353,'walking'],
      [266,16,392,336,'departing'],[301,15,392,191,'walking'],[361,15,420,191,'walking'],
      [411,15,392,191,'walking'],[461,15,420,191,'walking'],[511,15,392,191,'walking'],
    ]) expected.set(tick,[map,x,y,phase,false]);
    return {inputs,expected,full};
  }

  function validateArt(art) {
    insist(art?.schema_version===1 && Array.isArray(art.actors) && art.actors.length===10,'Expected complete ten-actor art roster');
    insist(membersEqual(art.actors.map(a=>a.id),ROSTER.map(a=>a.id)),'Actor IDs differ from source census');
    for (const expected of ROSTER) {
      const actor=art.actors.find(a=>a.id===expected.id);
      for (const [field,value] of Object.entries(expected)) {
        insist(same(actor[field],value),`Unqualified ${expected.id} ${field}`);
      }
      const pose=actor.pose;
      insist(pose && (expected.source_id===0x88d618
        ? pose.kind==='direct' && pose.address===0xa2f0ac
        : pose.kind==='compressed' && uint(pose.packet) && pose.packet>=0x800000 && uint(pose.decoded_offset,65535)),`Invalid ${actor.id} pose`);
      insist(uint(actor.graphics_packet) && actor.graphics_packet>=0x800000 && Array.isArray(actor.source_ranges) && actor.source_ranges.length>0 &&
        actor.source_ranges.every(r=>pair(r) && uint(r[0],0x3fffff) && uint(r[1],0x400000) && r[1]>r[0]),`Invalid ${actor.id} source provenance`);
    }
    insist(art.frames && membersEqual(Object.keys(art.frames),[...ARK_KEYS,...ROSTER.map(a=>a.key)]),'Expected exactly 28 Ark + 10 house rasters');
    for (const [key,frame] of Object.entries(art.frames)) {
      insist(frame && uint(frame.width,64) && frame.width>0 && uint(frame.height,64) && frame.height>0 && pair(frame.offset) &&
        Array.isArray(frame.rgba) && frame.rgba.length===frame.width*frame.height*4 &&
        frame.rgba.every((v,i)=>uint(v,255) && (i%4!==3 || v===0 || v===255)),`Invalid raster ${key}`);
    }
    insist(art.scene_ids && membersEqual(Object.keys(art.scene_ids),MAPS.map(String)),'Expected exactly six scene_ids');
    insist(art.foreground && membersEqual(Object.keys(art.foreground),MAPS.map(String)),'Expected exactly six foreground masks');
    for (const map of MAPS) {
      insist(membersEqual(art.scene_ids[map],['ark',...ROSTER.filter(a=>a.map_id===map).map(a=>a.id)]),`Source scene membership differs for map ${map}`);
      const mask=art.foreground[map];
      insist(mask && mask.width===512 && mask.height===1024 && Array.isArray(mask.runs) && mask.runs.length%2===0,`Invalid mask ${map}`);
      let end=0;
      for (let i=0; i<mask.runs.length; i+=2) {
        const [start,length]=mask.runs.slice(i,i+2);
        insist(uint(start) && uint(length) && length>0 && start>=end && start+length<=512*1024,`Invalid mask run ${map}`);
        end=start+length;
      }
    }
    insist(Array.isArray(art.door_patches) && art.door_patches.length===2 &&
      membersEqual(art.door_patches.map(p=>JSON.stringify(p.position)),['[128,304]','[128,320]']),'Invalid door patch positions');
    for (const patch of art.door_patches) {
      insist(Array.isArray(patch.rgba) && patch.rgba.length===1024 && patch.rgba.every((v,i)=>uint(v,255) && (i%4!==3 || v===255)) &&
        Array.isArray(patch.high) && patch.high.length===256 && patch.high.every(v=>typeof v==='boolean'),'Invalid full metatile door patch');
    }
  }

  function expectedScene(map,position,key) {
    insist(MAPS.includes(map) && ARK_KEYS.includes(key),'Unsupported map/Ark frame');
    const residents=ROSTER.filter(a=>a.map_id===map);
    return [...residents,{id:'ark',key,position,tie_rank:residents.length}]
      .sort((a,b)=>a.position[1]-b.position[1] || a.tie_rank-b.tie_rank)
      .map(({id,key,position})=>({id,key,position}));
  }

  // Independent full-canvas BG-low < world-Y OBJ2 < BG-high composition.
  // Art rasters/patches are source-derived inputs, not screenshots or page drawing code.
  function compareCanvas({actual,background,sheetWidth,mask,doorPatches,doorOpen,camera,sprites,width=256,height=224}) {
    const counts=Object.fromEntries(sprites.map(s=>[s.id,{viewportPixels:0,highOccludedPixels:0,actorOccludedPixels:0,pixels:0}]));
    const [cx,cy]=camera;
    for (let y=0; y<height; y++) for (let x=0; x<width; x++) {
      const wx=x+cx, wy=y+cy, world=wy*sheetWidth+wx;
      let pixels=background, at=world*4, high=mask[world], top=null;
      if (doorOpen) for (const patch of doorPatches) {
        const dx=wx-patch.position[0], dy=wy-patch.position[1];
        if (dx>=0 && dx<16 && dy>=0 && dy<16) {
          at=(dy*16+dx)*4; pixels=patch.rgba; high=patch.high[dy*16+dx];
        }
      }
      for (const {id,frame,position} of sprites) {
        const sx=wx-position[0]-frame.offset[0], sy=wy-position[1]-frame.offset[1], src=(sy*frame.width+sx)*4;
        if (sx<0 || sy<0 || sx>=frame.width || sy>=frame.height || frame.rgba[src+3]!==255) continue;
        counts[id].viewportPixels++;
        if (high) counts[id].highOccludedPixels++;
        else {
          if (top!==null) counts[top].actorOccludedPixels++;
          pixels=frame.rgba; at=src; top=id;
        }
      }
      if (top!==null) counts[top].pixels++;
      for (let c=0; c<4; c++) {
        if (actual[(y*width+x)*4+c]!==pixels[at+c]) throw new Error(`Canvas mismatch pixel ${x},${y}, channel ${c}`);
      }
    }
    return counts;
  }

  function requireCoverage(evidence) {
    insist(evidence.ark?.pixels>0,'Full route lacks nonvacuous pixel evidence for Ark');
    const hidden=[];
    for (const actor of ROSTER) {
      const counts=evidence[actor.id];
      if (counts?.pixels>0) continue;
      // Only the source-created table may be entirely behind high background in
      // its frozen setup. Never excuse missing rooms, offscreen-only or alpha-zero art.
      if (actor.source_id===0x88d618 && counts?.viewportPixels>0 && counts.highOccludedPixels===counts.viewportPixels) {
        hidden.push({id:actor.id,reason:'Frozen setup table has opaque viewport samples, all hidden by source high-background pixels; no visible-pixel claim.'});
      } else throw new Error(`Full route lacks nonvacuous pixel evidence for ${actor.id}`);
    }
    return hidden;
  }

  function createControls($,key) {
    let held='', previous=null;
    function input(next) {
      if (next===held) return;
      if (held) key('keyup',held);
      held=next;
      if (held) key('keydown',held);
    }
    function click(id,label) {
      const button=$(id);
      insist(button && !button.disabled && (!label || button.textContent===label),`Unavailable actual ${id} control (${label || 'Interact'})`);
      button.click();
    }
    return {
      resume() { click('pause','Resume'); },
      apply(button,state) {
        if (previous && (previous.map!==state.map || previous.phase!==state.phase)) input('');
        previous=state;
        if (button===5) {
          input('');
          insist(state.phase==='walking','Interaction during transition would be discarded');
          click('interact');
        } else input(state.phase==='walking' ? ['','ArrowLeft','ArrowRight','ArrowUp','ArrowDown'][button] : '');
      },
      stop() { input(''); if ($('pause').textContent==='Pause') click('pause','Pause'); },
    };
  }

  function createTickDriver({route,controls,checkTick,finish}) {
    let seen=0;
    return tick=> {
      if (tick===seen) return;
      insist(tick===seen+1,`Non-sequential UI tick ${seen} -> ${tick}`);
      seen=tick;
      const view=checkTick(tick);
      if (tick===route.inputs.length) { finish(); return; }
      // Interact pauses and acknowledges exactly one logical tick. Only now
      // resume the actual pump; never queue a direction into its pending command.
      if (route.inputs[tick-1]===5) controls.resume();
      controls.apply(route.inputs[tick],view);
    };
  }

  // CommonJS is only a bounded helper test seam; browser eval returns the promise.
  if (typeof module!=='undefined' && module.exports && typeof document==='undefined') {
    module.exports={compileRoute,validateArt,expectedScene,compareCanvas,createControls,createTickDriver,requireCoverage};
    return;
  }
  return (async () => {
    const route=compileRoute(globalThis.HOUSE_BROWSER_ROUTE);
    const $=id=>document.getElementById(id);
    const until=async predicate=> {
      const deadline=performance.now()+5000;
      while (!predicate()) {
        if (performance.now()>deadline) throw new Error('UI startup timed out');
        await new Promise(resolve=>setTimeout(resolve,10));
      }
    };
    insist($('new-game') && !$('new-game').disabled,'New Game control is unavailable');
    $('new-game').click();
    await until(()=>$('position').textContent==='304, 112' && $('tick').textContent==='0' && !$('pause').disabled);
    const initial=await (await fetch('/state')).json();
    insist(initial.start_kind==='new-game' && initial.tick===0 && initial.error===null,'Host did not acknowledge a clean fresh New Game');
    const art=await (await fetch('/art.json')).json();
    validateArt(art); // Complete six-scene contract, including on the old511 route.
    const image=new Image();
    await new Promise((resolve,reject)=> { image.onload=resolve; image.onerror=reject; image.src='/map.bmp'; });
    insist(image.width===512 && image.height===1024,'Expected 512x1024 closed background');
    const sheet=document.createElement('canvas'); sheet.width=image.width; sheet.height=image.height;
    const ctx=sheet.getContext('2d'); ctx.drawImage(image,0,0);
    const background=ctx.getImageData(0,0,sheet.width,sheet.height).data;
    const masks=Object.fromEntries(Object.entries(art.foreground).map(([id,mask])=> {
      const bytes=new Uint8Array(mask.width*mask.height);
      for (let i=0; i<mask.runs.length; i+=2) bytes.fill(1,mask.runs[i],mask.runs[i]+mask.runs[i+1]);
      return [id,bytes];
    }));
    const visualKeys=new Set(), visitedMaps=new Set(), actorEvidence=Object.fromEntries(['ark',...ROSTER.map(a=>a.id)].map(id=>
      [id,{sceneChecks:0,visibleChecks:0,viewportPixels:0,highOccludedPixels:0,actorOccludedPixels:0,pixels:0}]));
    let visualChecks=0;
    function readView() {
      const door=$('room').dataset.woodenDoorOpen;
      insist(door==='true' || door==='false','Missing boolean canvas woodenDoorOpen');
      const position=$('position').textContent.split(', ').map(Number), camera=$('camera').textContent.split(', ').map(Number);
      const map=parseInt($('map').textContent.slice(1),16);
      insist(pair(position) && pair(camera) && camera[0]>=0 && camera[1]>=0 && camera[0]+256<=512 && camera[1]+224<=1024,'Invalid UI position/camera');
      return {map,position,camera,phase:$('phase').textContent,door:door==='true',key:$('actor-key').textContent};
    }
    function checkPixels(view) {
      insist(!$('error').textContent,$('error').textContent);
      const scene=expectedScene(view.map,view.position,view.key);
      const actualScene=JSON.parse($('room').dataset.scene);
      insist(Array.isArray(actualScene) && same(actualScene.map(({id,key,position})=>({id,key,position})),scene),'Canvas scene identity/membership/position/native painter order differs');
      const counts=compareCanvas({actual:$('room').getContext('2d').getImageData(0,0,256,224).data,
        background,sheetWidth:sheet.width,mask:masks[view.map],doorPatches:art.door_patches,doorOpen:view.door,camera:view.camera,
        sprites:scene.map(entry=>({...entry,frame:art.frames[entry.key]}))});
      for (const entry of scene) {
        visualKeys.add(entry.key);
        const total=actorEvidence[entry.id], count=counts[entry.id];
        total.sceneChecks++; if (count.pixels>0) total.visibleChecks++;
        for (const field of Object.keys(count)) total[field]+=count[field];
      }
      visitedMaps.add(view.map); visualChecks++;
    }
    let view=readView();
    checkPixels(view);
    const controls=createControls($,(type,key)=>document.dispatchEvent(new KeyboardEvent(type,{key,bubbles:true})));
    let seen=0;
    const checkpoints=[];
    await new Promise((resolve,reject)=> {
      let observer, finished=false;
      const timeout=setTimeout(()=>finish(new Error('Exploration UI timed out')),Math.max(30000,route.inputs.length*100));
      function finish(error) {
        if (finished) return;
        finished=true; observer?.disconnect(); clearTimeout(timeout);
        try { controls.stop(); } catch (failure) { error ||= failure; }
        error ? reject(error) : resolve();
      }
      const drive=createTickDriver({route,controls,finish,checkTick(tick) {
        seen=tick; view=readView(); checkPixels(view);
        const actual=[view.map,...view.position,view.phase,view.door];
        if (route.expected.has(tick)) {
          insist(same(actual,route.expected.get(tick)),`Checkpoint ${tick}: ${JSON.stringify(actual)} expected ${JSON.stringify(route.expected.get(tick))}`);
          checkpoints.push({tick,map:$('map').textContent,position:$('position').textContent,phase:view.phase,woodenDoorOpen:view.door});
        }
        return view;
      }});
      observer=new MutationObserver(()=> {
        if (finished) return;
        try {
          insist(!$('error').textContent,$('error').textContent);
          drive(Number($('tick').textContent));
        } catch (error) { finish(new Error(`Tick ${seen}: ${error.message}`)); }
      });
      observer.observe($('tick'),{childList:true});
      try { controls.resume(); controls.apply(route.inputs[0],view); } catch (error) { finish(error); }
    });
    const final=await (await fetch('/state')).json();
    insist(final.tick===route.inputs.length && final.error===null && same(
      [final.map_id,final.x,final.y,final.phase,final.wooden_door_open],
      [view.map,...view.position,view.phase,view.door]),`Final host state differs: ${JSON.stringify(final)}`);
    const hiddenActors=route.full ? requireCoverage(actorEvidence) : [];
    return {kind:'real-browser-new-game-house-exploration',route:route.full?'supplied-full-route':'default511',initial,checkpoints,final,
      visualChecks,actorEvidence,hiddenActors,visitedMaps:[...visitedMaps].sort((a,b)=>a-b),spriteKeys:[...visualKeys].sort(),
      limits:'Default-name semantic start; six frozen fresh-setup house scenes (nine residents and one table), 28 ordinary Ark rasters. Full canvas uses source rasters, closed first background, wooden-door final metatile patches and high-pixel occlusion. No native AI, wandering, dialogue, shadows, secondary effects or intro presentation. Transition timing is logical, not native video timing. Only the selected route is exercised; no every-actor/every-tick visibility claim.'};
  })();
})()
