// Input-only Pandora acceptance. See docs/pandora-browser.md for pinned inputs.
// Returns immediately; PANDORA_BROWSER_RUN retains status/result/error/promise.
(() => {
  'use strict';
  const insist=(ok,message)=>{if(!ok)throw new Error(message);};
  const canonical=value=>JSON.stringify(value,(_,v)=>v && typeof v==='object' && !Array.isArray(v)
    ? Object.fromEntries(Object.keys(v).sort().map(k=>[k,v[k]])):v);
  const same=(a,b)=>canonical(a)===canonical(b);
  const withoutTick=({tick,...state})=>state;
  const crossClock=({tick,snapshot_sha256,...state})=>state;
  const hash=value=>typeof value==='string' && /^[a-f0-9]{64}$/.test(value);
  const ROUTE_SHA='b969d6877f595ff811a0de8a912de307aaf5a3eb2b42e1720f2f5da6db53b830';
  const TEXT_SHA='9df184146df8664e06bc4da68bf3a453958337da6809a59f73b154f58b93e682';
  function checkState(actual,expected,tick) {
    insist(actual?.tick===tick && same(withoutTick(actual),withoutTick(expected)),`State differs at UI tick ${tick}`);
  }
  function blockingDialogue(state) {
    insist(state.dialogue_ready===undefined || typeof state.dialogue_ready==='boolean','Invalid dialogue readiness');
    return state.dialogue!=null && state.dialogue_ready!==false;
  }
  function checkDialogueControls($,state) {
    insist($('dialogue-panel').hidden===(state.dialogue===null),'Dialogue panel visibility differs');
    if(!state.dialogue)return;
    const blocking=blockingDialogue(state),choice=state.dialogue.choice??null;
    insist(['pause','step'].every(id=>$(id).disabled===blocking),'Dialogue Resume/neutral availability differs');
    insist(['interact','pot-action'].every(id=>$(id).disabled),'Dialogue manual controls not locked');
    insist($('continue').disabled===(!blocking || choice!==null) && $('continue').hidden===(choice!==null) &&
      $('dialogue-choices').hidden===(choice===null),'Wrong ack/choice controls');
    for(const id of ['choice1','choice2','choice-cancel'])insist($(id).disabled===(!blocking || choice===null),'Choice availability differs');
  }
  function projectRoute(route,proof) {
    insist(Array.isArray(route?.actions) && route.actions.length>0,'Invalid route');
    const commands=[];
    for(const row of route.actions) {
      insist(Array.isArray(row) && row.length===2 && Number.isInteger(row[0]) && row[0]>=0 && row[0]<=10 &&
        Number.isInteger(row[1]) && row[1]>0 && commands.length+row[1]<=100000,'Invalid route command/count');
      commands.push(...Array(row[1]).fill(row[0]));
    }
    insist(proof?.schema===1 && Array.isArray(proof.offline) && proof.offline.length===commands.length+1 &&
      Array.isArray(proof.projected),'Missing complete fresh replay proof');
    for(const [kind,records] of [['offline',proof.offline],['projected',proof.projected]]) for(const [tick,r] of records.entries()) {
      insist(r?.state?.tick===tick && hash(r.continuation) && Object.hasOwn(r.state,'dialogue'),`Invalid ${kind} continuation/tick ${tick}`);
      blockingDialogue(r.state); // Reject malformed readiness even on retained/manual states.
    }
    insist(same(proof.offline[0],proof.projected[0]),'Fresh replay initial state differs');
    const steps=[],omissions=[];
    for(const [i,command] of commands.entries()) {
      const before=proof.offline[i],after=proof.offline[i+1],offlineTick=i+1;
      if(command<=4 && blockingDialogue(before.state)) {
        insist(before.continuation===after.continuation && same(crossClock(before.state),crossClock(after.state)),
          `Paused dialogue command ${command} at offline ${offlineTick} is not a proved no-op; require fresh UI route`);
        omissions.push({command,offlineTick,continuation:before.continuation});
      } else {
        const replay=proof.projected[steps.length+1];
        insist(replay && replay.continuation===after.continuation,'Projected fresh continuation differs');
        insist(replay.state.tick===steps.length+1 && same(crossClock(replay.state),crossClock(after.state)),'Projected fresh observations differ');
        steps.push({command,offlineTick,state:replay.state});
      }
    }
    insist(proof.projected.length===steps.length+1,'Extra/missing projected actions');
    return {initial:proof.projected[0].state,steps,omissions,offlineTicks:commands.length};
  }
  function createCommands($,motion) {
    const click=id=>{const b=$(id);insist(b && !b.disabled && !b.hidden,`Unavailable actual ${id} button`);b.click();};
    return {click,stop:()=>motion.stop(),send(command,state) {
      insist(Number.isInteger(command) && command>=0 && command<=10,'Invalid UI command');
      insist($('pause').textContent==='Resume','UI command must start paused');
      if(command<=4) {
        insist(!blockingDialogue(state),'Cannot advance paused dialogue');
        motion.resume();motion.apply(command,{map:state.map_id,phase:state.phase});
      } else {
        if(command>=6 && command<=9)insist(blockingDialogue(state),'Cannot acknowledge/choose unready dialogue');
        insist(command!==10 || state.dialogue===null,'Pot input during dialogue');
        click(['interact','continue','choice-cancel','choice1','choice2','pot-action'][command-5]);
      }
    }};
  }
  // Invert the documented preview palette to compare source indexed-byte hashes,
  // not just a host raster to itself. Background index is per source page.
  function indexedPage(frame,ref) {
    insist(frame?.width===ref.width && frame.height===ref.height && Array.isArray(frame.rgba) &&
      frame.rgba.length===frame.width*frame.height*4,'Invalid text dimensions');
    const palette=[[0,0,0,255],[240,244,248,255],[36,47,55,255],[0,0,0,0]];
    palette[ref.background_index]=[0,0,0,0];
    const result=new Uint8Array(frame.width*frame.height);
    for(let p=0;p<result.length;p++) {
      const color=frame.rgba.slice(p*4,p*4+4),index=palette.findIndex(v=>same(v,color));
      insist(index>=0,'Unknown source text palette');
      // Index3 is intrinsically transparent; native pages use background3 or
      // background0. Resolve transparent pixels to the declared background.
      result[p]=color[3]===0?ref.background_index:index;
    }
    return result;
  }
  // Independent bounded mode09 compositor. Shared Render helpers validate/select
  // source roster, typed carry and patches; this does not call their drawing code.
  function compareComposition({actual,background,base,high,camera,sprites,patches,width=256,height=224}) {
    insist(actual?.length===width*height*4 && background?.data.length===background.width*background.height*4 &&
      base?.data.length===background.data.length && high?.data.length===background.data.length,'Invalid composition dimensions');
    const evidence={background:0,sprites:{},patches:{},colors:new Set()};
    const patchCells=new Set(patches.map(p=>p.cell));
    for(let y=0;y<height;y++)for(let x=0;x<width;x++) {
      const wx=x+camera[0],wy=y+camera[1],world=wy*background.width+wx;
      insist(wx>=0 && wy>=0 && wx<background.width && wy<background.height,'Camera outside sheet');
      let winner=null,at=world*4,pixels=background.data;
      for(const s of sprites) {
        insist([2,3].includes(s.priority) && s.flipX===false,'Unsupported OBJ policy');
        const sx=wx-s.position[0]-s.offset[0],sy=wy-s.position[1]-s.offset[1],w=s.source[2],h=s.source[3];
        if(sx>=0 && sy>=0 && sx<w && sy<h && s.rgba[(sy*w+sx)*4+3]===255)winner={s,at:(sy*w+sx)*4};
      }
      if(winner && !(winner.s.priority===2 && high.data[world*4+3])) {
        pixels=winner.s.rgba;at=winner.at;const id=winner.s.id;
        evidence.sprites[id]=(evidence.sprites[id]||0)+1;
      } else {
        evidence.background++;evidence.colors.add(Array.from(pixels.slice(at,at+4)).join(','));
        const cell=Math.floor(wy/16)*(background.width/16)+Math.floor(wx/16);
        if(patchCells.has(cell) && pixels.slice(at,at+4).some((v,c)=>v!==base.data[world*4+c]))evidence.patches[cell]=(evidence.patches[cell]||0)+1;
      }
      for(let c=0;c<4;c++)insist(actual[(y*width+x)*4+c]===pixels[at+c],`Canvas pixel ${x},${y} channel ${c} differs`);
    }
    return evidence;
  }
  const GRANTS=[[965,0x26],[3187,0x28],[5706,0x27],[5778,0x2e],[9189,0x292],[10130,0x22],[11218,0x243],[11222,0x244]];
  const CHECKPOINTS={1701:[10,538,815],2617:[10,472,304],2710:[19,392,207],3187:[19,360,144],4333:[10,472,400],4517:[10,504,400],
    5520:[10,504,768],10078:[33,136,360],10087:[33,136,359],10129:[33,136,368],11222:[65,136,208],11314:[65,120,208],
    11406:[65,120,192],11498:[65,136,192],11590:[65,136,208]};
  function checkSemanticCheckpoint(tick,s) {
    // Independently retained source-qualification boundaries, not host-derived flags.
    for(const [grant,bit] of GRANTS)insist(s.events.includes(bit)===(tick>=grant),`Grant ${bit.toString(16)} differs at offline ${tick}`);
    if(CHECKPOINTS[tick])insist(same([s.map_id,s.x,s.y],CHECKPOINTS[tick]),`Semantic checkpoint ${tick} differs`);
    if(tick>=11222)insist(s.owner==='player' && s.phase==='walking' && s.dialogue===null,'Final movement did not retain player control');
  }
  function requireCoverage(e) {
    for(const key of ['house','exterior','town13','cellars','box','tour'])insist(e.backgrounds[key]>0,`Missing background pixels ${key}`);
    // Source atlas tests and docs/pandora-pots.md: wooden, damaged/open cellar,
    // three consumed cells. Count actual changed, unoccluded pixels, not metadata.
    for(const key of ['house:616:246','house:648:247','house:651:423','house:651:246','house:683:203',
      'house:675:248','house:676:248','house:677:248'])insist(e.patches[key]>0,`Missing changed patch pixels ${key}`);
    for(const key of ['lifting','standing','walking','throwing','flight-miss','flight-hit'])insist(e.carry[key]>0,`Missing carry pixels ${key}`);
    insist(e.ark>0,'Missing Ark pixels');
  }
  function bounded(value,label,ms=5000) {
    let timer;
    return Promise.race([
      Promise.resolve(value),
      new Promise((_,reject)=>{timer=setTimeout(()=>reject(new Error(`${label} timeout`)),ms);}),
    ]).finally(()=>clearTimeout(timer));
  }
  function createInspection({runtime,localRequested=runtime!==undefined,fetchFn=globalThis.fetch,timeout=ms=>AbortSignal.timeout(ms),deadlineMs=5000}) {
    if(localRequested) {
      insist(runtime && ['request','loadArt','loadBackground'].every(name=>typeof runtime[name]==='function'),'Requested browser-local Wasm runtime is unavailable');
      return Object.freeze({
        kind:'browser-local-wasm-worker',
        state:()=>bounded(runtime.request('/state'),'Wasm state inspection',deadlineMs),
        async art(){return JSON.parse(new TextDecoder().decode(await bounded(runtime.loadArt(),'Wasm art inspection',deadlineMs)));},
        background:path=>bounded(runtime.loadBackground(path),'Wasm background inspection',deadlineMs),
      });
    }
    const get=async path=>{const r=await fetchFn(path,{cache:'no-store',signal:timeout(5000)});insist(r.ok,`GET ${path}: ${r.status}`);return r.json();};
    return Object.freeze({kind:'native-http',state:()=>get('/state'),art:()=>get('/art.json'),background:path=>path});
  }
  function checkLocalBootstrap(preview,$,state) {
    insist(preview?.status==='ready','Browser-local Wasm bootstrap is not ready');
    insist($('local-rom-status')?.dataset.kind==='ready','Local ROM status is not ready');
    insist(!$('error')?.textContent,'Local ROM bootstrap has a UI error');
    insist($('new-game') && !$('new-game').disabled,'Local New Game control is unavailable');
    insist(state?.start_kind==='saved-checkpoint' && state.tick===0 && state.map_id===15 && state.x===472 && state.y===176,
      'Local ROM did not construct the authenticated saved checkpoint');
  }
  const helpers={projectRoute,checkState,createCommands,bounded,createInspection,checkLocalBootstrap,indexedPage,compareComposition,requireCoverage,checkSemanticCheckpoint,blockingDialogue,checkDialogueControls};
  if(typeof module!=='undefined' && module.exports && typeof document==='undefined'){module.exports=helpers;return;}
  globalThis.PandoraBrowserHelpers=helpers;
  if(globalThis.PANDORA_BROWSER_HELPERS_ONLY)return 'PandoraBrowserHelpers ready (no input)';
  insist(globalThis.PANDORA_BROWSER_READY===location.origin && /^http:\/\/(127\.0\.0\.1|localhost):\d+$/.test(location.origin) && location.port!=='8765',
    'Explicit isolated loopback origin required; live8765 forbidden');
  insist(globalThis.HouseBrowserHelpers && globalThis.ConversationBrowserHelpers && globalThis.RoomSlice,'Load house/conversation helpers-only on the real room page');
  insist(globalThis.PANDORA_BROWSER_RUN?.status!=='running','Pandora verifier already running');
  const run=globalThis.PANDORA_BROWSER_RUN={status:'running',lastTick:0,offlineTick:0,checkpoints:[],visualChecks:0,
    evidence:{backgrounds:{},patches:{},carry:{},ark:0},invocations:[],rasterEvidence:{}};
  run.promise=(async()=>{
    const H=globalThis.HouseBrowserHelpers,C=globalThis.ConversationBrowserHelpers,R=globalThis.RoomSlice,$=id=>document.getElementById(id);
    const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
    const digest=async bytes=>Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',bytes)),b=>b.toString(16).padStart(2,'0')).join('');
    const pinned=async(text,sha,label)=>{insist(typeof text==='string' && hash(sha) && await digest(new TextEncoder().encode(text))===sha,`${label} pin differs`);return JSON.parse(text);};
    const key=(type,key)=>document.dispatchEvent(new KeyboardEvent(type,{key,bubbles:true}));
    let controls,started=false;
    try {
      controls=createCommands($,H.createControls($,key));
      const route=await pinned(globalThis.PANDORA_BROWSER_ROUTE_TEXT,ROUTE_SHA,'fixed route');
      const reference=await pinned(globalThis.PANDORA_BROWSER_TEXT_REFERENCE,TEXT_SHA,'source text reference');
      const proof=await pinned(globalThis.PANDORA_BROWSER_PROOF_TEXT,globalThis.PANDORA_BROWSER_PROOF_SHA,'reviewed fresh replay proof');
      insist(proof.provenance?.route_sha256===ROUTE_SHA && proof.provenance.rom_sha256===route.rom_sha256 &&
        proof.provenance.start==='NewGame' && proof.provenance.policy==='SemanticPreview' && proof.provenance.tick_erasure==='global-tick-only',
        'Missing source fresh replay provenance');
      const plan=projectRoute(route,proof);run.projection={offlineTicks:plan.offlineTicks,uiTicks:plan.steps.length,omissions:plan.omissions,proofSha:globalThis.PANDORA_BROWSER_PROOF_SHA};
      insist(plan.offlineTicks===11590,'Fixed route action count differs');
      const localRuntimeRequested=globalThis.RoomSliceRuntime!==undefined;
      const runtimeModule=localRuntimeRequested?await bounded(globalThis.RoomSliceRuntime.ready,'Wasm runtime readiness'):null;
      const inspection=createInspection({runtime:runtimeModule?.runtime,localRequested:localRuntimeRequested});
      const bundle=await inspection.art(),before=await inspection.state();
      if(inspection.kind==='browser-local-wasm-worker')checkLocalBootstrap(globalThis.PandoraLocalPreview,$,before);
      run.transport=inspection.kind;
      run.bootstrap=inspection.kind==='browser-local-wasm-worker'?structuredClone(globalThis.PandoraLocalPreview):null;
      insist(before.pot_action===true && before.choice_interaction===true && before.dialogue_acknowledgement===true && before.world_background &&
        typeof before.owner==='string' && typeof before.dialogue_ready==='boolean','Enabled host contract is not ready');
      // Reuse the existing independent house/source contracts on their exact
      // subset; expanded Pandora frames/resources must not weaken the prefix.
      H.validateArt({...bundle,frames:Object.fromEntries(Object.entries(bundle.frames).filter(([k])=>!k.startsWith('pandora:')))});
      C.validateDialogueArt({...bundle,
        dialogue_requests:Object.fromEntries(Object.entries(bundle.dialogue_requests).filter(([k])=>/^88(8f|90|91)/.test(k))),
        dialogue_pages:Object.fromEntries(Object.entries(bundle.dialogue_pages).filter(([k])=>/^(text:88(8f|90|91)|choice:[01]:)/.test(k)))});
      // Validate every direct/retry source page metadata and indexed raster hash.
      for(const p of reference.pages) {
        const source=p.text_source.toString(16),key=`text:${source}:${p.index}`,m=bundle.dialogue_requests?.[source]?.[p.index];
        for(const [field,value] of Object.entries({key,page_id:p.page_id,boundary_source:p.boundary_source,glyph_count:p.glyph_count,acknowledgement:p.acknowledgement}))
          insist(m?.[field]===value,`Source metadata ${key} ${field} differs`);
        insist(await digest(indexedPage(bundle.dialogue_pages?.[key],p))===p.sha256,`Source raster ${key} differs`);
      }
      const make=(width,height,data)=>({width,height,data:new Uint8ClampedArray(data)});
      const update=(image,x,y,w,h,data)=>{for(let row=0;row<h;row++)image.data.set(data.subarray(row*w*4,(row+1)*w*4),((y+row)*image.width+x)*4);};
      const bases=Object.fromEntries(await Promise.all(Object.entries(R.backgroundManifest(bundle)).map(async([name,d])=>{
        const source=await inspection.background(d.url),local=inspection.kind==='browser-local-wasm-worker';
        insist(!local || source instanceof Blob,`Wasm background ${name} is not an owned Blob`);
        const objectUrl=local?URL.createObjectURL(source):null,image=new Image();
        try {
          await bounded(new Promise((resolve,reject)=>{image.onload=resolve;image.onerror=()=>reject(new Error(`Bitmap ${name}`));image.src=objectUrl??source;}),`Bitmap ${name} decode`);
          insist(image.width===d.width && image.height===d.height,`Source bitmap size ${name}`);
          const canvas=document.createElement('canvas');canvas.width=d.width;canvas.height=d.height;
          const ctx=canvas.getContext('2d');ctx.drawImage(image,0,0);return [name,ctx.getImageData(0,0,d.width,d.height)];
        } finally {if(objectUrl!==null)URL.revokeObjectURL(objectUrl);}
      })));
      const art=R.prepareArt(bundle,bases,make,update),backgroundColors={};
      started=true;controls.click('new-game');
      const deadline=performance.now()+5000;
      while($('position').textContent!=='304, 112' || $('tick').textContent!=='0' || $('pause').disabled || $('step').disabled) {
        insist(performance.now()<deadline,'New Game startup timeout');await sleep(10);
      }
      let state=await inspection.state();
      insist(state.start_kind==='new-game' && state.tick===0 && state.map_id===15 && state.x===304 && state.y===112 && state.dialogue===null && state.owner==='player','Not a fresh New Game');
      checkState(state,plan.initial,0);run.initial=state;
      function raster(id,frame,key) {
        const canvas=$(id);insist(canvas.dataset.key===key && canvas.width===frame.width && canvas.height===frame.height,`Wrong rendered ${id}`);
        const ink=C.compareRaster(canvas.getContext('2d').getImageData(0,0,canvas.width,canvas.height),frame);
        run.rasterEvidence[key]=(run.rasterEvidence[key]||0)+ink;
      }
      function pixels(s) {
        insist(s.error===null && !$('error').textContent,`Host/UI error ${s.error || $('error').textContent}`);
        insist(Number($('tick').textContent)===s.tick && $('map').textContent===`$${s.map_id.toString(16).toUpperCase().padStart(4,'0')}` &&
          $('position').textContent===`${s.x}, ${s.y}` && $('camera').textContent===s.camera.join(', ') && $('phase').textContent===s.phase &&
          $('actor-key').textContent===s.actor_key,'DOM/GET inspector divergence');
        insist(same(JSON.parse($('room').dataset.scene),s.scene),'Canvas scene/GET divergence');
        if(s.scene_phase===undefined)insist(same(s.scene.map(({id,key,position})=>({id,key,position})),H.expectedScene(s.map_id,[s.x,s.y],s.actor_key)),
          'Source house roster/order differs');
        const bg=R.selectBackground(art,s),selected=R.selectActors(art,s);
        const carryKeys=s.carry && art.pandoraCarry;
        // Match selected raster identity to immutable art, not an inferred object
        // in state.scene (the typed carry overlay is intentionally absent there).
        const sprites=selected.map(a=>({...a,id:a.image===art.actors[s.actor_key].image?'ark':
          s.scene.some(e=>e.id!=='ark' && art.actors[e.key].image===a.image)?'resident':'carry'}));
        const canvas=$('room');insist(canvas.width===256 && canvas.height===224,'Room canvas size differs');
        const e=compareComposition({actual:canvas.getContext('2d').getImageData(0,0,256,224).data,background:bg.image,
          base:bases[s.world_background.key],high:bg.high,camera:s.camera,sprites,patches:s.world_background.patches});
        const name=s.world_background.key,colors=backgroundColors[name] ||= new Set();for(const color of e.colors)colors.add(color);
        if(colors.size>1)run.evidence.backgrounds[name]=(run.evidence.backgrounds[name]||0)+e.background;
        run.evidence.ark+=e.sprites.ark||0;
        for(const p of s.world_background.patches) {const k=`${name}:${p.cell}:${p.tile}`;run.evidence.patches[k]=(run.evidence.patches[k]||0)+(e.patches[p.cell]||0);}
        if(carryKeys && e.sprites.carry) {const k=s.carry.flight?(s.x===136?'flight-miss':'flight-hit'):s.carry.pose.split(':')[0];run.evidence.carry[k]=(run.evidence.carry[k]||0)+e.sprites.carry;}
        checkDialogueControls($,s);
        if(s.dialogue) {
          raster('dialogue-page',bundle.dialogue_pages[s.dialogue.key],s.dialogue.key);
          const choice=s.dialogue.choice??null;
          if(choice!==null)R.selectChoices(art,s).forEach((option,i)=>raster(`choice${i+1}-label`,bundle.dialogue_pages[option.key],option.key));
        }
        run.visualChecks++;
      }
      pixels(state);
      let previousRequest=null;
      function checkpoint(previous,s,step) {
        const request=s.dialogue?.key.split(':').slice(0,2).join(':')??null;
        if(request!==previousRequest && request && reference.invocations.some(i=>request===`text:${i.source.toString(16)}`))run.invocations.push(request);
        previousRequest=request;
        if(previous.map_id!==s.map_id || previous.scene_phase!==s.scene_phase || previous.owner!==s.owner || previous.dialogue_ready!==s.dialogue_ready || !same(previous.events,s.events) || !same(previous.dialogue,s.dialogue))
          run.checkpoints.push({offlineTick:step.offlineTick,state:structuredClone(s)});
      }
      const latch=C.createAckLatch(0);
      for(const step of plan.steps) {
        const previous=state;
        await new Promise((resolve,reject)=>{
          let observer,timer,done=false;
          const finish=error=>{if(done)return;done=true;observer?.disconnect();clearTimeout(timer);try{controls.stop();}catch(e){error ||= e;}error?reject(error):resolve();};
          observer=new MutationObserver(()=>{try {
            insist(!$('error').textContent,$('error').textContent);
            if(latch.observe(Number($('tick').textContent))){run.lastTick=latch.lastTick;finish();}
          }catch(error){finish(error);}});
          observer.observe($('tick'),{childList:true});
          timer=setTimeout(()=>finish(new Error(`UI ack timeout for command ${step.command}`)),5000);
          try{latch.arm();controls.send(step.command,previous);}catch(error){finish(error);}
        });
        state=await inspection.state();checkState(state,step.state,run.lastTick);checkSemanticCheckpoint(step.offlineTick,state);pixels(state);
        run.offlineTick=step.offlineTick;checkpoint(previous,state,step);
      }
      insist(same(run.invocations,reference.invocations.map(i=>`text:${i.source.toString(16)}`)),'Missing/reordered direct34 source invocations');
      requireCoverage(run.evidence);
      insist(state.map_id===65 && state.x===136 && state.y===208 && state.owner==='player' && state.phase==='walking' && state.dialogue===null &&
        [0x26,0x28,0x27,0x2e,0x292,0x22,0x243,0x244].every(bit=>state.events.includes(bit)),'Final41 control/flags differ');
      run.result={kind:inspection.kind==='browser-local-wasm-worker'?'real-ui-pandora-cadence-projection-browser-local-wasm':'real-ui-pandora-cadence-projection',transport:run.transport,bootstrap:run.bootstrap,initial:run.initial,final:state,projection:run.projection,checkpoints:run.checkpoints,
        invocations:run.invocations,evidence:run.evidence,visualChecks:run.visualChecks,rasterEvidence:run.rasterEvidence,
        limits:'Source-composition semantic preview, not native scheduler frames/whole RGB. Offline tick and snapshot identities differ. Retry/refusal optional branch not replayed. No equipment acquisition or world return claim.'};
      run.status='passed';return run.result;
    }catch(error){try{if(started)controls.stop();}catch(_){/* retain original failure */}run.status='failed';run.error=`UI ${run.lastTick}, offline ${run.offlineTick}: ${error.stack||error}`;return {error:run.error};}
  })();
  return 'Pandora verifier started; inspect PANDORA_BROWSER_RUN (promise retained; no CLI long await).';
})()
