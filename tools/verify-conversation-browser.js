// Input-only acceptance recipe, twice verified against the integrated profile9 host.
// On the integrated real room page:
// { echo 'globalThis.HOUSE_BROWSER_HELPERS_ONLY=true;'; cat tools/verify-house-browser.js;
//   echo '; globalThis.CONVERSATION_BROWSER_READY=true;'; cat tools/verify-conversation-browser.js;
// } | agent-browser eval --stdin
// Returns immediately; inspect CONVERSATION_BROWSER_RUN.status/result/error later.
// The retained .promise also resolves to the result; do not long-await it over CLI.
// Parent may override motion spans via CONVERSATION_BROWSER_MOTION, not source
// expectations. The default1671-step route qualifies Up100/release20 and B->D->A.
(() => {
  'use strict';
  const insist=(ok,message)=> { if (!ok) throw new Error(message); };
  const same=(a,b)=>JSON.stringify(a)===JSON.stringify(b);
  const members=(a,b)=>same(Object.keys(a || {}).sort(),b.slice().sort());
  // docs/house-dialogue.md. Independent fixed source boundaries, NOT host-derived flow.
  const REQUESTS={
    '888fda':[[0x888fef,19,'end']],
    '888ff0':[[0x889027,37,'next'],[0x889059,36,'none']],
    '88905a':[[0x889083,39,'next'],[0x8890b0,33,'next'],[0x8890d8,36,'end']],
    '8890d9':[[0x8890f8,29,'next'],[0x889126,39,'next'],[0x889155,39,'end']],
    '889156':[[0x88918b,35,'none']],
    '88918c':[[0x8891aa,27,'next'],[0x8891d5,36,'end']],
    '8891d6':[[0x8891eb,20,'next'],[0x889217,38,'end']],
  };
  function validateRaster(frame,width,height,key) {
    insist(frame?.width===width && frame.height===height && Array.isArray(frame.rgba) && frame.rgba.length===width*height*4 &&
      frame.rgba.every((v,i)=>Number.isInteger(v) && v>=0 && v<=255 && (i%4!==3 || v===0 || v===255)),`Invalid raster ${key}`);
    compareRaster({width,height,data:frame.rgba},frame); // reject blank source inputs too
  }
  function compareRaster(actual,frame) {
    insist(actual.width===frame.width && actual.height===frame.height && actual.data.length===frame.rgba.length,'Raster dimensions differ');
    let ink=0; const colors=new Set();
    for (let i=0;i<frame.rgba.length;i+=4) {
      const alpha=frame.rgba[i+3];
      for (let c=0;c<4;c++) {
        // Canvas canonicalizes fully transparent RGB; no visible-pixel exemption.
        const expected=alpha===0 && c<3?0:frame.rgba[i+c];
        insist(actual.data[i+c]===expected,`Raster pixel ${i/4}, channel ${c} differs`);
      }
      colors.add(alpha===0?'transparent':frame.rgba.slice(i,i+4).join(','));
      if (alpha===255 && frame.rgba.slice(i,i+3).some(v=>v>0)) ink++;
    }
    insist(ink>0 && colors.size>1,'Raster lacks nonvacuous foreground/background variation');
    return ink;
  }
  function validateDialogueArt(art) {
    insist(members(art.dialogue_requests,Object.keys(REQUESTS)),'Expected seven source requests');
    const keys=[];
    for (const [source,pages] of Object.entries(REQUESTS)) {
      const actual=art.dialogue_requests[source];
      insist(Array.isArray(actual) && actual.length===pages.length,`Request page count ${source}`);
      pages.forEach(([boundary_source,glyph_count,acknowledgement],index)=> {
        const key=`text:${source}:${index}`, expected={key,page_id:(parseInt(source,16)<<4)|index,boundary_source,glyph_count,acknowledgement};
        for (const [field,value] of Object.entries(expected)) insist(actual[index]?.[field]===value,`Source ${key} ${field} differs`);
        keys.push(key); validateRaster(art.dialogue_pages?.[key],224,48,key);
      });
    }
    insist(members(art.choice_catalogs,['0','1']),'Expected source catalogs0/1');
    for (const catalog of [0,1]) {
      const expected=[`choice:${catalog}:1`,`choice:${catalog}:2`];
      insist(same(art.choice_catalogs[catalog],expected),`Source catalog ${catalog} result order differs`);
      const tail=art.dialogue_pages[catalog===0?'text:888ff0:1':'text:889156:0'];
      expected.forEach((key,index)=> {
        keys.push(key); const crop=art.dialogue_pages[key]; validateRaster(crop,212,16,key);
        for(let y=0;y<16;y++) for(let x=0;x<212;x++) for(let c=0;c<4;c++) {
          insist(crop.rgba[(y*212+x)*4+c]===tail.rgba[((y+16*(index+1))*224+x+12)*4+c],`Source choice crop ${key} differs`);
        }
      });
    }
    insist(members(art.dialogue_pages,keys),'Expected exactly fourteen source pages and four crops');
  }
  function conversationPlan() {
    return [
      [5,'888ff0',0,null,false],[6,'888ff0',1,0],[8,'8890d9',0],
      [6,'8890d9',1],[6,'8890d9',2],[6,null],
      [5,'889156',0,1],[7,'8891d6',0],[6,'8891d6',1],[6,null],
      [5,'889156',0,1],[8,'88918c',0],[6,'88918c',1],[6,null],
    ].map(([command,source,index,choice=null,granted=true])=>({command,key:source?`text:${source}:${index}`:null,choice,granted}));
  }
  function checkEvents(state,granted) {
    insist(state.error===null,'Host error: '+state.error);
    insist(same(state.events,granted?[32,38,251]:[32,251]),`Native events differ: ${JSON.stringify(state.events)}`);
  }
  function checkConversationState(state,expected) {
    checkEvents(state,expected.granted);
    insist(state.choice_interaction===true && state.dialogue_acknowledgement===true,'Missing dialogue capabilities');
    insist(state.map_id===11 && state.x===120 && state.y===128,'Ark moved during resident dialogue');
    insist(state.phase===(expected.key?'dialogue':'walking'),'Unexpected conversation phase');
    if (!expected.key) insist(state.dialogue===null,'Follow-up did not close');
    else {
      insist(state.dialogue?.key===expected.key,'Wrong source page/branch');
      insist((state.dialogue.choice ?? null)===expected.choice,'Wrong choice/page wait (None tails must be choice waits)');
    }
  }
  function createAckLatch(initialTick) {
    let lastTick=initialTick,pending=false;
    return {
      get lastTick() { return lastTick; },
      arm() { insist(!pending,'Command already pending'); pending=true; },
      observe(tick) {
        if (tick===lastTick) return false;
        insist(pending,'Unsolicited UI tick');
        insist(tick===lastTick+1,`Non-sequential UI tick ${lastTick} -> ${tick}`);
        lastTick=tick;pending=false;return true;
      },
    };
  }
  function createUICommands($,key) {
    const click=id=> { const button=$(id);insist(button && !button.disabled && !button.hidden,`Unavailable actual ${id} button`);button.click(); };
    const pause=()=> { if ($('pause').textContent==='Pause') click('pause'); };
    return {click,pause,
      release() {for(const direction of ['ArrowLeft','ArrowRight','ArrowUp','ArrowDown']) key('keyup',direction);},
      send(command,state) {
        insist(Number.isInteger(command) && command>=0 && command<=9,'Invalid UI command');
        insist($('pause').textContent==='Resume','One-shot command must start paused');
        if (command>=5) click(['interact','continue','choice-cancel','choice1','choice2'][command-5]);
        else {
          insist(state.phase!=='dialogue','Movement command during dialogue');
          click('pause');
          if(command && state.phase==='walking') key('keydown',['','ArrowLeft','ArrowRight','ArrowUp','ArrowDown'][command]);
        }
      },
    };
  }
  function motionRecipe(overrides={}) {
    const recipe={
      // Exactly first15 core-route.jsonl chunks, ending B(120,191).
      toB:[[2,62],[0,38],[4,67],[0,35],[4,42],[1,78],[0,90],[1,12],[0,100],[1,55],[3,70],[0,100],[5,1],[3,13],[0,35]],
      target:[[3,100],[0,20]],
      toD:[[4,58],[0,35],[1,10],[4,82],[0,100]],
      toA:[[4,68],[0,180]],
      down:[[4,32],[0,60]],right:[[2,24],[0,90]],
      ...overrides,
    };
    insist(members(recipe,['toB','target','toD','toA','down','right']),'Unknown motion override');
    for (const chunks of Object.values(recipe)) insist(Array.isArray(chunks) && chunks.length>0 && chunks.every(c=>
      Array.isArray(c) && c.length===2 && Number.isInteger(c[0]) && c[0]>=0 && c[0]<=5 &&
      Number.isInteger(c[1]) && c[1]>0 && c[1]<=10000),'Invalid motion recipe');
    return recipe;
  }
  function checkExterior(state) {
    insist(state.map_id===10 && state.phase==='walking','Not settled exterior A');
    // Conservative qualified player envelope inside source-cell halo [29,47,36,53].
    // This observes the route, not the backend's old/new collision probes.
    insist(state.x>=488 && state.x<=552 && state.y>=768 && state.y<=832,'Outside qualified exterior halo/envelope');
    insist(same(state.camera,[Math.max(0,Math.min(768,state.x-128)),Math.max(0,Math.min(768,state.y-112))]),'Exterior source camera differs');
  }
  const helpers={validateDialogueArt,conversationPlan,checkConversationState,compareRaster,createAckLatch,createUICommands,motionRecipe,checkExterior};
  if (typeof module!=='undefined' && module.exports && typeof document==='undefined') { module.exports=helpers; return; }
  globalThis.ConversationBrowserHelpers=helpers;
  insist(globalThis.CONVERSATION_BROWSER_READY===true,'Backend readiness must be explicitly confirmed; no live inputs sent');
  insist(globalThis.HouseBrowserHelpers,'Load verify-house-browser.js with HOUSE_BROWSER_HELPERS_ONLY=true first');
  insist(globalThis.CONVERSATION_BROWSER_RUN?.status!=='running','Conversation verifier already running');
  const run=globalThis.CONVERSATION_BROWSER_RUN={status:'running',lastTick:0,checkpoints:[],visualChecks:0,rasterEvidence:{},motion:motionRecipe(globalThis.CONVERSATION_BROWSER_MOTION)};
  run.promise=(async()=> {
    const H=globalThis.HouseBrowserHelpers, $=id=>document.getElementById(id);
    const key=(type,key)=>document.dispatchEvent(new KeyboardEvent(type,{key,bubbles:true}));
    const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
    const get=async path=> {
      const response=await fetch(path,{cache:'no-store',signal:AbortSignal.timeout(5000)});
      insist(response.ok,`GET ${path}: ${response.status}`);return response.json();
    };
    const controls=createUICommands($,key), {click,pause}=controls;
    try {
      // Observe contracts before any input. Readiness flag alone must not reset an old host.
      const art=await get('/art.json');H.validateArt(art);validateDialogueArt(art);
      const before=await get('/state');
      insist(before.choice_interaction===true && before.dialogue_acknowledgement===true,'Backend conversation integration is not ready');
      const backgrounds=await H.loadBackgrounds(art),masks=H.expandMasks(art);
      click('new-game');
      const deadline=performance.now()+5000;
      while ($('position').textContent!=='304, 112' || $('tick').textContent!=='0' || $('pause').disabled || !$('step') || $('step').disabled) {
        insist(performance.now()<deadline,'New Game startup timeout');await sleep(10);
      }
      let state=await get('/state');
      insist(state.start_kind==='new-game' && state.tick===0 && state.map_id===15 && state.x===304 && state.y===112 && state.phase==='walking' && state.dialogue===null,'Not a fresh New Game');
      checkEvents(state,false);run.initial=state;
      const latch=createAckLatch(0);
      function pixels(s) {
        insist(!$('error').textContent,$('error').textContent);
        insist(Number($('tick').textContent)===s.tick && $('map').textContent===`$${s.map_id.toString(16).toUpperCase().padStart(4,'0')}` &&
          $('position').textContent===`${s.x}, ${s.y}` && $('camera').textContent===s.camera.join(', ') &&
          $('phase').textContent===s.phase && $('actor-key').textContent===s.actor_key,'DOM/GET state diverged');
        const view={map:s.map_id,position:[s.x,s.y],key:s.actor_key,camera:s.camera,door:s.wooden_door_open};
        insist($('room').dataset.woodenDoorOpen===String(view.door),'Canvas door state diverged');
        const scene=H.expectedScene(view.map,view.position,view.key);
        const trim=entries=>entries.map(({id,key,position})=>({id,key,position}));
        insist(same(trim(s.scene),scene) && same(trim(JSON.parse($('room').dataset.scene)),scene),'Source scene membership/painter order differs');
        const canvas=$('room');insist(canvas.width===256 && canvas.height===224,'Room canvas dimensions differ');
        const counts=H.compareCanvas({actual:canvas.getContext('2d').getImageData(0,0,256,224).data,
          ...H.compositionInputs(art,backgrounds,masks,view),sprites:scene.map(e=>({...e,frame:art.frames[e.key]}))});
        run.visualChecks++;run.arkPixels=(run.arkPixels || 0)+counts.ark.pixels;
        if (s.map_id===10 && s.phase==='walking') { checkExterior(s);run.exteriorArkPixels=(run.exteriorArkPixels || 0)+counts.ark.pixels; }
        if (!s.dialogue) { insist($('dialogue-panel').hidden,'Stale visible dialogue panel');return; }
        insist(!$('dialogue-panel').hidden,'Dialogue panel hidden');
        const check=(id,key)=> {
          const canvas=$(id),frame=art.dialogue_pages[key];insist(canvas.dataset.key===key,'Wrong canvas source key');
          const ink=compareRaster(canvas.getContext('2d').getImageData(0,0,canvas.width,canvas.height),frame);
          run.rasterEvidence[key]=(run.rasterEvidence[key] || 0)+ink;
        };
        check('dialogue-page',s.dialogue.key);
        const choice=s.dialogue.choice ?? null;
        insist(['pause','step','interact'].every(id=>$(id).disabled) && [...document.querySelectorAll('[data-direction]')].every(b=>b.disabled),'Dialogue controls not locked');
        insist($('continue').hidden===(choice!==null) && $('continue').disabled===(choice!==null) && $('continue').textContent.trim()==='Continue (Space / Enter)','None tail exposed Continue or page lacks Continue');
        insist($('dialogue-choices').hidden===(choice===null),'Choice visibility differs');
        for (const id of ['choice1','choice2','choice-cancel']) insist($(id).disabled===(choice===null),'Choice control availability differs');
        if (choice!==null) for (let result=1;result<=2;result++) check(`choice${result}-label`,`choice:${choice}:${result}`);
      }
      pixels(state);
      async function step(command) {
        const previous=state;
        await new Promise((resolve,reject)=> {
          let observer,timer;
          const done=error=> {observer?.disconnect();clearTimeout(timer);try {pause();} catch(e) {error ||= e;} error?reject(error):resolve();};
          observer=new MutationObserver(()=> {
            try {insist(!$('error').textContent,$('error').textContent);if(latch.observe(Number($('tick').textContent))) {run.lastTick=latch.lastTick;done();}}
            catch(error) {done(error);}
          });
          observer.observe($('tick'),{childList:true});
          timer=setTimeout(()=>done(new Error(`UI command ${command} ack timeout at ${latch.lastTick}`)),5000);
          try {
            latch.arm();
            controls.send(command,previous);
          } catch(error) {done(error);}
        });
        // Every command is one paced actual UI callback, paused before GET/render
        // observations. Manual dialogue callbacks never Resume until page end.
        controls.release();
        state=await get('/state');insist(state.tick===latch.lastTick,'Host/UI tick diverged');pixels(state);
      }
      const checkpoint=(label,map,x,y)=> {
        insist(same([state.map_id,state.x,state.y,state.phase,state.dialogue],[map,x,y,'walking',null]),`${label}: wrong route endpoint ${JSON.stringify(state)}`);
        run.checkpoints.push({label,state:structuredClone(state)});
      };
      async function motion(chunks,granted) {
        for(const [command,steps] of chunks) for(let i=0;i<steps;i++) {
          await step(command);checkEvents(state,granted);insist(state.dialogue===null,'Unexpected dialogue during motion');
        }
      }
      await motion(run.motion.toB,false);checkpoint('B-entry',11,120,191);
      await motion(run.motion.target,false);checkpoint('resident-Up-target',11,120,128);
      // Re-press Up into the source resident collision boundary is part of the
      // verified target recipe, not a position injection or radial NPC admission.
      for (const expected of conversationPlan()) {
        await step(expected.command);checkConversationState(state,expected);
        run.checkpoints.push({label:expected.key || 'followup-closed',state:structuredClone(state)});
        if (state.dialogue) {
          const saved=structuredClone(state);
          // Wrong inputs and no-input time cannot move Ark, grant, ack or choose.
          for (const direction of ['ArrowRight','ArrowDown','b','x']) {key('keydown',direction);key('keyup',direction);}
          if (expected.choice===null) $('choice-cancel').click();else $('continue').click();
          await sleep(100);state=await get('/state');
          insist(same(state,saved),'Locked dialogue changed on wrong input/no ack');pixels(state);
        }
      }
      await motion(run.motion.toD,true);checkpoint('D-reloaded-after-grant',13,120,625);
      await motion(run.motion.toA,true);checkpoint('landed-A',10,504,769);
      await motion(run.motion.down,true);checkpoint('exterior-down-settled',10,504,815);
      await motion(run.motion.right,true);checkpoint('exterior-right-settled',10,538,815);
      insist(run.arkPixels>0 && run.exteriorArkPixels>0,'Missing nonvacuous Ark composition evidence');
      run.result={kind:'real-ui-conversation-source-composition',initial:run.initial,final:state,checkpoints:run.checkpoints,
        visualChecks:run.visualChecks,rasterEvidence:run.rasterEvidence,motion:run.motion,
        limits:'Source composition only, not native whole RGB outside: no secondary BG/color math, shadows, transient overlays or outdoor NPCs. Fourteen page metadata/raster contracts validated; actual pixels only for visited pages and both choice catalogs. Entry and first cancel/option2 pages are source-only, not replayed. Halo checks observe player envelope, not backend collision probe admission. Default motion recipe passed two identical live1671-step runs; logical callbacks are not native scheduler frames.'};
      run.status='passed';return run.result;
    } catch(error) {
      try {controls.release();pause();} catch (_) { /* retain the original failure */ }
      run.status='failed';run.error=`Tick ${run.lastTick}: ${error.stack || error}`;return {error:run.error};
    }
  })();
  return 'Conversation verifier started; inspect CONVERSATION_BROWSER_RUN (promise retained, no CLI long await).';
})()
