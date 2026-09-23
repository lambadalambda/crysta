//! Native background presentation, separate from the asset inspector's checkerboard.
use assets::graphics::{self, Bgr555, IndexedPixel, Tile4bpp};
use assets::maps::visual::{
    camera::CameraRegion, crysta_animation::CrystaAnimation, StaticBackground,
};

/// Presentation age since entry, advanced by host simulation updates, not redraws.
pub struct VisitClock {
    map: u16,
    tick: u64,
}

impl VisitClock {
    pub const fn new(map: u16) -> Self {
        Self { map, tick: 0 }
    }

    pub fn advance(&mut self, map: u16) {
        if self.map == map {
            self.tick = self.tick.saturating_add(1);
        } else {
            *self = Self::new(map);
        }
    }

    pub const fn tick(&self) -> u64 {
        self.tick
    }
}

/// A world map's flat Mode 7 plane, its whole extent as the region. The
/// native view is in perspective and wraps; neither is drawn.
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
        patches: Patches::default(),
        world: true,
    })
}

/// Load the static baseline without changing the inspector's export policy.
pub fn load(cartridge: &rom::Rom, map: u16) -> Result<CachedBackground, String> {
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
    let animation = if map == 0xA {
        let color = exterior_backdrop(cartridge.image(), &scene)?;
        composite_backdrop(&mut background.pixels, &indices, color)?;
        Some(AnimatedExterior::new(cartridge.image(), scene, color)?)
    } else {
        None
    };
    Ok(CachedBackground {
        frame: background,
        region,
        animation,
        patches: Patches::default(),
        world: false,
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

/// Cells the world's scripts have re-tiled, and what they looked like.
#[derive(Default)]
struct Patches {
    /// The map's static scene, decoded on the first patch.
    scene: Option<StaticBackground>,
    /// Cells drawn with a patched tile: column, row, tile.
    drawn: Vec<(u16, u16, u16)>,
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
    pub frame: crate::frame::Background,
    /// The part of the shared layer this map's camera may show.
    pub region: CameraRegion,
    animation: Option<AnimatedExterior>,
    patches: Patches,
    /// A world map: its camera follows the player unclamped (`$87:9123`).
    pub world: bool,
}

impl CachedBackground {
    /// The camera for the player: a world map's, or the region's clamp.
    pub fn camera(&self, player: (u16, u16), width: usize) -> (i32, i32) {
        if self.world {
            crate::frame::world_camera(player, width)
        } else {
            crate::frame::camera(&self.region, player, width)
        }
    }

    /// Draws the world's patched cells (`COP 44`), and restores cells that
    /// are no longer patched, from the map's own metatiles.
    pub fn apply_patches(&mut self, image: &[u8], map: u16, patched: &[(u16, u16, u16)]) {
        if self.patches.drawn == patched {
            return;
        }
        self.patches.scene(image, map);
        let Some(scene) = &self.patches.scene else {
            return;
        };
        let width = scene.layer().width();
        let original = |column: u16, row: u16| {
            let cell = usize::from(row) * width + usize::from(column);
            scene.layer().cells().get(cell).map(|cell| cell.raw() & 511)
        };
        let mut redraw: Vec<(u16, u16, u16)> = self
            .patches
            .drawn
            .iter()
            .filter(|&&(column, row, _)| !patched.iter().any(|&(c, r, _)| (c, r) == (column, row)))
            .filter_map(|&(column, row, _)| Some((column, row, original(column, row)?)))
            .collect();
        redraw.extend(
            patched
                .iter()
                .filter(|cell| !self.patches.drawn.contains(cell)),
        );
        let backdrop = self.animation.as_ref().map(|animation| animation.backdrop);
        for (column, row, tile) in redraw {
            let at = (usize::from(column), usize::from(row));
            draw_metatile(scene, &mut self.frame, at, tile, backdrop);
        }
        self.patches.drawn = patched.to_vec();
    }

    /// A lifted object's metatile as a sprite standing on its bottom centre,
    /// cut out of the floor it leaves (`floor`): a stand-in for a pot until
    /// its own sprite (`COP D8 $A2C000`) is decoded.
    pub fn lifted_raster(
        &mut self,
        image: &[u8],
        map: u16,
        (tile, floor): (u16, u16),
    ) -> Option<crysta_runtime::art::Raster> {
        let scene = self.patches.scene(image, map)?;
        let under = metatile_pixels(scene, floor)?;
        let pixels = metatile_pixels(scene, tile)?
            .into_iter()
            .zip(under)
            .enumerate()
            .map(|(at, ((index, _), (floor, _)))| {
                if index == floor {
                    0
                } else {
                    0xFF00_0000 | static_rgb(index, scene, (at % 16, at / 16))
                }
            })
            .collect();
        Some(crysta_runtime::art::Raster {
            width: 16,
            height: 16,
            offset: (-8, -16),
            pixels,
        })
    }

    pub fn update(&mut self, age: u64) {
        if let Some(animation) = &mut self.animation {
            animation.update(age, &mut self.frame);
        }
    }
}

struct AnimatedExterior {
    scene: StaticBackground,
    source: CrystaAnimation,
    tiles: Vec<Tile4bpp>,
    palette: [Bgr555; 128],
    backdrop: u32,
    // Map cell index and bitmask of source phase-key slots it depends on.
    cells: Vec<(usize, u8)>,
    key: Option<[Option<u64>; 7]>,
}

impl AnimatedExterior {
    fn new(image: &[u8], scene: StaticBackground, backdrop: u32) -> Result<Self, String> {
        let source = CrystaAnimation::from_rom(image).map_err(|error| error.to_string())?;
        let masks: Vec<u8> = scene
            .metatiles()
            .iter()
            .map(|words| {
                words.iter().fold(0, |mask, word| {
                    // CrystaAnimation::phase_key documents these disjoint destinations.
                    let graphics = match word.tile_index() {
                        9..=12 => 1,
                        tile @ 496..=511 => 1 << (1 + (tile - 496) / 4),
                        _ => 0,
                    };
                    let palette = match word.palette() {
                        6 => 1 << 5,
                        7 => 1 << 6,
                        _ => 0,
                    };
                    mask | graphics | palette
                })
            })
            .collect();
        let cells = scene
            .layer()
            .cells()
            .iter()
            .enumerate()
            .filter_map(|(i, cell)| {
                let mask = masks[usize::from(cell.raw() & 511)];
                (mask != 0).then_some((i, mask))
            })
            .collect();
        Ok(Self {
            tiles: scene.tiles().to_vec(),
            palette: *scene.palette(),
            scene,
            source,
            backdrop,
            cells,
            key: None,
        })
    }

    fn update(&mut self, age: u64, frame: &mut crate::frame::Background) {
        let key = self.source.phase_key(age);
        let changed = (0..7).fold(0u8, |mask, i| {
            mask | if self.key.is_none_or(|old| old[i] != key[i]) {
                1 << i
            } else {
                0
            }
        });
        if changed == 0 {
            return;
        }
        // Reset before random seeking, including re-entry into startup ages0..2.
        // Only 768 tiles; no ROM decode, allocation or elapsed-frame replay here.
        self.tiles.copy_from_slice(self.scene.tiles());
        self.palette = *self.scene.palette();
        self.source
            .apply(age, &mut self.tiles, &mut self.palette)
            .expect("validated animation destination extents");
        let width = self.scene.layer().width();
        for &(cell, dependencies) in &self.cells {
            if dependencies & changed == 0 {
                continue;
            }
            let words =
                &self.scene.metatiles()[usize::from(self.scene.layer().cells()[cell].raw() & 511)];
            for y in 0..16 {
                for x in 0..16 {
                    let (color, high) = match graphics::sample_metatile(words, &self.tiles, x, y)
                        .expect("validated map definitions and tile extents")
                    {
                        IndexedPixel::Transparent => (self.backdrop, false),
                        IndexedPixel::Opaque {
                            palette_index,
                            priority,
                        } => (rgb(self.palette[usize::from(palette_index)]), priority),
                    };
                    let offset = (cell / width * 16 + y) * frame.width + cell % width * 16 + x;
                    frame.pixels[offset] = color;
                    frame.high[offset] = high;
                }
            }
        }
        self.key = Some(key);
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

fn rgb(color: Bgr555) -> u32 {
    let [r, g, b] = color.rgb8();
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
    #[ignore = "requires owned JP ROM: set CRYSTA_JP_ROM"]
    fn dirty_updates_match_full_source_render_including_wrap_and_reentry() {
        let bytes = std::fs::read(std::env::var("CRYSTA_JP_ROM").unwrap()).unwrap();
        let rom = rom::Rom::load(&bytes).unwrap();
        let mut cached = load(&rom, 0xA).unwrap();
        let base = StaticBackground::from_rom(rom.image(), 0xA).unwrap();
        let source = CrystaAnimation::from_rom(rom.image()).unwrap();
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
        let mut indoor = load(&rom, 0xB).unwrap();
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
mod patch_tests {
    #[test]
    #[ignore = "requires owned JP ROM: set CRYSTA_JP_ROM"]
    fn a_patched_cell_is_redrawn_and_restored() {
        let bytes = std::fs::read(std::env::var("CRYSTA_JP_ROM").unwrap()).unwrap();
        let rom = rom::Rom::load(&bytes).unwrap();
        let mut cached = super::load(&rom, 0xC).unwrap();
        let cell = |cached: &super::CachedBackground| {
            let frame = &cached.frame;
            (0..16)
                .flat_map(|y| (0..16).map(move |x| (21 * 16 + y) * frame.width + 11 * 16 + x))
                .map(|at| frame.pixels[at])
                .collect::<Vec<_>>()
        };
        let before = cell(&cached);
        // The blue door's broken tile from its second hit; the cell holds $181.
        cached.apply_patches(rom.image(), 0xC, &[(11, 21, 0xCB)]);
        assert_ne!(cell(&cached), before);
        cached.apply_patches(rom.image(), 0xC, &[]);
        assert_eq!(cell(&cached), before, "restored when the patch is gone");
    }

    #[test]
    #[ignore = "requires owned JP ROM: set CRYSTA_JP_ROM"]
    fn the_first_layer_renders_as_the_inspector_exports_it_and_the_tour_loads() {
        let bytes = std::fs::read(std::env::var("CRYSTA_JP_ROM").unwrap()).unwrap();
        let rom = rom::Rom::load(&bytes).unwrap();
        let exported = map_inspector::render_static_background(&rom, 0xF).unwrap();
        let reference = crate::frame::decode_bmp(&exported.bitmap).unwrap();
        let loaded = super::load(&rom, 0xF).unwrap();
        assert_eq!(loaded.frame.pixels, reference.pixels);
        let priorities: Vec<bool> = exported.priorities.iter().map(|bit| *bit != 0).collect();
        assert_eq!(loaded.frame.high, priorities);
        for map in 0x41..=0x44 {
            assert!(super::load(&rom, map).is_ok(), "map {map:#x}");
        }
    }

    #[test]
    #[ignore = "requires owned JP ROM: set CRYSTA_JP_ROM"]
    fn the_underworld_loads_as_its_flat_mode_7_plane() {
        let bytes = std::fs::read(std::env::var("CRYSTA_JP_ROM").unwrap()).unwrap();
        let rom = rom::Rom::load(&bytes).unwrap();
        let loaded = super::load(&rom, 0x3).unwrap();
        assert!(loaded.world);
        assert_eq!((loaded.frame.width, loaded.frame.height), (1024, 1024));
        assert_eq!(loaded.region.bounds, [0, 0, 1024, 1024]);
        let world = assets::maps::visual::world::WorldMap::from_rom(rom.image(), 0x3).unwrap();
        let (x, y) = (536, 530);
        let expected = super::rgb(world.color(world.pixel(x, y)));
        assert_eq!(loaded.frame.pixels[y * 1024 + x], expected);
    }

    #[test]
    #[ignore = "requires owned JP ROM: set CRYSTA_JP_ROM"]
    fn a_pot_tile_is_a_sprite_standing_on_its_bottom_centre() {
        let bytes = std::fs::read(std::env::var("CRYSTA_JP_ROM").unwrap()).unwrap();
        let rom = rom::Rom::load(&bytes).unwrap();
        let mut cached = super::load(&rom, 0xC).unwrap();
        let pot = cached
            .lifted_raster(rom.image(), 0xC, (0xFA, 0xF8))
            .unwrap();
        assert_eq!((pot.width, pot.height, pot.offset), (16, 16, (-8, -16)));
        assert!(pot.is_visible());
        assert!(
            pot.pixels.iter().any(|pixel| pixel >> 24 == 0),
            "the floor is cut out"
        );
        // The pot's own cell at (3,21) in the static background.
        let frame = &cached.frame;
        let (x, y) = (3 * 16 + 8, 21 * 16 + 8);
        assert_eq!(
            pot.pixels[8 * 16 + 8] & 0x00FF_FFFF,
            frame.pixels[y * frame.width + x]
        );
    }
}
