// Browser regression: run on a generated viewer via agent-browser eval --stdin.
// Only changes in-memory state; restores the qualified capture after testing.
(() => {
  const assert = (condition, message) => { if (!condition) throw new Error(message); };
  const calls = [];
  const original = {drawImage: ac.drawImage, strokeRect: ac.strokeRect};
  const cp = current();
  const x = Math.ceil(cp.camera[0] / 16) + 1;
  const y = Math.ceil(cp.camera[1] / 16) + 1;
  const cell = y * cp.width + x;
  const raw = cp.cells[cell];
  const controls = ['grid', 'changes', 'place-image'];
  const checked = controls.map(id => $(id).checked);
  try {
    assert(images[index].naturalWidth === 256, 'reference image must be loaded');
    controls.forEach(id => { $(id).checked = true; });
    cp.cells[cell] ^= 1; // A synthetic changed word inside the captured viewport.
    ac.drawImage = function (...args) {
      calls.push({kind: 'image'});
      return original.drawImage.apply(this, args);
    };
    ac.strokeRect = function (...args) {
      calls.push({kind: 'outline', color: this.strokeStyle, args});
      return original.strokeRect.apply(this, args);
    };
    render();
    const image = calls.findIndex(call => call.kind === 'image');
    const grid = calls.findIndex(call => call.kind === 'outline' &&
      call.args[0] === x * 16 + .5 && call.args[1] === y * 16 + .5);
    const change = calls.findIndex(call => call.color === '#ffef73' &&
      call.args[0] === x * 16 + 1 && call.args[1] === y * 16 + 1);
    assert(image >= 0 && grid > image && change > image,
      'atlas grid and changed-cell outlines must render above the reference image');
    assert($('change-count').textContent === 'Changed cells: 1 / 2560',
      'synthetic changed-cell count must be visible');
    $('changes').checked = false;
    render();
    assert($('change-count').textContent === 'Changed cells: 1 / 2560',
      'changed count must remain visible with highlighting off');
    cp.cells[cell] = raw;
    render();
    assert($('change-count').textContent === 'Changed cells: 0 / 2560',
      'unchanged count must remain visible with highlighting off');
    return 'PASS: atlas overlay order and changed-cell counts';
  } finally {
    cp.cells[cell] = raw;
    controls.forEach((id, i) => { $(id).checked = checked[i]; });
    Object.assign(ac, original);
    render();
  }
})()
