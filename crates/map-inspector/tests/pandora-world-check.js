// Synthetic finite catalog, not source authentication or a live journey witness.
'use strict';
const assert=require('node:assert/strict');
const vm=require('node:vm');
const {fixture:baseFixture,html}=require('./pandora-render-check.js');
function fixture() {
  const f=baseFixture(), {bundle,pixels}=f;
  const sheets={...bundle.backgrounds,...bundle.pandora_backgrounds.backgrounds};
  bundle.world_backgrounds={};
  for(const [index,[key,{width,height}]] of Object.entries(sheets).entries()) {
    const base=[20+index,40,60,255];
    for(let p=0;p<width*height;p++) pixels[key].data.set(base,p*4);
    const tile=(color,high)=>({rgba:Array.from({length:256},()=>color).flat(),high:Array(256).fill(high)});
    // Tile3 represents source transparency already rendered as opaque checker.
    const checker=tile([0,0,0,255],false);
    for(let p=0;p<256;p++) checker.rgba.splice(p*4,4,...((p%16<8)===(Math.floor(p/16)<8)?[36,36,36,255]:[48,48,48,255]));
    const mixed=tile([70+index,80,90,255],false);mixed.high=mixed.high.map((_,p)=>p%3!==0);
    bundle.world_backgrounds[key]={tiles:{1:tile([100+index,10,30,255],false),2:tile([10,120+index,30,255],true),3:checker,4:mixed},candidates:{0:[1,2,3,4],1:[1,2,3], [width/16+1]:[4]}};
  }
  // All maps of a sheet must describe the same immutable mask.
  for(const [id,mask] of Object.entries({...bundle.foreground,...bundle.pandora_backgrounds.foreground})) {
    mask.runs=[];
    for(let y=0;y<16;y++) mask.runs.push(y*mask.width,16);
  }
  // Deliberately invalid legacy door catalog: world capability must not touch it.
  bundle.door_patches='obsolete';
  f.state={map_id:15,actor_key:'ark',x:1,y:1,camera:[0,0],scene:[{id:'ark',key:'ark',position:[1,1]}],world_background:{key:'house',patches:[]}};
  return f;
}
// Shared Node/real Canvas checks, including each byte of both affected cells.
function exercise(api, fixture, make, update, read, draw) {
  const eq=(a,b,label)=>{if(JSON.stringify(Array.from(a))!==JSON.stringify(Array.from(b))) throw new Error(label);};
  const ok=(v,label)=>{if(!v) throw new Error(label);};
  const {bundle,pixels,state}=fixture();
  let allocations=0,writes=0;
  const art=api.prepareArt(bundle,pixels,(...args)=>{allocations++;return make(...args);},update && ((...args)=>{writes++;update(...args);}));
  ok(art.openBackground===null&&Object.keys(art.openForeground).length===0,'no competing door allocations');
  const initialAllocations=allocations;
  const apply=(patches,extra={})=>api.selectBackground(art,{...state,...extra,world_background:{key:'house',patches}});
  const original=apply([]), high=original.high, image=original.image;
  ok(high,'ordinary house must use software priority');
  const actor=api.selectActors(art,state)[0];
  ok(actor.priority===2,'validated legacy priority defaults to2');
  const expectedCell=(tile,cell)=>tile?bundle.world_backgrounds.house.tiles[tile]:{rgba:Array.from({length:256},()=>[20,40,60,255]).flat(),high:Array(256).fill(cell===0)};
  let checks=0;
  const check=(bg,tile,cell=0)=>{
    const x=cell%32*16,y=Math.floor(cell/32)*16;
    const expected=expectedCell(tile,cell),actual=read(bg.image,x,y,16,16);
    eq(actual,expected.rgba,`RGBA tile${tile}`);
    for(let p=0;p<256;p++) ok(!!bg.high.data[(((y+Math.floor(p/16))*512+x+p%16)*4)+3]===expected.high[p],`high bit${p} tile${tile}`);
    ok(bg.image===image&&bg.high===high,'stable working pair');checks++;
  };
  check(original,0);
  for(const tile of [1,2,3,4,2,1]) {
    const bg=apply([{cell:0,tile}],{wooden_door_open:true});check(bg,tile);
    // Front OBJ2 hides rear OBJ3 even when the front winner is hidden by BG.
    const rear={...actor,priority:3,rgba:new Uint8ClampedArray([1,2,3,255,0,0,0,0])};
    const objects=[rear,actor];
    const expected=tile===2?[0,0,0,0]:[200,10,20,255];
    eq(api.composeObjects(1,1,objects,[0,0],bg.high),expected,'winner OBJ before high BG');
    if(draw) draw(api,bg,state,objects,tile===2?[10,120,30,255]:[200,10,20,255]);
    const before=writes;apply([{cell:0,tile}],{wooden_door_open:false});ok(writes===before,'unchanged set/inspection field writes nothing');
  }
  apply([{cell:0,tile:2},{cell:1,tile:1}]);
  const beforeRemove=writes;
  check(apply([{cell:1,tile:1}]),0);
  if(update) ok(writes===beforeRemove+1,'only removed cell restored');
  check(apply([{cell:1,tile:1}],{map_id:12,scene_phase:'c-departed'}),1,1);
  // A different palette/family must not disturb the retained house pair.
  const cellar=api.selectBackground(art,{...state,map_id:14,scene_phase:'cellar-e',world_background:{key:'cellars',patches:[{cell:0,tile:1}]}});
  eq(read(cellar.image,0,0,16,16),bundle.world_backgrounds.cellars.tiles[1].rgba,'cellar palette');
  check(apply([{cell:1,tile:1}]),1,1);
  const restored=apply([]);check(restored,0);
  eq(read(restored.image,16,0,16,16),Array.from({length:256},()=>[20,40,60,255]).flat(),'removed low-base cell restores');
  for(let p=0;p<256;p++) ok(!high.data[(Math.floor(p/16)*512+16+p%16)*4+3],'restore low base bits');
  check(apply([{cell:33,tile:4}]),4,33);
  check(apply([]),0,33);
  for(let i=0;i<40;i++) apply(i%2?[{cell:0,tile:2}]:[]);
  ok(allocations===initialAllocations,'no tick/combo raster allocations');
  eq(pixels.house.data.slice(0,4),[20,40,60,255],'immutable input base');
  return {checks,writes,allocations};
}
function nodeCheck(api) {
  const make=(width,height,data)=>({width,height,data:new Uint8ClampedArray(data)});
  const update=(image,x,y,width,height,data)=>{
    assert.equal(width,16);assert.equal(height,16);assert.equal(data.length,1024);
    for(let row=0;row<height;row++) image.data.set(data.subarray(row*width*4,(row+1)*width*4),((y+row)*image.width+x)*4);
  };
  const read=(image,x,y,w,h)=>Array.from({length:h},(_,r)=>Array.from(image.data.slice(((y+r)*image.width+x)*4,((y+r)*image.width+x+w)*4))).flat();
  const result=exercise(api,fixture,make,update,read);
  const {bundle,pixels,state}=fixture();
  let writes=0;
  const art=api.prepareArt(bundle,pixels,make,(...args)=>{writes++;update(...args);});
  const good={...state,world_background:{key:'house',patches:[{cell:0,tile:2}]}};
  const bg=api.selectBackground(art,good);
  const rgba=bg.image.data.slice(),high=bg.high.data.slice(),before=writes;
  for(const world of [undefined,null,[],{}, {key:'house'}, {key:'evil',patches:[]},{key:'cellars',patches:[]},
    {key:'house',patches:[],extra:1},...[null,{},[null],[{cell:0,tile:1},{cell:1,tile:99}],
    [{cell:1,tile:1},{cell:0,tile:1}],[{cell:0,tile:1},{cell:0,tile:2}],
    [{cell:2,tile:1}],[{cell:2048,tile:1}],[{cell:-1,tile:1}],[{cell:0.5,tile:1}],
    [{cell:'0',tile:1}],[{cell:0,tile:'1'}],[{cell:0,tile:512}],[{cell:0,tile:1,extra:0}]
  ].map(patches=>({key:'house',patches}))]) {
    assert.throws(()=>api.selectBackground(art,{...good,world_background:world}),/world/i);
    assert.deepEqual(bg.image.data,rgba,'invalid complete set must be atomic');
    assert.deepEqual(bg.high.data,high);assert.equal(writes,before);
  }
  for(const mutate of [b=>b.world_backgrounds=null,b=>b.world_backgrounds=[],b=>delete b.world_backgrounds.box,b=>b.world_backgrounds.evil={},
    b=>b.world_backgrounds.house.tiles[512]=b.world_backgrounds.house.tiles[1],b=>b.world_backgrounds.house.tiles['01']=b.world_backgrounds.house.tiles[1],
    b=>b.world_backgrounds.house.tiles[1].rgba.pop(),b=>b.world_backgrounds.house.tiles[1].rgba[0]=256,
    b=>b.world_backgrounds.house.tiles[1].rgba[3]=0,b=>b.world_backgrounds.house.tiles[1].high[0]=1,b=>b.world_backgrounds.house.tiles[1].high.pop(),
    b=>b.world_backgrounds.house.candidates[2048]=[1],b=>b.world_backgrounds.house.candidates[2]=[5],
    b=>b.world_backgrounds.house.candidates[2]=[1,1],b=>b.world_backgrounds.house.candidates[2]=['1'],
    b=>b.world_backgrounds.house.candidates=[],b=>b.foreground[12].runs=[]]) {
    const bad=structuredClone(bundle);mutate(bad);assert.throws(()=>api.prepareArt(bad,pixels,make,update),/world|mask/i);
  }
  const empty=structuredClone(bundle);empty.world_backgrounds.box={tiles:{},candidates:{}};
  assert.doesNotThrow(()=>api.prepareArt(empty,pixels,make,update));
  // Different run encodings with equal bits are permitted.
  const equivalent=structuredClone(bundle);equivalent.foreground[12].runs.splice(0,2,0,8,8,8);
  assert.doesNotThrow(()=>api.prepareArt(equivalent,pixels,make,update));
  // The immutable copies/catalog must not alias mutable caller arrays.
  pixels.house.data.fill(255);bundle.world_backgrounds.house.tiles[1].rgba.fill(255);
  api.selectBackground(art,{...state,world_background:{key:'house',patches:[{cell:0,tile:1}]}});
  assert.deepEqual(read(bg.image,0,0,1,1),[100,10,30,255]);
  api.selectBackground(art,state);assert.deepEqual(read(bg.image,0,0,1,1),[20,40,60,255]);
  return result;
}
function browserCheck(api,fixture,exercise) {
  let rasterWrites=0;
  const make=(w,h,data)=>{
    const c=document.createElement('canvas');c.width=w;c.height=h;
    const ctx=c.getContext('2d');ctx.putImageData(new ImageData(new Uint8ClampedArray(data),w,h),0,0);
    if(h>=512) {
      const put=ctx.putImageData.bind(ctx);
      ctx.putImageData=(pixels,x,y)=>{
        if(pixels.width!==16||pixels.height!==16||x%16||y%16) throw new Error('Unbounded canvas write');
        rasterWrites++;put(pixels,x,y);
      };
    }
    return c;
  };
  const read=(c,x,y,w,h)=>c.getContext('2d').getImageData(x,y,w,h).data;
  let fullCanvasChecks=0;
  const canvas=make(256,224,new Uint8ClampedArray(256*224*4));
  const draw=(api,bg,state,objects,color)=>{
    api.drawScene(canvas.getContext('2d'),bg.image,state,objects,bg.foreground,bg.high);
    const actual=read(canvas,0,0,256,224),expected=read(bg.image,0,0,256,224);expected.set(color,0);
    for(let i=0;i<actual.length;i++) if(actual[i]!==expected[i]) throw new Error(`world canvas byte${i}: ${actual[i]} != ${expected[i]}`);
    fullCanvasChecks++;
  };
  const result=exercise(api,fixture,make,undefined,read,draw);
  if(rasterWrites!==52) throw new Error(`Unexpected Canvas2D write count: ${rasterWrites}`);
  return {...result,writes:rasterWrites,fullCanvasChecks,status:'passed',evidence:'synthetic catalog and states; real Canvas2D, not native/runtime qualification'};
}
if(require.main===module) {
  if(process.argv.includes('--browser-script')) console.log(`(()=>{const baseFixture=${baseFixture};return (${browserCheck})(globalThis.RoomSlice,${fixture},${exercise});})()`);
  else {const sandbox={};vm.runInNewContext(html.match(/<script>([\s\S]*?)<\/script>/)[1],sandbox);console.log(nodeCheck(sandbox.RoomSlice));}
}
