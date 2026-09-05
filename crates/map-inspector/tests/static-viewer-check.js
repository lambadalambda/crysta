// Offline browser regression (no Node dependencies). On a generated index.html:
// agent-browser --session static-viewer open file:///absolute/path/index.html
// agent-browser --session static-viewer wait --fn "document.getElementById('atlas').dataset.ready === 'true'"
// agent-browser --session static-viewer eval --stdin < crates/map-inspector/tests/static-viewer-check.js
// Repeat at desktop and mobile: agent-browser --session static-viewer set viewport 390 844
// Uses synthetic words in memory, restores all data/UI state, and never reads canvas
// pixels (file:// canvas readback can be restricted).
// Negative fixtures: emit copies with image set to an absent BMP, a wrong-sized
// BMP, and https://example.invalid/map.bmp. Open the respective HTML with
// ?expect-image-error=missing, ?expect-image-error=dimensions, or
// ?expect-image-error=nonlocal; wait --fn "!!document.getElementById('error').textContent"
// and run this same check. Error testing is opt-in so broken normal exports fail.
(() => {
  'use strict';
  const assert = (ok, message) => { if (!ok) throw new Error(message); };
  const $ = id => document.getElementById(id);
  const hex = (n, width = 4) => '$' + n.toString(16).toUpperCase().padStart(width, '0');
  const atlas = $('atlas');
  const expectedFailure = new URLSearchParams(location.search).get('expect-image-error');
  if (expectedFailure) {
    const messages = {missing: 'could not be loaded', dimensions: 'dimensions do not match', nonlocal: 'local BMP filename'};
    assert(messages[expectedFailure], 'unknown expected image failure');
    assert(atlas && atlas.dataset.ready === 'false', 'failed BMP must not mark canvas ready');
    assert($('error').textContent.includes(messages[expectedFailure]), 'expected image failure must be explained');
    if (expectedFailure === 'nonlocal') {
      assert(mapImage.getAttribute('src') === null, 'rejected filename must never be assigned to the image');
      assert(!performance.getEntriesByType('resource').some(entry => /^https?:/.test(entry.name)),
        'rejected remote filename must not cause a network request');
    }
    const coordinates = $('coordinates').textContent.match(/^Cell (\d+), (\d+)/);
    const scroll = $('atlas-scroll'), left = scroll.scrollLeft, top = scroll.scrollTop;
    const focus = document.activeElement;
    try {
      atlas.focus({preventScroll: true});
      atlas.dispatchEvent(new KeyboardEvent('keydown', {key: 'Home', ctrlKey: true, cancelable: true}));
      atlas.dispatchEvent(new KeyboardEvent('keydown', {key: 'ArrowRight', cancelable: true}));
      assert($('coordinates').textContent.startsWith('Cell 1, 0 ·') &&
        $('raw').textContent === hex(mapData.cells[1]) && $('tile-words').rows.length === 4,
        'raw cell keyboard inspection must still work without a BMP');
      return 'PASS: expected ' + expectedFailure + ' image error and continued raw inspection';
    } finally {
      const rect = atlas.getBoundingClientRect(), zoom = Number($('zoom').value) / 100;
      atlas.dispatchEvent(new MouseEvent('click', {
        clientX: rect.left + (Number(coordinates[1]) + .5) * 16 * zoom,
        clientY: rect.top + (Number(coordinates[2]) + .5) * 16 * zoom,
      }));
      scroll.scrollLeft = left; scroll.scrollTop = top;
      focus.focus({preventScroll: true});
      if (document.activeElement !== focus) atlas.blur();
    }
  }
  assert(atlas && atlas.dataset.ready === 'true', 'full-map BMP must be loaded');
  assert(atlas.width === mapData.width * 16 && atlas.height === mapData.height * 16,
    'canvas must contain the entire natural-resolution map');
  assert($('error').textContent === '', 'viewer must not report an error');
  assert($('limitations').textContent.includes(mapData.limits), 'manifest limitations must be visible');
  assert(/static/i.test($('limitations').textContent) && /sprites/i.test($('limitations').textContent),
    'static-only scope must be explicit');
  const provenance = JSON.parse($('provenance').textContent);
  assert(provenance.rom_sha256 === mapData.rom_sha256, 'ROM digest must be preserved');
  assert(JSON.stringify(provenance.resources) === JSON.stringify(mapData.resources),
    'resource kinds, source ranges, and decoded digests must be preserved');
  assert(document.documentElement.scrollWidth <= window.innerWidth,
    'page must fit the viewport; only map/table containers should scroll');

  const ac = atlas.getContext('2d');
  const pc = $('preview').getContext('2d');
  const original = {draw: ac.drawImage, stroke: ac.strokeRect, preview: pc.drawImage};
  const scroll = $('atlas-scroll');
  const saved = {
    raw: mapData.cells[0], tile: mapData.metatiles[511], zoom: $('zoom').value,
    grid: $('grid').checked, coordinates: $('coordinates').textContent,
    left: scroll.scrollLeft, top: scroll.scrollTop, focus: document.activeElement,
    pageX: window.scrollX, pageY: window.scrollY,
  };
  const calls = [];
  const input = id => $(id).dispatchEvent(new Event('input', {bubbles: true}));
  const key = (key, ctrlKey = false) => atlas.dispatchEvent(new KeyboardEvent('keydown', {
    key, ctrlKey, bubbles: true, cancelable: true,
  }));
  const at = (x, y) => assert($('coordinates').textContent.startsWith(`Cell ${x}, ${y} ·`),
    `selection should be cell ${x}, ${y}`);
  try {
    ac.drawImage = function (...args) {
      calls.push({kind: 'image', args});
      return original.draw.apply(this, args);
    };
    ac.strokeRect = function (...args) {
      calls.push({kind: 'outline', args});
      return original.stroke.apply(this, args);
    };
    pc.drawImage = function (...args) {
      calls.push({kind: 'preview', args});
      return original.preview.apply(this, args);
    };
    mapData.cells[0] = 0xFDFF; // Low nine bits = 511, not the upper flags.
    mapData.metatiles[511] = [0x0000, 0x3FFF, 0x4001, 0x8002];
    key('Home', true);
    at(0, 0);
    assert($('raw').textContent === '$FDFF', 'raw cell word must retain upper bits');
    assert($('tile-index').textContent === '$1FF (511)', 'metatile ID must use only low nine bits');
    const rows = [...$('tile-words').rows].map(row => [...row.cells].map(cell => cell.textContent));
    assert(JSON.stringify(rows) === JSON.stringify([
      ['TL', '$0000', '$000 (0)', '0', '0', 'No', 'No'],
      ['TR', '$3FFF', '$3FF (1023)', '7', '1', 'No', 'No'],
      ['BL', '$4001', '$001 (1)', '0', '0', 'Yes', 'No'],
      ['BR', '$8002', '$002 (2)', '0', '0', 'No', 'Yes'],
    ]), 'SNES words must parse graphics bits 0–9, palette 10–12, priority 13, H 14, V 15');

    $('zoom').value = '200';
    input('zoom');
    assert(atlas.style.width === atlas.width * 2 + 'px' &&
      atlas.style.height === atlas.height * 2 + 'px' && $('zoom-value').textContent === '200%',
      'zoom must change CSS size, not map resolution');
    $('grid').checked = true;
    calls.length = 0;
    input('grid');
    const imageIndex = calls.findIndex(call => call.kind === 'image');
    const outlines = calls.filter(call => call.kind === 'outline');
    assert(imageIndex >= 0 && calls.findIndex(call => call.kind === 'outline') > imageIndex,
      'grid/selection must be above the image');
    assert(outlines.length >= mapData.cells.length, 'grid must cover every cell');
    $('grid').checked = false;
    calls.length = 0;
    input('grid');
    assert(calls.filter(call => call.kind === 'outline').length < mapData.cells.length,
      'grid must turn off');

    // Click at a scrolled, zoomed position: use the transformed canvas bounds.
    scroll.scrollLeft = 32;
    scroll.scrollTop = 32;
    const rect = atlas.getBoundingClientRect();
    atlas.dispatchEvent(new MouseEvent('click', {bubbles: true,
      clientX: rect.left + 2.5 * 16 * 2, clientY: rect.top + 2.5 * 16 * 2}));
    at(2, 2);
    assert(document.activeElement === atlas, 'click must focus keyboard inspection');
    assert($('raw').textContent === hex(mapData.cells[2 * mapData.width + 2]),
      'click must read the row-major cell');
    const preview = calls.filter(call => call.kind === 'preview').at(-1);
    const image = calls.find(call => call.kind === 'image');
    assert(preview && image && preview.args[0] === image.args[0] &&
      JSON.stringify(preview.args.slice(1, 5)) === '[32,32,16,16]' &&
      JSON.stringify(preview.args.slice(5)) === '[0,0,128,128]',
      'preview must crop the selected 16×16 metatile from the original BMP, without overlays');
    assert(!pc.imageSmoothingEnabled, 'enlarged preview must remain pixel-sharp');
    key('ArrowRight'); at(3, 2);
    key('ArrowDown'); at(3, 3);
    key('ArrowLeft'); at(2, 3);
    key('ArrowUp'); at(2, 2);
    key('Home', true); key('ArrowLeft'); key('ArrowUp'); at(0, 0);
    key('End', true); key('ArrowRight'); key('ArrowDown');
    at(mapData.width - 1, mapData.height - 1);
    const zoom = Number($('zoom').value) / 100;
    assert(scroll.scrollLeft + scroll.clientWidth >= mapData.width * 16 * zoom - 1 &&
      scroll.scrollTop + scroll.clientHeight >= mapData.height * 16 * zoom - 1,
      'keyboard navigation must scroll the selected cell into view');
    assert(getComputedStyle(scroll).outlineStyle !== 'none' && parseFloat(getComputedStyle(scroll).outlineWidth) >= 2,
      'visible scroll container must indicate map focus even at the far corner');
    key('Home'); at(0, mapData.height - 1);
    key('End'); at(mapData.width - 1, mapData.height - 1);
    return 'PASS: full BMP, provenance, static limits, responsive layout, SNES words, zoom, grid, click, keyboard, preview';
  } finally {
    mapData.cells[0] = saved.raw;
    mapData.metatiles[511] = saved.tile;
    ac.drawImage = original.draw;
    ac.strokeRect = original.stroke;
    pc.drawImage = original.preview;
    $('zoom').value = saved.zoom;
    $('grid').checked = saved.grid;
    input('zoom');
    const match = saved.coordinates.match(/^Cell (\d+), (\d+)/);
    if (match) {
      const rect = atlas.getBoundingClientRect(), zoom = Number(saved.zoom) / 100;
      atlas.dispatchEvent(new MouseEvent('click', {
        clientX: rect.left + (Number(match[1]) + .5) * 16 * zoom,
        clientY: rect.top + (Number(match[2]) + .5) * 16 * zoom,
      }));
    }
    scroll.scrollLeft = saved.left;
    scroll.scrollTop = saved.top;
    saved.focus.focus({preventScroll: true});
    if (document.activeElement !== saved.focus) atlas.blur();
    window.scrollTo(saved.pageX, saved.pageY);
  }
})()
