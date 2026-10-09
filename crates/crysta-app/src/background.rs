//! Native background presentation, separate from the asset inspector's checkerboard.
use crate::frame::{rgb, signed, Canvas, CLASSIC_WIDTH, VIEW_HEIGHT};
use assets::graphics::{self, BgTileWord, Bgr555, IndexedPixel, Tile4bpp};
use assets::maps::actors::SpawnList;
use assets::maps::scripts::EventFlags;
use assets::maps::visual::{
    camera::CameraRegion, profile::Presentation, scene_animation::SceneAnimation, SecondLayer,
    StaticBackground,
};

/// The palette entry the darkness of tower 5's top rewrites (`$7F:06FE`).
const DARKNESS_COLOUR: u8 = 0x7F;

/// Presentation age since entry, advanced by host simulation updates, not redraws.
pub struct VisitClock {
    map: u16,
    tick: u64,
}

impl VisitClock {
    /// A visit to `map` starting now.
    pub const fn new(map: u16) -> Self {
        Self { map, tick: 0 }
    }

    /// One frame in `map`; another map starts a new visit.
    pub fn advance(&mut self, map: u16) {
        if self.map == map {
            self.tick = self.tick.saturating_add(1);
        } else {
            *self = Self::new(map);
        }
    }

    /// Frames into the visit.
    pub const fn tick(&self) -> u64 {
        self.tick
    }
}

/// A world map's flat Mode 7 plane, its whole extent as the region, and
/// the perspective view the hosts draw of it ([`crate::mode7`]).
fn load_world(cartridge: &rom::Rom, map: u16) -> Result<CachedBackground, String> {
    let world = assets::maps::visual::world::WorldMap::from_rom(cartridge.image(), map)
        .map_err(|error| error.to_string())?;
    let (width, height) = (world.width() * 16, world.height() * 16);
    let pixels = (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .map(|(x, y)| rgb(world.color(world.pixel(x, y))))
        .collect();
    let edge = |pixels: usize| u16::try_from(pixels).unwrap_or(u16::MAX);
    Ok(CachedBackground {
        frame: crate::frame::Background {
            pixels,
            width,
            height,
            high: Vec::new(),
        },
        region: CameraRegion {
            record_offset: 0,
            bounds: [0, 0, edge(width), edge(height)],
            vertical_extent: edge(height),
        },
        animation: None,
        second: None,
        sky: None,
        front: None,
        second_cells: Vec::new(),
        patches: Patches::default(),
        world: true,
        mode7: assets::maps::visual::mode7::Mode7View::from_rom(cartridge.image())
            .inspect_err(|error| eprintln!("world map drawn flat: {error}"))
            .ok()
            .map(|view| Box::new(crate::mode7::Mode7 { map: world, view })),
    })
}

/// Load the static baseline without changing the inspector's export policy,
/// with the animation the map's service actors play under `events`, the
/// flags its spawn list ran with.
pub fn load(cartridge: &rom::Rom, map: u16, events: &[u8]) -> Result<CachedBackground, String> {
    if crysta_runtime::WORLD_MAPS.contains(&map) {
        return load_world(cartridge, map);
    }
    let region =
        CameraRegion::from_rom(cartridge.image(), map).map_err(|error| error.to_string())?;
    let scene = assets::maps::visual::first_background(cartridge.image(), map)
        .map_err(|error| error.to_string())?;
    let (mut background, indices) = render(&scene)?;
    let [_, _, right, bottom] = region.bounds;
    if usize::from(right) > background.width || usize::from(bottom) > background.height {
        return Err("camera region outside the decoded layer".into());
    }
    let image = cartridge.image();
    let backdrop = if map == 0xA {
        let color = exterior_backdrop(image, &scene)?;
        composite_backdrop(&mut background.pixels, &indices, color)?;
        Some(color)
    } else {
        None
    };
    let source = SpawnList::resolve(image, map, EventFlags::Bitmap(events))
        .ok()
        .and_then(|records| {
            SceneAnimation::from_records(image, &records, |flag| {
                EventFlags::Bitmap(events).get(flag) == Some(true)
            })
            .ok()
        })
        .filter(|source| !source.is_empty());
    // Drawn in front, as added light or as a sky; not yet subtracted
    // (`meta/issues/tower-second-layer.md`).
    let layer = SecondLayer::from_rom(image, map).ok();
    let (front, layer) = match layer {
        Some(layer) if layer.presentation() == Presentation::Front => (Some(layer), None),
        layer => (None, layer),
    };
    // The cells of a layer that shows; patches to another are not drawn.
    let shown = layer
        .as_ref()
        .filter(|layer| layer.presentation() != Presentation::Hidden);
    let second_cells: Vec<u16> = front
        .as_ref()
        .or(shown)
        .map(|layer| {
            layer
                .layer()
                .cells()
                .iter()
                .map(|cell| cell.raw() & 511)
                .collect()
        })
        .unwrap_or_default();
    if let Some(front) = &front {
        let all = 0..second_cells.len();
        let layer = (front, &second_cells[..]);
        overlay_front(
            &mut background,
            layer,
            (scene.tiles(), scene.palette()),
            all,
        );
    }
    let second = layer
        .filter(|layer| {
            matches!(
                layer.presentation(),
                Presentation::Added | Presentation::Sky | Presentation::Subtracted
            )
        })
        .map(|layer| {
            // The first layer's 512 tiles (a tower's sheet), then the
            // layer's own from `$200`.
            let mut tiles = scene.tiles().to_vec();
            if !layer.tiles().is_empty() {
                tiles.truncate(0x200);
                tiles.extend_from_slice(layer.tiles());
            }
            Second {
                layer,
                tiles,
                palette: *scene.palette(),
            }
        });
    let sky = second
        .as_ref()
        .filter(|second| second.layer.fixed())
        .map(|second| Sky::new(second, &indices));
    let animation = source.map(|source| Animated::new(scene, source, backdrop, front.as_ref()));
    Ok(CachedBackground {
        frame: background,
        region,
        animation,
        sky,
        second_cells,
        second,
        front,
        patches: Patches::default(),
        world: false,
        mode7: None,
    })
}

/// The whole first layer, as the inspector's static export draws it, and
/// each pixel's palette index. The same loop as `map_inspector`'s export,
/// whose source is hash-pinned by a qualification fixture; it also covers the
/// tour maps through `first_background`.
fn render(scene: &StaticBackground) -> Result<(crate::frame::Background, Vec<u8>), String> {
    let (width, height) = (scene.layer().width() * 16, scene.layer().height() * 16);
    let mut background = crate::frame::Background {
        pixels: Vec::with_capacity(width * height),
        width,
        height,
        high: Vec::with_capacity(width * height),
    };
    let mut indices = Vec::with_capacity(width * height);
    for y in 0..height {
        for x in 0..width {
            let (index, high) = match scene.pixel(x, y).map_err(|error| error.to_string())? {
                IndexedPixel::Transparent => (0, false),
                IndexedPixel::Opaque {
                    palette_index,
                    priority,
                } => (palette_index, priority),
            };
            background.pixels.push(static_rgb(index, scene, (x, y)));
            background.high.push(high);
            indices.push(index);
        }
    }
    Ok((background, indices))
}

/// A patched cell: column, row, metatile.
pub type Patched = (u16, u16, u16);

/// Cells the world's scripts have re-tiled, and what they looked like.
#[derive(Default)]
struct Patches {
    /// The map's static scene, decoded on the first patch.
    scene: Option<StaticBackground>,
    /// Cells drawn with a patched tile: column, row, tile.
    drawn: Vec<Patched>,
    /// The second layer's.
    second: Vec<Patched>,
}

/// The cells whose tile changes from `drawn` to `now`, with their new tile:
/// the patched ones, and those no longer patched back to `original`.
fn changed(
    drawn: &[Patched],
    now: &[Patched],
    original: impl Fn(usize) -> Option<u16>,
    width: usize,
) -> Vec<Patched> {
    let mut cells: Vec<Patched> = drawn
        .iter()
        .filter(|&&(column, row, _)| !now.iter().any(|&(c, r, _)| (c, r) == (column, row)))
        .filter_map(|&(column, row, _)| {
            let tile = original(usize::from(row) * width + usize::from(column))?;
            Some((column, row, tile))
        })
        .collect();
    cells.extend(now.iter().filter(|cell| !drawn.contains(cell)));
    cells
}

impl Patches {
    fn scene(&mut self, image: &[u8], map: u16) -> Option<&StaticBackground> {
        if self.scene.is_none() {
            self.scene = assets::maps::visual::first_background(image, map).ok();
        }
        self.scene.as_ref()
    }
}

/// Static pixels with an optional, bounded map-A animation overlay.
pub struct CachedBackground {
    /// The composed first background.
    pub frame: crate::frame::Background,
    /// The part of the shared layer this map's camera may show.
    pub region: CameraRegion,
    animation: Option<Animated>,
    /// The second layer: the town's clouds, the rooms' light rays.
    second: Option<Second>,
    /// A fixed second layer behind the first.
    sky: Option<Sky>,
    /// The second layer in front of the first ([`overlay_front`]).
    front: Option<SecondLayer>,
    /// The second layer's metatiles now, patches included.
    second_cells: Vec<u16>,
    patches: Patches,
    /// A world map: its camera follows the player unclamped (`$87:9123`).
    pub world: bool,
    /// A world map's Mode 7 view, drawn in place of the flat plane.
    pub mode7: Option<Box<crate::mode7::Mode7>>,
}

impl CachedBackground {
    /// Draws the first layer seen from `camera`: a world map in Mode 7
    /// about the player, any other map flat.
    pub fn draw(&self, frame: &mut Canvas, camera: (i32, i32), player: (u16, u16)) {
        if let Some(mode7) = &self.mode7 {
            crate::mode7::draw(frame, mode7, player);
        } else {
            crate::frame::draw_background(frame, &self.frame, camera);
            if let Some(sky) = &self.sky {
                sky.show(frame, camera, self.frame.width);
            }
        }
    }

    /// The player-helper's mask (`docs/depth-order.md`): its priority-1
    /// tiles lose to the background and win over every sprite, so the wall
    /// above a doorway or a stairwell's frame shows over Ark. `masks` are
    /// world rectangles, half-open.
    pub fn cover(&self, frame: &mut Canvas, camera: (i32, i32), masks: &[(i32, i32, i32, i32)]) {
        for &(left, top, right, bottom) in masks {
            for world_y in top..bottom {
                for world_x in left..right {
                    let (Ok(bx), Ok(by)) = (usize::try_from(world_x), usize::try_from(world_y))
                    else {
                        continue;
                    };
                    if let Some(&pixel) = (bx < self.frame.width)
                        .then(|| self.frame.pixels.get(by * self.frame.width + bx))
                        .flatten()
                    {
                        frame.set((world_x - camera.0, world_y - camera.1), pixel);
                    }
                }
            }
        }
    }

    /// Extends the region's edges into a wide view
    /// ([`crate::frame::extend_edges`]); the Mode 7 view wraps and fills the
    /// screen itself.
    pub fn extend_edges(&self, frame: &mut Canvas, camera: (i32, i32)) {
        if self.mode7.is_none() {
            crate::frame::extend_edges(frame, camera, self.region.bounds);
        }
    }

    /// Adds the second layer onto the view: a subscreen layer the scene
    /// adds to the main screen in full, over the sprites too, as in the game
    /// -- the town's crystal clouds (`CGADSUB $33`, `docs/house-exterior.md`)
    /// and the rooms' light rays (`CGADSUB $21`). It scrolls with the
    /// camera; the clouds also drift a pixel left and down every three
    /// frames (`$086C`/`$086E` = `$02FF`/`$0201`, native town checkpoints:
    /// 80 pixels over 241 frames). Its tiles and colours are the animated
    /// ones where the map animates.
    pub fn add_second_layer(
        &self,
        canvas: &mut crate::frame::Canvas,
        camera: (i32, i32),
        age: u64,
    ) {
        let Some(second) = self
            .second
            .as_ref()
            .filter(|second| second.layer.presentation() == Presentation::Added)
        else {
            return;
        };
        let (tiles, palette) = self
            .animation
            .as_ref()
            .map_or((&second.tiles[..], &second.palette), |animation| {
                (&animation.tiles[..], &animation.palette)
            });
        let layer = second.layer.layer();
        let (width, height) = (layer.width() * 16, layer.height() * 16);
        let drift = if second.layer.drifts() {
            i64::try_from(age / 3).unwrap_or(0)
        } else {
            0
        };
        let wrap = |at: i64, extent: usize| {
            usize::try_from(at.rem_euclid(i64::try_from(extent).unwrap_or(1))).unwrap_or(0)
        };
        for (row, line) in canvas.pixels.chunks_mut(canvas.width).enumerate() {
            let y = wrap(
                i64::from(camera.1) + i64::try_from(row).unwrap_or(0) + drift,
                height,
            );
            for (column, pixel) in line.iter_mut().enumerate() {
                let x = wrap(
                    i64::from(camera.0) + i64::try_from(column).unwrap_or(0) - drift,
                    width,
                );
                let cell = usize::from(self.second_cells[y / 16 * layer.width() + x / 16]);
                if cell == 0 {
                    continue;
                }
                if let Ok(IndexedPixel::Opaque { palette_index, .. }) = graphics::sample_metatile(
                    &second.layer.metatiles()[cell],
                    tiles,
                    x % 16,
                    y % 16,
                ) {
                    *pixel = add(*pixel, rgb(palette[usize::from(palette_index)]));
                }
            }
        }
    }

    /// Subtracts the second layer from the view (`$11B`, `$123`:
    /// `docs/tower-second-layer.md` §4.6): on the subscreen, it darkens the
    /// pixels colour math takes (the first layer, the backdrop and OBJ
    /// palettes 4 to 7), in full; where it is clear nothing changes. It
    /// scrolls with the camera.
    pub fn subtract_second_layer(
        &self,
        canvas: &mut crate::frame::Canvas,
        camera: (i32, i32),
        darkness: Option<&crysta_runtime::world::Darkness>,
    ) {
        let Some(second) = self
            .second
            .as_ref()
            .filter(|second| second.layer.presentation() == Presentation::Subtracted)
        else {
            return;
        };
        let layer = second.layer.layer();
        let (width, height) = (layer.width() * 16, layer.height() * 16);
        // Tower 5's top: the mask's colours cycle (`COP 8A $22`).
        let mut palette = second.palette;
        for &(index, colour) in darkness.map_or(&[][..], |darkness| &darkness.light) {
            if let Some(slot) = palette.get_mut(usize::from(index)) {
                *slot = Bgr555::new(colour);
            }
        }
        for (at, pixel) in canvas.pixels.iter_mut().enumerate() {
            if !canvas.math[at] {
                continue;
            }
            let (column, row) = (signed(at % canvas.width), signed(at / canvas.width));
            let (Ok(x), Ok(y)) = (
                usize::try_from(camera.0 + column),
                usize::try_from(camera.1 + row),
            ) else {
                continue;
            };
            // Tower 5's top: a band whose torch is out shows the fill, the
            // darkness's colour, on every pixel (`$8F:8182`).
            let dark = darkness.map(|darkness| Bgr555::new(darkness.colour));
            if let Some(darkness) = darkness.filter(|darkness| !darkness.lit(camera.1 + row)) {
                *pixel = subtract(*pixel, rgb(Bgr555::new(darkness.colour)));
                continue;
            }
            if x >= width || y >= height {
                continue;
            }
            let cell = usize::from(self.second_cells[y / 16 * layer.width() + x / 16]);
            if let Ok(IndexedPixel::Opaque { palette_index, .. }) = graphics::sample_metatile(
                &second.layer.metatiles()[cell],
                &second.tiles,
                x % 16,
                y % 16,
            ) {
                let colour = match dark {
                    Some(dark) if palette_index == DARKNESS_COLOUR => dark,
                    _ => palette[usize::from(palette_index)],
                };
                *pixel = subtract(*pixel, rgb(colour));
            }
        }
    }

    /// The camera for the player: a world map's, or the region's clamp.
    pub fn camera(&self, player: (u16, u16), width: usize) -> (i32, i32) {
        if self.world {
            crate::frame::world_camera(player, width)
        } else {
            crate::frame::camera(&self.region, player, width)
        }
    }

    /// Draws the world's patched cells (`COP 44`, `COP 46`) on both
    /// layers, and restores cells that are no longer patched, from the
    /// map's own metatiles.
    pub fn apply_patches(
        &mut self,
        image: &[u8],
        map: u16,
        patched: &[Patched],
        second: &[Patched],
    ) {
        if self.patches.drawn == patched && self.patches.second == second {
            return;
        }
        self.patches.scene(image, map);
        let Some(scene) = &self.patches.scene else {
            return;
        };
        let first_original =
            |cell: usize| scene.layer().cells().get(cell).map(|cell| cell.raw() & 511);
        let first = changed(
            &self.patches.drawn,
            patched,
            first_original,
            scene.layer().width(),
        );
        let shown = self
            .front
            .as_ref()
            .or(self.second.as_ref().map(|second| &second.layer));
        let second_width = shown.map_or(1, |layer| layer.layer().width());
        let second_original = |cell: usize| {
            shown?
                .layer()
                .cells()
                .get(cell)
                .map(|cell| cell.raw() & 511)
        };
        let changed_second = changed(&self.patches.second, second, second_original, second_width);
        for &(column, row, tile) in changed_second.iter().filter(|_| shown.is_some()) {
            let cell = usize::from(row) * second_width + usize::from(column);
            if let Some(slot) = self.second_cells.get_mut(cell) {
                *slot = tile;
            }
        }
        let backdrop = self
            .animation
            .as_ref()
            .and_then(|animation| animation.backdrop);
        // The first layer's tile at a cell now: patched or the map's own.
        let now = |column: u16, row: u16| {
            patched
                .iter()
                .find(|&&(c, r, _)| (c, r) == (column, row))
                .map(|&(_, _, tile)| tile)
                .or_else(|| {
                    first_original(usize::from(row) * scene.layer().width() + usize::from(column))
                })
        };
        let mut redraw: Vec<(u16, u16)> = first
            .iter()
            .map(|&(column, row, _)| (column, row))
            .collect();
        if self.front.is_some() {
            redraw.extend(changed_second.iter().map(|&(column, row, _)| (column, row)));
            redraw.sort_unstable();
            redraw.dedup();
        }
        for (column, row) in redraw {
            let Some(tile) = now(column, row) else {
                continue;
            };
            let at = (usize::from(column), usize::from(row));
            draw_metatile(scene, &mut self.frame, at, tile, backdrop);
            if let Some(front) = &self.front {
                let cell = usize::from(row) * second_width + usize::from(column);
                let layer = (front, &self.second_cells[..]);
                overlay_front(
                    &mut self.frame,
                    layer,
                    (scene.tiles(), scene.palette()),
                    cell..=cell,
                );
            }
        }
        self.patches.drawn = patched.to_vec();
        self.patches.second = second.to_vec();
    }

    /// Advances the animation to `age` frames into the visit.
    pub fn update(&mut self, age: u64) {
        if let Some(animation) = &mut self.animation {
            let front = self
                .front
                .as_ref()
                .map(|front| (front, &self.second_cells[..]));
            animation.update(age, &mut self.frame, front);
        }
    }
}

/// A map's animated tiles and colours, redrawn into its background.
struct Animated {
    scene: StaticBackground,
    source: SceneAnimation,
    tiles: Vec<Tile4bpp>,
    palette: [Bgr555; 128],
    /// The town's backdrop behind transparent pixels; rooms keep the
    /// static export's colour 0.
    backdrop: Option<u32>,
    /// Cells drawn from an animated tile or palette.
    cells: Vec<usize>,
    key: Option<Vec<Option<u64>>>,
}

/// A map's second layer with the static tiles and colours it draws from
/// where nothing animates them.
struct Second {
    layer: SecondLayer,
    tiles: Vec<Tile4bpp>,
    palette: [Bgr555; 128],
}

/// A fixed second layer behind the first (tower 1's night sky,
/// `docs/tower-entry.md`): the screen it shows, darkened line by line, and
/// where the first layer is clear.
struct Sky {
    pixels: Vec<u32>,
    clear: Vec<bool>,
}

impl Sky {
    fn new(second: &Second, indices: &[u8]) -> Self {
        let layer = second.layer.layer();
        let mut pixels = vec![0; CLASSIC_WIDTH * VIEW_HEIGHT];
        for (row, line) in pixels.chunks_mut(CLASSIC_WIDTH).enumerate() {
            let dark = crate::mode7::fixed(sky_subtraction(row));
            for (column, pixel) in line.iter_mut().enumerate() {
                let at = (row / 16) * layer.width() + (column / 16) % layer.width();
                let cell = layer.cells().get(at).map_or(0, |cell| cell.raw() & 511);
                if let Some(Ok(IndexedPixel::Opaque { palette_index, .. })) = second
                    .layer
                    .metatiles()
                    .get(usize::from(cell))
                    .map(|words| {
                        graphics::sample_metatile(words, &second.tiles, column % 16, row % 16)
                    })
                {
                    let colour = second.palette[usize::from(palette_index)];
                    *pixel = rgb(crate::mode7::subtract(colour, dark));
                }
            }
        }
        Self {
            pixels,
            clear: indices.iter().map(|&index| index == 0).collect(),
        }
    }

    /// Shows the sky where the first layer is clear; a wide view repeats
    /// its 256 columns about the centre.
    fn show(&self, frame: &mut Canvas, camera: (i32, i32), layer_width: usize) {
        let left = (signed(frame.width) - signed(CLASSIC_WIDTH)) / 2;
        for row in 0..VIEW_HEIGHT {
            for column in 0..frame.width {
                let world = (camera.0 + signed(column), camera.1 + signed(row));
                let (Ok(x), Ok(y)) = (usize::try_from(world.0), usize::try_from(world.1)) else {
                    continue;
                };
                if x >= layer_width
                    || !self
                        .clear
                        .get(y * layer_width + x)
                        .copied()
                        .unwrap_or(false)
                {
                    continue;
                }
                let sky_x = usize::try_from((signed(column) - left).rem_euclid(256)).unwrap_or(0);
                frame.pixels[row * frame.width + column] = self.pixels[row * CLASSIC_WIDTH + sky_x];
            }
        }
    }
}

/// The sky's subtracted intensity on view row `row` (`$97:B4BA`: 32 entries
/// of 3 lines, `$1F` down to 0, from line 1).
fn sky_subtraction(row: usize) -> u8 {
    u8::try_from(31_usize.saturating_sub(row / 3)).unwrap_or(0)
}

/// The SNES's colour subtraction: each channel floors at zero.
fn subtract(main: u32, sub: u32) -> u32 {
    let channel =
        |shift: u32| ((main >> shift) & 0xFF).saturating_sub((sub >> shift) & 0xFF) << shift;
    channel(16) | channel(8) | channel(0)
}

/// The SNES's colour addition: each channel saturates.
fn add(main: u32, sub: u32) -> u32 {
    let channel =
        |shift: u32| (((main >> shift) & 0xFF) + ((sub >> shift) & 0xFF)).min(0xFF) << shift;
    channel(16) | channel(8) | channel(0)
}

impl Animated {
    /// The animation of `scene`'s first layer, and of the second in front
    /// of it: a cell is redrawn when either layer's metatile there uses an
    /// animated tile or colour row.
    fn new(
        scene: StaticBackground,
        source: SceneAnimation,
        backdrop: Option<u32>,
        front: Option<&SecondLayer>,
    ) -> Self {
        let tiles: std::collections::HashSet<usize> = source.tiles().collect();
        let rows: std::collections::HashSet<usize> =
            source.colors().map(|color| color / 16).collect();
        let moving = |cells: &[assets::maps::MapCell], metatiles: &[[BgTileWord; 4]]| {
            let animated: Vec<bool> = metatiles
                .iter()
                .map(|words| {
                    words.iter().any(|word| {
                        tiles.contains(&usize::from(word.tile_index()))
                            || rows.contains(&usize::from(word.palette()))
                    })
                })
                .collect();
            cells
                .iter()
                .enumerate()
                .filter(|(_, cell)| animated.get(usize::from(cell.raw() & 511)) == Some(&true))
                .map(|(i, _)| i)
                .collect::<Vec<usize>>()
        };
        let mut cells = moving(scene.layer().cells(), scene.metatiles());
        if let Some(front) = front {
            cells.extend(moving(front.layer().cells(), front.metatiles()));
            cells.sort_unstable();
            cells.dedup();
        }
        Self {
            tiles: scene.tiles().to_vec(),
            palette: *scene.palette(),
            scene,
            source,
            backdrop,
            cells,
            key: None,
        }
    }

    fn update(
        &mut self,
        age: u64,
        frame: &mut crate::frame::Background,
        front: Option<(&SecondLayer, &[u16])>,
    ) {
        let key = self.source.phase_key(age);
        if self.key.as_ref() == Some(&key) {
            return;
        }
        self.tiles.copy_from_slice(self.scene.tiles());
        self.palette = *self.scene.palette();
        self.source.apply(age, &mut self.tiles, &mut self.palette);
        let width = self.scene.layer().width();
        for &cell in &self.cells {
            let words =
                &self.scene.metatiles()[usize::from(self.scene.layer().cells()[cell].raw() & 511)];
            let (column, row) = (cell % width * 16, cell / width * 16);
            for y in 0..16 {
                for x in 0..16 {
                    let (color, high) = match graphics::sample_metatile(words, &self.tiles, x, y)
                        .expect("validated map definitions and tile extents")
                    {
                        IndexedPixel::Transparent => (
                            self.backdrop.unwrap_or_else(|| {
                                static_rgb(0, &self.scene, (column + x, row + y))
                            }),
                            false,
                        ),
                        IndexedPixel::Opaque {
                            palette_index,
                            priority,
                        } => (rgb(self.palette[usize::from(palette_index)]), priority),
                    };
                    let offset = (row + y) * frame.width + column + x;
                    frame.pixels[offset] = color;
                    frame.high[offset] = high;
                }
            }
        }
        if let Some(front) = front {
            let cells = self.cells.iter().copied();
            overlay_front(frame, front, (&self.tiles, &self.palette), cells);
        }
        self.key = Some(key);
    }
}

/// Draws the second layer's `cells` in front of the first
/// ([`Presentation::Front`], `docs/tower-second-layer.md` §3): an opaque
/// pixel covers the first layer's unless it is low over a high one, and
/// brings its priority. The layers share tiles and colours; `now` holds
/// the second layer's metatiles, patches included.
fn overlay_front(
    frame: &mut crate::frame::Background,
    (front, now): (&SecondLayer, &[u16]),
    (tiles, palette): (&[Tile4bpp], &[Bgr555; 128]),
    cells: impl IntoIterator<Item = usize>,
) {
    let width = front.layer().width();
    for cell in cells {
        let Some(words) = now
            .get(cell)
            .and_then(|&tile| front.metatiles().get(usize::from(tile)))
        else {
            continue;
        };
        let (column, row) = (cell % width * 16, cell / width * 16);
        for y in 0..16 {
            for x in 0..16 {
                let Ok(IndexedPixel::Opaque {
                    palette_index,
                    priority,
                }) = graphics::sample_metatile(words, tiles, x, y)
                else {
                    continue;
                };
                let at = (row + y) * frame.width + column + x;
                if column + x >= frame.width || at >= frame.pixels.len() {
                    continue;
                }
                if priority || !frame.high[at] {
                    frame.pixels[at] = rgb(palette[usize::from(palette_index)]);
                    frame.high[at] = priority;
                }
            }
        }
    }
}

/// Draws one 16x16 cell from a scene's metatile, as the static render does
/// (`map_inspector::static_pixel_rgb`), or over the exterior's backdrop.
fn draw_metatile(
    scene: &StaticBackground,
    frame: &mut crate::frame::Background,
    (column, row): (usize, usize),
    tile: u16,
    backdrop: Option<u32>,
) {
    if (column + 1) * 16 > frame.width {
        return;
    }
    let Some(pixels) = metatile_pixels(scene, tile) else {
        return;
    };
    for (at, (index, high)) in pixels.into_iter().enumerate() {
        let (world_x, world_y) = (column * 16 + at % 16, row * 16 + at / 16);
        let color = match (index, backdrop) {
            (0, Some(backdrop)) => backdrop,
            _ => static_rgb(index, scene, (world_x, world_y)),
        };
        let offset = world_y * frame.width + world_x;
        if let Some(pixel) = frame.pixels.get_mut(offset) {
            *pixel = color;
        }
        if let Some(bit) = frame.high.get_mut(offset) {
            *bit = high;
        }
    }
}

/// A metatile's 16×16 pixels, row-major: palette index (0 transparent) and
/// priority. `None` when it does not decode.
fn metatile_pixels(scene: &StaticBackground, tile: u16) -> Option<Vec<(u8, bool)>> {
    let words = scene.metatiles().get(usize::from(tile))?;
    (0..256)
        .map(
            |at| match graphics::sample_metatile(words, scene.tiles(), at % 16, at / 16) {
                Ok(IndexedPixel::Opaque {
                    palette_index,
                    priority,
                }) => Some((palette_index, priority)),
                Ok(IndexedPixel::Transparent) => Some((0, false)),
                Err(_) => None,
            },
        )
        .collect()
}

fn static_rgb(index: u8, scene: &StaticBackground, at: (usize, usize)) -> u32 {
    let [r, g, b] = map_inspector::static_pixel_rgb(index, scene, at.0, at.1);
    u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b)
}

/// The ordinary map initialization copies staged palette color32 to backdrop0.
/// Keep the native policy explicit and map-A-only; static exports retain color0.
fn exterior_backdrop(image: &[u8], scene: &StaticBackground) -> Result<u32, String> {
    // $8D:8C52..8C62: LDA $7F0640 / STA $7F0600, then the high byte.
    // Source palette handler $86:8903 stages colors at $7F0600. The map-A
    // validated palette load supplies entries32..127, so this is its first word.
    const COPY: &[u8] = &[
        0xAF, 0x40, 0x06, 0x7F, 0x8F, 0x00, 0x06, 0x7F, 0xAF, 0x41, 0x06, 0x7F, 0x8F, 0x01, 0x06,
        0x7F, 0x60,
    ];
    if image.get(0xD_8C52..0xD_8C52 + COPY.len()) != Some(COPY) {
        return Err("unsupported exterior backdrop initialization".into());
    }
    Ok(rgb(scene.palette()[32]))
}

fn composite_backdrop(pixels: &mut [u32], indices: &[u8], color: u32) -> Result<(), String> {
    if pixels.len() != indices.len() {
        return Err("background color/index extent mismatch".into());
    }
    for (pixel, index) in pixels.iter_mut().zip(indices) {
        if *index == 0 {
            *pixel = color;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sky_darkens_less_every_three_lines() {
        // `$97:B4BA`: 32 entries `03 vv`, `vv` from `$FF` down to `$E0`.
        assert_eq!(
            [0, 2, 3, 92, 93, 223].map(sky_subtraction),
            [31, 31, 30, 1, 0, 0]
        );
    }

    #[test]
    fn colour_addition_saturates_each_channel() {
        assert_eq!(super::add(0x0010_2030, 0x0001_0203), 0x0011_2233);
        assert_eq!(super::add(0x00F0_8000, 0x0020_9001), 0x00FF_FF01);
    }

    #[test]
    #[ignore = "requires owned JP ROM: set CRYSTA_JP_ROM"]
    fn dirty_updates_match_full_source_render_including_wrap_and_reentry() {
        let bytes = std::fs::read(std::env::var("CRYSTA_JP_ROM").unwrap()).unwrap();
        let rom = rom::Rom::load(&bytes).unwrap();
        let mut cached = load(&rom, 0xA, &crysta_runtime::world::new_game_flags()).unwrap();
        let base = StaticBackground::from_rom(rom.image(), 0xA).unwrap();
        let source =
            assets::maps::visual::crysta_animation::CrystaAnimation::from_rom(rom.image()).unwrap();
        let backdrop = exterior_backdrop(rom.image(), &base).unwrap();
        for age in [0, 1, 2, 8, 42, 64, 528, 3, 2, 1, 1, 0] {
            cached.update(age);
            let mut tiles = base.tiles().to_vec();
            let mut palette = *base.palette();
            source.apply(age, &mut tiles, &mut palette).unwrap();
            for y in 0..cached.frame.height {
                for x in 0..cached.frame.width {
                    let cell = base.layer().cells()[y / 16 * base.layer().width() + x / 16];
                    let (color, high) = match graphics::sample_metatile(
                        &base.metatiles()[usize::from(cell.raw() & 511)],
                        &tiles,
                        x % 16,
                        y % 16,
                    )
                    .unwrap()
                    {
                        IndexedPixel::Transparent => (backdrop, false),
                        IndexedPixel::Opaque {
                            palette_index,
                            priority,
                        } => (rgb(palette[usize::from(palette_index)]), priority),
                    };
                    let i = y * cached.frame.width + x;
                    assert_eq!(
                        (cached.frame.pixels[i], cached.frame.high[i]),
                        (color, high),
                        "age {age}, pixel {x},{y}"
                    );
                }
            }
        }
        // A non-exterior map remains byte-for-byte the existing static baseline.
        let mut indoor = load(&rom, 0xB, &crysta_runtime::world::new_game_flags()).unwrap();
        let original = indoor.frame.pixels.clone();
        indoor.update(100);
        assert_eq!(indoor.frame.pixels, original);
        let static_bg = map_inspector::render_static_background(&rom, 0xB).unwrap();
        assert_eq!(
            indoor.frame.pixels,
            crate::frame::decode_bmp(&static_bg.bitmap).unwrap().pixels
        );
    }

    #[test]
    fn visit_clock_resets_on_entry_and_advances_independently_of_drawing() {
        let mut clock = VisitClock::new(0xF);
        assert_eq!(clock.tick(), 0);
        clock.advance(0xF);
        assert_eq!(clock.tick(), 1);
        clock.advance(0xA);
        assert_eq!(clock.tick(), 0);
        for _ in 0..20 {
            clock.advance(0xA);
        }
        assert_eq!(clock.tick(), 20);
        clock.advance(0xB);
        clock.advance(0xA);
        assert_eq!(clock.tick(), 0);
    }

    #[test]
    fn only_transparent_indices_receive_the_backdrop() {
        let mut pixels = [0x0018_2228, 0x0024_2F37, 0x0018_2228];
        composite_backdrop(&mut pixels, &[0, 0, 1], 0x0012_3456).unwrap();
        assert_eq!(pixels, [0x0012_3456, 0x0012_3456, 0x0018_2228]);
    }

    #[test]
    fn mismatched_planes_do_not_partially_change_pixels() {
        let mut pixels = [1, 2];
        assert!(composite_backdrop(&mut pixels, &[0], 3).is_err());
        assert_eq!(pixels, [1, 2]);
    }

    #[test]
    #[ignore = "requires owned JP ROM: set CRYSTA_JP_ROM"]
    fn backdrop_is_read_from_source_and_changed_consumer_is_refused() {
        let bytes = std::fs::read(std::env::var("CRYSTA_JP_ROM").unwrap()).unwrap();
        let rom = rom::Rom::load(&bytes).unwrap();
        let mut image = rom.image().to_vec();
        let scene = StaticBackground::from_rom(&image, 0xA).unwrap();
        let palette = scene.resources()[1].source_range().start;
        image[palette..palette + 2].copy_from_slice(&0x001Fu16.to_le_bytes());
        let scene = StaticBackground::from_rom(&image, 0xA).unwrap();
        assert_eq!(exterior_backdrop(&image, &scene).unwrap(), 0x00FF_0000);
        image[0xD_8C53] ^= 1;
        assert!(exterior_backdrop(&image, &scene).is_err());
    }
}

#[cfg(test)]
mod front_tests {
    use super::*;

    #[test]
    #[ignore = "requires owned JP ROM: set CRYSTA_JP_ROM"]
    fn a_tower_floors_second_layer_stands_in_front() {
        // `$101` (`docs/tower-second-layer.md` §3): an opaque L2 pixel wins
        // unless it is low over a high L1 pixel; its priority is kept.
        let bytes = std::fs::read(std::env::var("CRYSTA_JP_ROM").unwrap()).unwrap();
        let rom = rom::Rom::load(&bytes).unwrap();
        let cached = load(&rom, 0x101, &crysta_runtime::world::new_game_flags()).unwrap();
        let scene = assets::maps::visual::first_background(rom.image(), 0x101).unwrap();
        let front = SecondLayer::from_rom(rom.image(), 0x101).unwrap();
        let width = front.layer().width();
        let (mut fronts, mut highs) = (0, 0);
        for y in 0..cached.frame.height {
            for x in 0..cached.frame.width {
                let cell = front.layer().cells()[y / 16 * width + x / 16];
                let words = &front.metatiles()[usize::from(cell.raw() & 511)];
                let Ok(IndexedPixel::Opaque {
                    palette_index,
                    priority,
                }) = graphics::sample_metatile(words, scene.tiles(), x % 16, y % 16)
                else {
                    continue;
                };
                let first_high = matches!(
                    scene.pixel(x, y),
                    Ok(IndexedPixel::Opaque { priority: true, .. })
                );
                let at = y * cached.frame.width + x;
                if priority || !first_high {
                    assert_eq!(
                        cached.frame.pixels[at],
                        rgb(scene.palette()[usize::from(palette_index)]),
                        "({x}, {y})"
                    );
                    assert_eq!(cached.frame.high[at], priority);
                    fronts += 1;
                    highs += usize::from(priority);
                }
            }
        }
        assert!(fronts > 1000 && highs > 100, "{fronts} {highs}");
    }
}

#[cfg(test)]
mod second_patch_tests {
    use super::*;

    #[test]
    #[ignore = "requires owned JP ROM: set CRYSTA_JP_ROM"]
    fn a_second_layer_patch_redraws_the_front_layer() {
        // `$101`'s door (`docs/block-patch.md`): its second-layer cells go
        // empty; the first layer shows there, and comes back unpatched.
        let bytes = std::fs::read(std::env::var("CRYSTA_JP_ROM").unwrap()).unwrap();
        let rom = rom::Rom::load(&bytes).unwrap();
        let mut cached = load(&rom, 0x101, &crysta_runtime::world::new_game_flags()).unwrap();
        let scene = assets::maps::visual::first_background(rom.image(), 0x101).unwrap();
        let cell = |cached: &CachedBackground| {
            let width = cached.frame.width;
            (0..256)
                .map(|at| cached.frame.pixels[(4 * 16 + at / 16) * width + 7 * 16 + at % 16])
                .collect::<Vec<_>>()
        };
        let before = cell(&cached);
        cached.apply_patches(rom.image(), 0x101, &[], &[(7, 4, 0)]);
        let first: Vec<u32> = (0..256)
            .map(|at| {
                let (x, y) = (7 * 16 + at % 16, 4 * 16 + at / 16);
                let index = match scene.pixel(x, y).unwrap() {
                    IndexedPixel::Transparent => 0,
                    IndexedPixel::Opaque { palette_index, .. } => palette_index,
                };
                static_rgb(index, &scene, (x, y))
            })
            .collect();
        assert_eq!(cell(&cached), first);
        assert_ne!(first, before, "the door covered the first layer");
        cached.apply_patches(rom.image(), 0x101, &[], &[]);
        assert_eq!(cell(&cached), before);
    }
}

#[cfg(test)]
mod patch_tests {
    #[test]
    #[ignore = "requires owned JP ROM: set CRYSTA_JP_ROM"]
    fn a_patched_cell_is_redrawn_and_restored() {
        let bytes = std::fs::read(std::env::var("CRYSTA_JP_ROM").unwrap()).unwrap();
        let rom = rom::Rom::load(&bytes).unwrap();
        let mut cached = super::load(&rom, 0xC, &crysta_runtime::world::new_game_flags()).unwrap();
        let cell = |cached: &super::CachedBackground| {
            let frame = &cached.frame;
            (0..16)
                .flat_map(|y| (0..16).map(move |x| (21 * 16 + y) * frame.width + 11 * 16 + x))
                .map(|at| frame.pixels[at])
                .collect::<Vec<_>>()
        };
        let before = cell(&cached);
        // The blue door's broken tile from its second hit; the cell holds $181.
        cached.apply_patches(rom.image(), 0xC, &[(11, 21, 0xCB)], &[]);
        assert_ne!(cell(&cached), before);
        cached.apply_patches(rom.image(), 0xC, &[], &[]);
        assert_eq!(cell(&cached), before, "restored when the patch is gone");
    }

    #[test]
    #[ignore = "requires owned JP ROM: set CRYSTA_JP_ROM"]
    fn the_first_layer_renders_as_the_inspector_exports_it_and_the_tour_loads() {
        let bytes = std::fs::read(std::env::var("CRYSTA_JP_ROM").unwrap()).unwrap();
        let rom = rom::Rom::load(&bytes).unwrap();
        let exported = map_inspector::render_static_background(&rom, 0xF).unwrap();
        let reference = crate::frame::decode_bmp(&exported.bitmap).unwrap();
        let loaded = super::load(&rom, 0xF, &crysta_runtime::world::new_game_flags()).unwrap();
        assert_eq!(loaded.frame.pixels, reference.pixels);
        let priorities: Vec<bool> = exported.priorities.iter().map(|bit| *bit != 0).collect();
        assert_eq!(loaded.frame.high, priorities);
        for map in 0x41..=0x44 {
            assert!(
                super::load(&rom, map, &crysta_runtime::world::new_game_flags()).is_ok(),
                "map {map:#x}"
            );
        }
    }

    #[test]
    #[ignore = "requires owned JP ROM: set CRYSTA_JP_ROM"]
    fn the_underworld_loads_as_its_flat_mode_7_plane() {
        let bytes = std::fs::read(std::env::var("CRYSTA_JP_ROM").unwrap()).unwrap();
        let rom = rom::Rom::load(&bytes).unwrap();
        let loaded = super::load(&rom, 0x3, &crysta_runtime::world::new_game_flags()).unwrap();
        assert!(loaded.world);
        assert_eq!((loaded.frame.width, loaded.frame.height), (1024, 1024));
        assert_eq!(loaded.region.bounds, [0, 0, 1024, 1024]);
        let world = assets::maps::visual::world::WorldMap::from_rom(rom.image(), 0x3).unwrap();
        let (x, y) = (536, 530);
        let expected = super::rgb(world.color(world.pixel(x, y)));
        assert_eq!(loaded.frame.pixels[y * 1024 + x], expected);
    }
}
