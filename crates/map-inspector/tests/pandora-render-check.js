// Dependency-free source-scene admission and independent full-RGBA oracle.
'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const html = fs.readFileSync(path.join(__dirname,'../web/room-slice.html'),'utf8');
function fixture() {
  const backgrounds = {town13:{url:'/town13.bmp',width:1024,height:512},cellars:{url:'/cellars.bmp',width:512,height:1024},box:{url:'/box.bmp',width:256,height:512},tour:{url:'/tour.bmp',width:512,height:512}};
  const keys = {19:'town13',14:'cellars',32:'cellars',33:'box',65:'tour',66:'tour',67:'tour',68:'tour'};
  const bounds = {19:[256,0,512,256],14:[0,768,256,1024],32:[256,768,512,1024],33:[0,0,256,512],65:[0,0,256,256],66:[0,256,256,512],67:[256,0,512,256],68:[256,256,512,512]};
  const actor = {id:'pandora:839530',key:'pandora:83f8a8:3:0:0',art_id:0x83f8a8,selector:3,hflip:false,position:[136,96],priority:3,tie_rank:0};
  const bundle = {schema_version:1,backgrounds:{house:{url:'/map.bmp',width:512,height:1024},exterior:{url:'/exterior.bmp',width:1024,height:1280}},background_keys:{10:'exterior',12:'house',15:'house'},door_background:'house',
    foreground:{10:{width:1024,height:1280,runs:[]},12:{width:512,height:1024,runs:[]},15:{width:512,height:1024,runs:[]}},scene_ids:{10:['ark'],12:['ark'],15:['ark']},
    frames:{ark:{width:2,height:1,offset:[-1,-1],rgba:[200,10,20,255,0,0,0,0]},[actor.key]:{width:2,height:1,offset:[-1,-1],rgba:[10,200,20,255,30,40,50,255]}},
    pandora_scenes:{'tour-start':{map_id:65,ark_tie_rank:1,policy:'source-endpoints-not-script-timing',limits:['ScriptedMotion'],actors:[actor]},'c-departed':{map_id:12,ark_tie_rank:0,policy:'source-endpoints-not-script-timing',limits:['CellarColorMath'],actors:[]}},
    pandora_backgrounds:{backgrounds,background_keys:keys,foreground:Object.fromEntries(Object.entries(keys).map(([id,key])=>[id,{width:backgrounds[key].width,height:backgrounds[key].height,runs:[]}])),
      cameras:Object.fromEntries(Object.entries(bounds).map(([id,bounds])=>[id,{bounds,vertical_extent:256,hardware_background:id==='19'?2:1,bgmode:9,policy:'settled-source-clamp-not-transition-pan'}])),policy:'natural-palette-first-background-checker-transparency-no-phase-patches'}};
  bundle.pandora_scenes['cellar-e']={map_id:14,ark_tie_rank:0,policy:'source-endpoints-not-script-timing',limits:[],actors:[]};
  const pixels = Object.fromEntries(Object.entries({...bundle.backgrounds,...backgrounds}).map(([key,{width,height}])=>[key,{width,height,data:new Uint8ClampedArray(width*height*4)}]));
  // Synthetic opaque-high at a real actor overlap; adjacent pixel is transparent BG.
  pixels.tour.data.set([80,90,100,255],(95*512+135)*4);
  bundle.pandora_backgrounds.foreground[65].runs=[95*512+135,1];
  const state={map_id:65,scene_phase:'tour-start',actor_key:'ark',x:136,y:96,camera:[0,0],scene:[{id:actor.id,key:actor.key,position:actor.position,priority:3},{id:'ark',key:'ark',position:[136,96],priority:2}]};
  return {bundle,pixels,state};
}
function check(api) {
  const {prepareArt,selectActors,selectBackground,composeObjects,backgroundManifest}=api;
  const make=(width,height,data)=>({width,height,data});
  const {bundle,pixels,state}=fixture();
  assert.equal(Object.keys(backgroundManifest(bundle)).length,6);
  const art=prepareArt(bundle,pixels,make);
  const sprites=selectActors(art,state), bg=selectBackground(art,state);
  assert.equal(bg.image,art.backgrounds.tour);
  assert.deepEqual(Array.from(composeObjects(1,1,sprites,[135,95],bg.high)),[0,0,0,0], 'hidden winning OBJ2 must not expose rear OBJ3');
  for(const mutate of [b=>b.pandora_backgrounds.backgrounds.box.url='/box.bmp?x=1',b=>b.pandora_backgrounds.backgrounds.cellars.width=256,b=>b.pandora_backgrounds.background_keys[14]='house',b=>delete b.pandora_backgrounds.cameras[65],b=>b.pandora_backgrounds.cameras[65].bgmode=1,b=>b.pandora_backgrounds.cameras[19].hardware_background=1,b=>delete b.pandora_backgrounds.foreground[68],b=>b.pandora_backgrounds.backgrounds.evil=b.backgrounds.house,b=>b.pandora_scenes['tour-start'].actors[0].priority=1,b=>b.pandora_scenes['tour-start'].actors[0].key='missing']) {
    const bad=structuredClone(bundle); mutate(bad); assert.throws(()=>prepareArt(bad,pixels,make),/background|scene|priority|art/i);
  }
  for(const change of [{scene_phase:'unknown'},{scene_phase:null},{scene_phase:undefined},{map_id:66},{scene:state.scene.slice(1)},{scene:[...state.scene].reverse()}, {scene:state.scene.map(e=>({...e,priority:3}))}, {scene:state.scene.map(e=>e.id==='ark'?e:{...e,key:'ark'})}, {scene:state.scene.map(e=>({...e,position:[136,97]}))}]) {
    assert.throws(()=>selectActors(art,{...state,...change}),/scene|phase|priority/i);
  }
  assert.throws(()=>selectBackground(art,{...state,map_id:66}),/phase|scene/i);
  assert.throws(()=>prepareArt(bundle,{...pixels,tour:undefined},make),/background/i);
  const empty={...state,map_id:12,scene_phase:'c-departed',scene:[state.scene[1]]};
  assert.equal(selectActors(art,empty).length,1);
  assert.equal(selectBackground(art,{...state,map_id:14,scene_phase:'cellar-e',wooden_door_open:true}).image,art.backgrounds.cellars);
  // House-only with opt-in artifact remains on its old path without a phase/priority.
  assert.equal(selectActors(art,{...empty,map_id:15,scene_phase:undefined,scene:[{id:'ark',key:'ark',position:[136,96]}]}).length,1);

  // Independent destination-pixel oracle: search front-to-back for the FIRST
  // opaque OBJ, then compare its priority. It never paints/masks individual OBJs.
  const W=5,H=4, high={width:7,height:6,data:new Uint8ClampedArray(7*6*4)};
  for(let p=0;p<42;p++) if(p%3===0) high.data.set([70,80,90,255],p*4);
  function oracle(actors,camera) {
    const out=[];
    for(let y=0;y<H;y++) for(let x=0;x<W;x++) {
      let result=[0,0,0,0];
      for(const a of [...actors].reverse()) {
        const [,,w,h]=a.source;
        const localX=x+camera[0]-a.position[0];
        const ax=(a.flipX?-localX-1:localX)-a.offset[0], ay=y+camera[1]-a.position[1]-a.offset[1];
        if(ax<0||ay<0||ax>=w||ay>=h) continue;
        const color=Array.from(a.rgba.slice((ay*w+ax)*4,(ay*w+ax)*4+4));
        if(!color[3]) continue;
        const bx=x+camera[0],by=y+camera[1];
        if(a.priority===3 || bx<0||by<0||bx>=high.width||by>=high.height||!high.data[(by*high.width+bx)*4+3]) result=color;
        break;
      }
      out.push(...result);
    }
    return out;
  }
  let opaque=0,hidden=0;
  for(const p1 of [2,3]) for(const p2 of [2,3]) for(const flipX of [false,true]) for(const camera of [[0,0],[1,1],[-1,-1]]) {
    const actors=[{source:[0,0,4,3],rgba:Array.from({length:48},(_,i)=>i%4===3?(Math.floor(i/4)%4?255:0):[210,30,60][i%4]),offset:[-2,-1],position:[2,2],priority:p1,flipX},
      {source:[0,0,3,2],rgba:[1,2,3,255,0,0,0,0,4,5,6,255,7,8,9,255,10,11,12,255,13,14,15,255],offset:[-1,-1],position:[2,2],priority:p2,flipX:false}];
    for(const order of [actors,[...actors].reverse()]) {
      const expected=oracle(order,camera), actual=Array.from(composeObjects(W,H,order,camera,high));
      assert.deepEqual(actual,expected);
      opaque+=actual.filter((v,i)=>i%4===3&&v===255).length;
      hidden+=actual.filter((v,i)=>i%4===3&&v===0).length;
    }
  }
  assert(opaque>100&&hidden>100,'oracle must exercise opaque and absent/occluded output');
  assert.throws(()=>composeObjects(1,1,[{...sprites[0],priority:0}],[0,0],bg.high),/priority/i);
  return {cases:48,opaque,hidden};
}
// Emit with --browser-script, then run via agent-browser eval --stdin on the
// actual inline room page. Synthetic art only; no host state/transport mutation.
function browserPixels(api, fixture) {
  const make=(width,height,rgba)=> {
    const canvas=document.createElement('canvas');canvas.width=width;canvas.height=height;
    canvas.getContext('2d').putImageData(new ImageData(new Uint8ClampedArray(rgba),width,height),0,0);
    return canvas;
  };
  const {bundle,pixels,state}=fixture();
  const art=api.prepareArt(bundle,pixels,make), bg=api.selectBackground(art,state), source=api.selectActors(art,state);
  const canvas=make(256,224,new Uint8ClampedArray(256*224*4)),ctx=canvas.getContext('2d');
  let checks=0;
  for(const guidePriority of [2,3]) for(const arkPriority of [2,3]) for(const reverse of [false,true]) for(const camera of [[0,0],[128,64]]) {
    const sprites=[{...source[0],priority:guidePriority},{...source[1],priority:arkPriority}];
    if(reverse) sprites.reverse();
    api.drawScene(ctx,bg.image,{...state,camera},sprites,bg.foreground,bg.high);
    // Hand-enumerated full canvas, independent of the renderer/compositor:
    // pixel135 is opaque-high with BOTH objects; pixel136 has transparent BG
    // and transparent Ark, so guide wins regardless of its priority/order.
    const winner=reverse?'guide':'ark', priority=reverse?guidePriority:arkPriority;
    const overlap=priority===2?[80,90,100,255]:winner==='guide'?[10,200,20,255]:[200,10,20,255];
    const expected=new Uint8ClampedArray(256*224*4);
    for(let i=0;i<expected.length;i+=4) expected.set([16,21,29,255],i);
    expected.set(overlap,((95-camera[1])*256+135-camera[0])*4);
    expected.set([30,40,50,255],((95-camera[1])*256+136-camera[0])*4);
    const actual=ctx.getImageData(0,0,256,224).data;
    for(let i=0;i<actual.length;i++) if(actual[i]!==expected[i]) throw new Error(`Browser RGBA mismatch case${checks} byte${i}: ${actual[i]} != ${expected[i]}`);
    checks++;
  }
  // Manifest-precomposed mirror must retain its distinct source anchor/raster;
  // hflip metadata is NOT permission to mirror the composed image a second time.
  const phase=bundle.pandora_scenes['tour-start'], actor=phase.actors[0], key=actor.key.slice(0,-1)+'1';
  bundle.frames[key]={width:2,height:1,offset:[-3,-1],rgba:[40,50,60,255,70,80,90,255]};
  Object.assign(actor,{hflip:true,key});
  const mirrored=api.prepareArt(bundle,pixels,make);
  const scene=[{...state.scene[0],key},state.scene[1]];
  const actors=api.selectActors(mirrored,{...state,scene});
  if(actors[0].flipX!==false || actors[0].offset[0]!==-3) throw new Error('Precomposed mirror/anchor lost');
  api.drawScene(ctx,bg.image,state,actors,bg.foreground,bg.high);
  const actual=ctx.getImageData(133,95,2,1).data;
  if(String(actual)!=='40,50,60,255,70,80,90,255') throw new Error('Precomposed mirror actual browser pixels differ');
  return {status:'passed',fullCanvasChecks:checks,pixelsPerCheck:256*224,precomposedMirrorPixels:2,limits:'Synthetic composition, not native whole RGB or live Pandora admission'};
}
module.exports={fixture,check,html};
if(require.main===module) {
  if(process.argv.includes('--browser-script')) console.log(`(${browserPixels})(globalThis.RoomSlice,${fixture})`);
  else {
    const sandbox={};vm.runInNewContext(html.match(/<script>([\s\S]*?)<\/script>/)[1],sandbox);
    console.log(check(sandbox.RoomSlice));
  }
}
