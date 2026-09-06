# Bounded Pandora world-patch renderer

The optional `art.world_backgrounds` capability lets the preview apply the host's
source-authenticated effective-room tile diff. It is not a second progression
model: the frontend never reads flags, predicts consumed pots/doors, chooses a
collision variant, or infers changes from occupancy. Host catalog construction
and raw-grid diff qualification are separate, parent-owned work.

## Wire contract

When enabled, the catalog has exactly six independent sheet keys: `house`,
`exterior`, `town13`, `cellars`, `box`, `tour`. Each contains:

- `tiles`: object keyed by canonical numeric low-nine-bit tile ID (`0..511`),
  each value `{rgba, high}`. `rgba` has 1024 byte values, with opaque alpha;
  source transparency has already become the preview's checker pixels.
  `high` has all 256 boolean bits, including false bits that clear old priority.
- `candidates`: object keyed by canonical numeric row-major 16×16 sheet cell,
  each value a nonempty array of distinct allowed replacement IDs in that
  sheet's tile catalog. Empty candidate objects (and unused empty tile catalogs)
  are allowed. Same tile IDs in house/cellar catalogs are **not** shared colors.

Every enabled state, including ordinary house/exterior states without a named
Pandora scene, must provide `world_background: {key, patches: [{cell, tile}]}`.
The key must match the selected map's sheet. Patches are the **complete** sparse
difference against the immutable bitmap base, strictly increasing by cell.
An empty set restores that sheet. Missing/unknown keys, unauthorized cells/tiles,
extra fields, malformed values, duplicates and unsorted sets are errors. The
entire set is validated before any remembered patch or pixel/mask update.

Source authenticity remains the host's job; syntactic frontend validation is
not an independent ROM authenticator. Named phase, roster/order, OBJ priority
and typed carry admission remain unchanged and mandatory where applicable.

## Memory and drawing

Preparation retains immutable base RGBA/high data and one working background
raster/high-mask pair **per sheet**, shared across all maps of that sheet. All
prepared maps sharing a sheet must have identical decoded base masks (equivalent
run encodings are accepted). World mode allocates neither legacy per-map
foreground rasters nor the wooden-door open background/foreground variants.

Selection compares the old and new sparse sets. Only cells added, changed or
removed are written. Each write replaces 16×16 RGBA and all 256 high bits;
removal restores those pixels/bits from the immutable base. Unchanged selections
write nothing. Other sheets retain their working pair until their next complete
set arrives. There is no full-sheet copy per tick and no combination cache.

`prepareArt(bundle, pixels, makeRaster, updateRaster?)` accepts an optional test
adapter `updateRaster(image, x, y, width, height, rgba)`. Writes are always 16×16.
Without it, the renderer uses Canvas2D `createImageData(16,16)` and
`putImageData` at the tile origin. A `makeRaster` fake may retain its argument;
restoration data does not alias that mutable buffer or the caller's source art.

World capability consolidates wooden-door rendering: the old patch catalog is
not prepared and `wooden_door_open` does not select another image. Its optional
boolean remains inspection data, not patch authority. Non-world bundles retain
the old door path. World selection always returns a high mask, including in
ordinary house scenes; validated legacy actors default to OBJ2. The existing
pure `composeObjects` resolves the winning opaque OBJ **before** comparing it
with high BG, so a hidden front OBJ2 cannot reveal a rear OBJ3. Carry and source
phase code are reused, not duplicated.

## Host source atlas and projection

The opt-in host compiler authenticates the owned ROM through the navigation
compiler, unions low-nine-bit profile deltas across the shared house/cellar
family, and uses the same admitted `SourceObject` catalog as the core adapter.
It adds the existing authenticated wooden-door writes through a shared raster
helper; no second progression model or whole-sheet pot scan is introduced.

`Art::world_background(map, &game.effective_room(data)?)` projects the complete
sorted sparse state against the original bitmap tile grid. Occupancy/material
attributes alone do not alter pixels. Unknown cell/tile differences and wrong
extents fail closed. Use the effective room, **not** `current_room()`'s selected
immutable collision profile. Town atlas candidates do not infer opened doors;
only source-backed effective core writes can select them.

Host tests cover every immutable navigation profile, family union, separate
palettes, occupancy-only changes, restoration and unauthorized changes. All
**5,120 candidate cell/palette pixel samples** match source RGBA and masks.
These admitted replacements are all opaque low BG; high/transparent replacement
behavior is covered by the synthetic renderer controls, not claimed as a native
candidate witness. The cracked door uses tile **0x1A7**, not 0xA7.

The navigation module registration changes the whole-file observer producer pin.
No capture recipe changed and no pixel pin is being renewed; separate producer
revalidation must preserve the original migration descriptor/evidence. Full
GameState→host→continuous browser acceptance remains a separate integration gate.

## Focused verification

```sh
node crates/map-inspector/tests/pandora-world-check.js
node crates/map-inspector/tests/room-slice-check.js
node crates/map-inspector/tests/pandora-render-check.js
node crates/map-inspector/tests/pandora-carry-check.js
node --test tools/verify-house-browser.test.js tools/verify-conversation-browser.test.js
```

For the real Canvas2D check, serve only the static page on an **owned** port and
use a named, isolated browser session (never the parent's port8765/default
session). No host state or transport overrides are needed:

```sh
# Start as a managed service in a separate terminal; stop it after checking.
python3 -m http.server 8879 --bind 127.0.0.1 --directory crates/map-inspector/web
agent-browser --session pandora-world-render open http://127.0.0.1:8879/room-slice.html
node crates/map-inspector/tests/pandora-world-check.js --browser-script \
  | agent-browser --session pandora-world-render eval --stdin
agent-browser --session pandora-world-render close
```

The static page's normal host-fetch error is expected: the test calls its exported
renderer with synthetic fixtures, not a live game. The common oracle reads all
1024 RGBA bytes and all256 high bits of each checked tile; it covers high→low,
low→high, mixed high bits, nonzero tile rows, checker transparency, restoration,
shared-sheet retention, distinct
house/cellar colors, ignored door selection, ordinary-house default priority and
hidden winning OBJ2 over rear OBJ3. Node additionally rejects malformed catalogs
and complete states without changing any sheet bytes or issuing writes, and
counts allocations/16×16 writes across repeated toggles. Browser checks use the
actual default Canvas2D updater and compare all57,344 destination pixels in each
of six priority/background transitions, with exactly52 bounded tile writes and
8 preparation raster allocations (6 sheets + 2 actors), unchanged through all
selections. These are synthetic renderer samples,
not native RGB/timing, source-candidate, occupancy-diff or live-route evidence.
No Rust/core/server changes or live Pandora enablement belong to this patch.
