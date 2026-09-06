// Real Canvas2D / full-RGBA oracle using ROM-authenticated record-zero rasters.
// State samples and diagnostic BG are synthetic, not a native journey witness.
'use strict';
const fs=require('node:fs');
const {fixture,stateFor}=require('./pandora-carry-check.js');
function browserPixels(api, fixture, stateFor, source) {
  const {bundle,pixels,state:initial}=fixture();
  // Only the test's diagnostic background/masks are synthetic. All foreground
  // actors, offsets, mirrors, carry pairs and resident phases come from ROM art.
  bundle.frames=source.frames;
  bundle.pandora_carry=source.pandora_carry;
  bundle.pandora_scenes=source.pandora_scenes;
  const phase=bundle.pandora_scenes['c-direct'];
  const initialScene=phase.actors.map(a=>({id:a.id,key:a.key,position:a.position,priority:a.priority}));
  const state={...initial,actor_key:'0:0',scene:[...initialScene,{id:'ark',key:'0:0',position:[184,368],priority:2}]};
  // stateFor only changes Ark. Sort supplied source scenes independently below.
  const highAt=(x,y)=>x>=32&&x<256&&y>=300&&y<460&&(x+2*y)%5===0;
  const backgroundAt=(x,y)=>[30+x%61,40+y%53,50+(x+y)%47,255];
  const mask=bundle.foreground[12]; mask.runs=[];
  const data=pixels.house.data;
  for(let y=0;y<1024;y++) for(let x=0;x<512;x++) {
    const p=y*512+x;data.set(backgroundAt(x,y),p*4);
    if(highAt(x,y)) mask.runs.push(p,1);
  }
  const make=(width,height,rgba)=> {
    const c=document.createElement('canvas');c.width=width;c.height=height;
    c.getContext('2d').putImageData(new ImageData(new Uint8ClampedArray(rgba),width,height),0,0);return c;
  };
  const art=api.prepareArt(bundle,pixels,make), canvas=make(256,224,new Uint8ClampedArray(256*224*4)),ctx=canvas.getContext('2d');
  const order=(a,b)=>a.position[1]-b.position[1]||a.tie_rank-b.tie_rank;
  const rank=e=>e.id==='ark'?phase.ark_tie_rank:phase.actors.find(a=>a.id===e.id).tie_rank;
  const cases=[];
  for(const pose of Object.keys(bundle.pandora_carry.poses)) for(const slot of [2442,2447]) {
    const s=stateFor(bundle,state,pose,slot);
    // Put Ark over source NPCs to exercise ordinary world-Y/equal-Y ties.
    if(!pose.startsWith('throwing:')) {
      s.x=152;s.y=368;s.scene=s.scene.map(e=>e.id==='ark'?{...e,position:[s.x,s.y]}:e);
    }
    cases.push(s);
  }
  for(const x of [136,184]) for(const slot of [2442,2447]) for(let tick=0;tick<32;tick++) cases.push(stateFor(bundle,state,'throwing:1',slot,tick,x));
  let visiblePotPixels=0,hiddenWinnerPixels=0,fullCanvasChecks=0,negativeControls=0;
  const kinds=new Set(),mirrors=new Set(),visibleByKindAndMode={};
  let potNpcDepthPixels=0;
  const colorAt=(actor,wx,wy)=> {
    const f=bundle.frames[actor.key],ax=wx-actor.position[0]-f.offset[0],ay=wy-actor.position[1]-f.offset[1];
    if(ax<0||ay<0||ax>=f.width||ay>=f.height) return null;
    const p=(ay*f.width+ax)*4;
    return f.rgba[p+3] ? f.rgba.slice(p,p+4) : null;
  };
  for(const s of cases) {
    s.scene.sort((a,b)=>order({...a,tie_rank:rank(a)},{...b,tie_rank:rank(b)}));
    const expectedActors=[...phase.actors,{id:'ark',key:s.actor_key,position:[s.x,s.y],priority:2,tie_rank:phase.ark_tie_rank}];
    const c=s.carry;
    if(c.reserved_slot!==null) {
      const flying=c.phase_tick>=18&&c.pose==='throwing:1';
      const key=flying?bundle.pandora_carry.flight[c.reserved_slot]:bundle.pandora_carry.poses[c.pose].pots[c.reserved_slot];
      expectedActors.push({id:'expected-pot',key,position:flying?c.flight:[s.x,s.y],priority:2,tie_rank:phase.ark_tie_rank+1});
      kinds.add(c.reserved_slot);mirrors.add(key.endsWith(':1'));
    }
    expectedActors.sort(order);
    // Independent destination-pixel oracle. First resolve the LAST opaque OBJ,
    // then compare its priority with diagnostic high BG. Never mask actors first.
    const expected=new Uint8ClampedArray(256*224*4);
    let potVisible=0;
    const depthIndices=[], frontToBack=[...expectedActors].reverse();
    for(let y=0;y<224;y++) for(let x=0;x<256;x++) {
      const wx=x+s.camera[0],wy=y+s.camera[1], at=(y*256+x)*4;
      let color=backgroundAt(wx,wy);
      for(const actor of frontToBack) {
        const rgba=colorAt(actor,wx,wy);
        if(!rgba) continue;
        if(actor.priority===3||!highAt(wx,wy)) {
          color=rgba;
          if(actor.id==='expected-pot') {
            potVisible++;
            // Resolve the actual replacement winner after moving the pot behind
            // residents. Exclude Ark and require an equal-Y, different-color NPC.
            const replacement=frontToBack.find(a=>a!==actor&&colorAt(a,wx,wy));
            if(replacement&&replacement.id!=='ark'&&replacement.position[1]===actor.position[1]&&
                !colorAt(expectedActors.find(a=>a.id==='ark'),wx,wy)&&
                colorAt(replacement,wx,wy).some((v,i)=>v!==rgba[i])) depthIndices.push(at);
          }
        } else hiddenWinnerPixels++;
        break;
      }
      expected.set(color,at);
    }
    const sprites=api.selectActors(art,s),bg=api.selectBackground(art,s);
    if(sprites.length!==expectedActors.length || sprites.some(a=>a.flipX||a.priority!==2)) throw new Error('Typed carry count/mirror/priority mismatch');
    api.drawScene(ctx,bg.image,s,sprites,bg.foreground,bg.high);
    const actual=ctx.getImageData(0,0,256,224).data;
    for(let i=0;i<actual.length;i++) if(actual[i]!==expected[i]) throw new Error(`Carry RGBA mismatch ${c.pose}/${c.reserved_slot}/${c.phase_tick} byte${i}: ${actual[i]} != ${expected[i]}`);
    fullCanvasChecks++;visiblePotPixels+=potVisible;potNpcDepthPixels+=depthIndices.length;
    if(c.reserved_slot!==null) {
      const category=`${c.reserved_slot}:${c.flight===null?'hand':'flight'}`;
      visibleByKindAndMode[category]=(visibleByKindAndMode[category]||0)+potVisible;
    }
    // Missing overlay, extra Y, double mirror, wrong kind and reversed pot/NPC
    // ties must each make the independent comparison fail on visible source art.
    if(potVisible>0&&c.pose==='standing:2'&&c.reserved_slot===2442) {
      if(depthIndices.length===0) throw new Error('No discriminating pot/NPC tie pixels');
      const index=expectedActors.findIndex(a=>a.id==='expected-pot');
      const wrongKind=art.actors[bundle.pandora_carry.poses[c.pose].pots[2447]];
      for(const [control,mutate] of [
        a=>a.filter((_,i)=>i!==index),
        a=>a.map((v,i)=>i===index?{...v,position:[v.position[0],v.position[1]-16]}:v),
        a=>a.map((v,i)=>i===index?{...v,flipX:true}:v),
        a=>a.map((v,i)=>i===index?{...v,...wrongKind}:v),
        a=>[a[index],...a.filter((_,i)=>i!==index)]
      ].entries()) {
        api.drawScene(ctx,bg.image,s,mutate(sprites),bg.foreground,bg.high);
        const bad=ctx.getImageData(0,0,256,224).data;
        if(bad.every((v,i)=>v===expected[i])) throw new Error('Vacuous carry pixel negative control');
        if(control===4&&!depthIndices.some(at=>[0,1,2,3].some(i=>bad[at+i]!==expected[at+i]))) throw new Error('Vacuous NPC-only depth control');
        negativeControls++;
      }
    }
  }
  if(visiblePotPixels<1000||hiddenWinnerPixels<1000||kinds.size!==2||mirrors.size!==2||negativeControls!==5||potNpcDepthPixels===0||
      Object.keys(visibleByKindAndMode).length!==4||Object.values(visibleByKindAndMode).some(n=>n===0)) throw new Error('Insufficient carry pixel coverage');
  return {status:'passed',fullCanvasChecks,pixelsPerCheck:256*224,visiblePotPixels,visibleByKindAndMode,potNpcDepthPixels,hiddenWinnerPixels,negativeControls,
    evidence:'ROM source rasters on diagnostic BG; synthetic finite samples, not native timing/live GameState journey'};
}
if(require.main===module) {
  if(!process.env.PANDORA_CARRY_EXPORT) throw new Error('Set PANDORA_CARRY_EXPORT to the authenticated host test art.json export');
  const source=JSON.parse(fs.readFileSync(process.env.PANDORA_CARRY_EXPORT,'utf8'));
  // Retain only records referenced by this test; no captures or state files.
  const keys=new Set(['0:0']);
  for(const p of Object.values(source.pandora_scenes)) for(const a of p.actors) keys.add(a.key);
  for(const p of Object.values(source.pandora_carry.poses)) for(const key of [p.ark,...Object.values(p.pots)]) keys.add(key);
  for(const key of Object.values(source.pandora_carry.flight)) keys.add(key);
  const subset={frames:Object.fromEntries([...keys].map(k=>[k,source.frames[k]])),pandora_carry:source.pandora_carry,pandora_scenes:source.pandora_scenes};
  console.log(`(()=>{const baseFixture=${require('./pandora-render-check.js').fixture};return (${browserPixels})(globalThis.RoomSlice,${fixture},${stateFor},${JSON.stringify(subset)});})()`);
}
