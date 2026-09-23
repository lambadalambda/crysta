//! Composing one 224-line view, classic or wide, without a window in sight.
//!
//! Everything here is a pure function over decoded pixels, so the renderer can
//! be tested without opening a window or owning a GPU.

use assets::maps::visual::camera::CameraRegion;
use assets::text::Placement;

/// Classic view width in pixels.
pub const CLASSIC_WIDTH: usize = 256;
/// Wide view width: 224 lines at about 16:9, in whole 8-pixel tiles.
pub const WIDE_WIDTH: usize = 400;
/// View height in pixels, in both widths.
pub const VIEW_HEIGHT: usize = 224;

/// One composed view, `width` by [`VIEW_HEIGHT`], in `0x00RRGGBB`.
pub struct Canvas {
    /// Row-major pixels.
    pub pixels: Vec<u32>,
    /// Width in pixels.
    pub width: usize,
}

impl Canvas {
    /// A blank view `width` pixels across.
    #[must_use]
    pub fn new(width: usize) -> Self {
        Self {
            pixels: vec![0; width * VIEW_HEIGHT],
            width,
        }
    }

    /// Sets one pixel; anything off the view is dropped rather than wrapped.
    fn set(&mut self, (x, y): (i32, i32), colour: u32) {
        if let (Ok(x), Ok(y)) = (usize::try_from(x), usize::try_from(y)) {
            if x < self.width && y < VIEW_HEIGHT {
                self.pixels[y * self.width + x] = colour;
            }
        }
    }
}

fn signed(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

/// A decoded background: one map's pixels, in `0x00RRGGBB`.
pub struct Background {
    /// Row-major pixels, `width * height` of them.
    pub pixels: Vec<u32>,
    /// Width in pixels.
    pub width: usize,
    /// Height in pixels.
    pub height: usize,
    /// Row-major, one per pixel: whether the tile there is opaque and high
    /// priority, which draws over an ordinary sprite. Empty means none is.
    pub high: Vec<bool>,
}

impl Background {
    /// Whether the background at a world pixel covers an ordinary sprite.
    #[must_use]
    pub fn occludes(&self, x: usize, y: usize) -> bool {
        x < self.width && self.high.get(y * self.width + x).copied().unwrap_or(false)
    }

    /// The pixel at a world position, or `None` off the layer.
    fn at(&self, x: i32, y: i32) -> Option<u32> {
        let (x, y) = (usize::try_from(x).ok()?, usize::try_from(y).ok()?);
        (x < self.width && y < self.height).then(|| self.pixels[y * self.width + x])
    }
}

/// Top-left of a view `width` pixels across, in layer pixels.
///
/// At the classic width this is the source clamp. A wider view follows the
/// player the same way inside a region wider than itself, and centres a
/// narrower region, so the origin can be left of the layer.
#[must_use]
pub fn camera(region: &CameraRegion, player: (u16, u16), width: usize) -> (i32, i32) {
    let [_, y] = region.settled_origin([player.0, player.1]);
    let [left, _, right, _] = region.bounds.map(i32::from);
    let width = signed(width);
    let x = if right - left <= width {
        left - (width - (right - left)) / 2
    } else {
        (i32::from(player.0) - width / 2).clamp(left, right - width)
    };
    (x, i32::from(y))
}

/// Blanks every pixel whose layer position is outside `bounds`, so a wide
/// view never shows a neighbouring room.
pub fn mask_outside(canvas: &mut Canvas, camera: (i32, i32), bounds: [u16; 4]) {
    let [left, top, right, bottom] = bounds.map(i32::from);
    for (row, line) in canvas.pixels.chunks_mut(canvas.width).enumerate() {
        let y = camera.1 + signed(row);
        for (column, pixel) in line.iter_mut().enumerate() {
            let x = camera.0 + signed(column);
            if !(left..right).contains(&x) || !(top..bottom).contains(&y) {
                *pixel = 0;
            }
        }
    }
}

/// Blits a sprite raster at a world position, occluded by the background.
///
/// This is the first-background compositor the browser viewer qualified,
/// narrowed to what the slice draws: every sprite is ordinary priority, so a
/// pixel under an opaque high-priority tile is the tile's. Later sprites in
/// the draw list overwrite earlier ones where both are opaque.
pub fn draw_sprite(
    canvas: &mut Canvas,
    background: &Background,
    camera: (i32, i32),
    raster: &crysta_runtime::art::Raster,
    at: (u16, u16),
) {
    for row in 0..raster.height {
        for column in 0..raster.width {
            let Some(pixel) = raster.pixels.get(row * raster.width + column) else {
                return;
            };
            if pixel >> 24 == 0 {
                continue;
            }
            let world_x = i32::from(at.0) + i32::from(raster.offset.0) + signed(column);
            let world_y = i32::from(at.1) + i32::from(raster.offset.1) + signed(row);
            let (Ok(x), Ok(y)) = (usize::try_from(world_x), usize::try_from(world_y)) else {
                continue;
            };
            if !background.occludes(x, y) {
                canvas.set(
                    (world_x - camera.0, world_y - camera.1),
                    pixel & 0x00FF_FFFF,
                );
            }
        }
    }
}

/// Colours for a dialogue page's four indices: black, text, shadow, and the
/// page's own background index, which takes the box colour.
///
/// A presentation policy, as the browser viewer's is: the ROM's font pixels
/// at high contrast, not the SNES window or its colours.
const PAGE_PALETTE: [u32; 4] = [0x0000_0000, 0x00F0_F4F8, 0x0024_2F37, 0x0010_1828];
/// The box behind a page.
const PAGE_BOX: u32 = 0x0010_1828;
/// The box's border.
const PAGE_BORDER: u32 = 0x00F0_F4F8;
/// Margin between the view's edge and the box, and between the box and its page.
const PAGE_MARGIN: usize = 8;

/// Where a page's box goes, from the page's native placement.
///
/// The standard window is centred along the bottom. `$DA` moves it to the
/// top when the player's screen row is in the lower half, as `$85964D` does
/// against the camera. A `$C2` window puts its content at its tile column
/// and row, with the box drawn around it.
///
/// In a wide view the page keeps to the classic area in the middle.
#[must_use]
pub fn page_origin(
    placement: Placement,
    (width, height): (usize, usize),
    player_screen_y: usize,
    view_width: usize,
) -> (usize, usize) {
    let classic = view_width.saturating_sub(CLASSIC_WIDTH) / 2;
    let box_width = (width + 2 * PAGE_MARGIN).min(CLASSIC_WIDTH);
    let box_height = (height + 2 * PAGE_MARGIN).min(VIEW_HEIGHT);
    let centred = classic + (CLASSIC_WIDTH - box_width) / 2;
    let bottom = VIEW_HEIGHT.saturating_sub(box_height + PAGE_MARGIN);
    let top = match placement {
        Placement::AwayFromPlayer if player_screen_y >= VIEW_HEIGHT / 2 => PAGE_MARGIN,
        Placement::Bottom | Placement::AwayFromPlayer => bottom,
        Placement::Tile { column, row } => {
            return (
                classic + (usize::from(column) * 8).saturating_sub(PAGE_MARGIN),
                (usize::from(row) * 8).saturating_sub(PAGE_MARGIN),
            )
        }
    };
    (centred, top)
}

/// Draws a dialogue page in a box whose top-left corner is `origin`.
///
/// `indexed` is the page's row-major two-bit pixels, `width * height` of
/// them, and `background_index` is the index that reads as clear.
pub fn draw_page(
    canvas: &mut Canvas,
    indexed: &[u8],
    (width, height): (usize, usize),
    background_index: u8,
    (left, top): (usize, usize),
) {
    if width == 0 || height == 0 || indexed.len() < width * height {
        return;
    }
    let (left, top) = (signed(left), signed(top));
    let box_width = signed((width + 2 * PAGE_MARGIN).min(CLASSIC_WIDTH));
    let box_height = signed((height + 2 * PAGE_MARGIN).min(VIEW_HEIGHT));
    fill(canvas, (left, top), (box_width, box_height), PAGE_BORDER);
    fill(
        canvas,
        (left + 1, top + 1),
        (box_width - 2, box_height - 2),
        PAGE_BOX,
    );
    let mut palette = PAGE_PALETTE;
    if let Some(slot) = palette.get_mut(usize::from(background_index)) {
        *slot = PAGE_BOX;
    }
    let margin = signed(PAGE_MARGIN);
    // A page larger than its box shows only what fits inside the margins.
    let shown = (
        width.min(CLASSIC_WIDTH - 2 * PAGE_MARGIN),
        height.min(VIEW_HEIGHT - 2 * PAGE_MARGIN),
    );
    for row in 0..shown.1 {
        for column in 0..shown.0 {
            let index = usize::from(indexed[row * width + column]) & 3;
            let at = (left + margin + signed(column), top + margin + signed(row));
            canvas.set(at, palette[index]);
        }
    }
}

/// Draws the choice cursor, a small arrow in the text colour, at a
/// page-relative position of the page whose box is at `origin`.
pub fn draw_cursor(canvas: &mut Canvas, (left, top): (usize, usize), [x, y]: [u16; 2]) {
    let (x, y) = (
        signed(left + PAGE_MARGIN) + i32::from(x),
        signed(top + PAGE_MARGIN) + i32::from(y),
    );
    for row in 0..7 {
        let width = 4 - (row - 3i32).abs();
        fill(canvas, (x, y + row), (width, 1), PAGE_PALETTE[1]);
    }
}

/// Decodes a 24-bit top-down BMP, which is what the qualified renderer emits.
///
/// Returns `None` for anything that is not that exact shape rather than
/// guessing at a format.
#[must_use]
pub fn decode_bmp(bytes: &[u8]) -> Option<Background> {
    // BITMAPFILEHEADER is 14 bytes; BITMAPINFOHEADER follows.
    let offset = u32::from_le_bytes(bytes.get(10..14)?.try_into().ok()?) as usize;
    let width = i32::from_le_bytes(bytes.get(18..22)?.try_into().ok()?);
    let height = i32::from_le_bytes(bytes.get(22..26)?.try_into().ok()?);
    let depth = u16::from_le_bytes(bytes.get(28..30)?.try_into().ok()?);
    if depth != 24 || width <= 0 {
        return None;
    }
    // A negative height is top-down, which is what the exporter writes.
    let top_down = height < 0;
    let (width, height) = (
        usize::try_from(width).ok()?,
        usize::try_from(height.unsigned_abs()).ok()?,
    );
    // Rows are padded to four bytes.
    let stride = (width * 3).next_multiple_of(4);
    let mut pixels = vec![0u32; width * height];
    for row in 0..height {
        let source = if top_down { row } else { height - 1 - row };
        let start = offset + source * stride;
        let line = bytes.get(start..start + width * 3)?;
        for (column, bgr) in line.chunks_exact(3).enumerate() {
            pixels[row * width + column] =
                u32::from(bgr[2]) << 16 | u32::from(bgr[1]) << 8 | u32::from(bgr[0]);
        }
    }
    Some(Background {
        pixels,
        width,
        height,
        high: Vec::new(),
    })
}

/// Blits the visible window of `background` into the view.
pub fn draw_background(canvas: &mut Canvas, background: &Background, camera: (i32, i32)) {
    for row in 0..VIEW_HEIGHT {
        for column in 0..canvas.width {
            let (x, y) = (signed(column), signed(row));
            if let Some(pixel) = background.at(camera.0 + x, camera.1 + y) {
                canvas.set((x, y), pixel);
            }
        }
    }
}

/// Fills a rectangle, clipped to the view.
pub fn fill(canvas: &mut Canvas, at: (i32, i32), size: (i32, i32), colour: u32) {
    for row in 0..size.1 {
        for column in 0..size.0 {
            canvas.set((at.0 + column, at.1 + row), colour);
        }
    }
}

/// Window width that shows a `view_width` view at `height` without borders.
#[must_use]
pub fn fitted_width(view_width: usize, height: u32) -> u32 {
    let across = u64::from(height) * u64::try_from(view_width).unwrap_or(u64::MAX)
        / u64::try_from(VIEW_HEIGHT).unwrap_or(1);
    u32::try_from(across).unwrap_or(u32::MAX)
}

/// Scales the view into `target` by the largest integer factor that fits.
///
/// Nearest neighbour and integer only: a non-integer scale would resample
/// pixels the renderer went to some trouble to reproduce exactly.
pub fn present(canvas: &Canvas, target: &mut [u32], size: (usize, usize)) {
    let width = canvas.width;
    let scale = (size.0 / width).min(size.1 / VIEW_HEIGHT).max(1);
    let (drawn_width, drawn_height) = (width * scale, VIEW_HEIGHT * scale);
    let (left, top) = (
        size.0.saturating_sub(drawn_width) / 2,
        size.1.saturating_sub(drawn_height) / 2,
    );
    target.fill(0);
    for row in 0..drawn_height.min(size.1) {
        let source = row / scale;
        for column in 0..drawn_width.min(size.0) {
            let pixel = canvas.pixels[source * width + column / scale];
            let index = (top + row) * size.0 + left + column;
            if let Some(slot) = target.get_mut(index) {
                *slot = pixel;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region(bounds: [u16; 4]) -> CameraRegion {
        CameraRegion {
            record_offset: 0,
            bounds,
            vertical_extent: 256,
        }
    }

    fn filled(width: usize, colour: u32) -> Canvas {
        let mut canvas = Canvas::new(width);
        canvas.pixels.fill(colour);
        canvas
    }

    #[test]
    fn the_classic_camera_is_the_source_clamp() {
        for bounds in [[0, 0, 1024, 1024], [256, 512, 512, 768], [0, 0, 256, 512]] {
            let region = region(bounds);
            for x in (0..1100).step_by(37) {
                for y in (0..1100).step_by(41) {
                    let [cx, cy] = region.settled_origin([x, y]);
                    let expected = (i32::from(cx), i32::from(cy));
                    assert_eq!(camera(&region, (x, y), CLASSIC_WIDTH), expected);
                }
            }
        }
    }

    #[test]
    fn the_wide_camera_follows_wide_regions_and_centres_narrow_ones() {
        let exterior = region([0, 0, 1024, 1024]);
        assert_eq!(camera(&exterior, (504, 1000), WIDE_WIDTH), (304, 768));
        assert_eq!(camera(&exterior, (10, 10), WIDE_WIDTH), (0, 0));
        assert_eq!(camera(&exterior, (1020, 10), WIDE_WIDTH), (624, 0));
        // A one-page room sits in the middle, 72 columns in from each side.
        let room = region([256, 512, 512, 768]);
        for player in [(260, 520), (500, 760)] {
            assert_eq!(camera(&room, player, WIDE_WIDTH), (184, 512));
        }
        // Left of the layer's origin is fine: the camera is signed.
        let first = region([0, 256, 256, 512]);
        assert_eq!(camera(&first, (128, 300), WIDE_WIDTH), (-72, 256));
    }

    #[test]
    fn everything_outside_the_region_is_blanked() {
        let mut canvas = filled(WIDE_WIDTH, 0x00AB_CDEF);
        mask_outside(&mut canvas, (-72, 256), [0, 256, 256, 512]);
        for y in [0, VIEW_HEIGHT - 1] {
            let row = &canvas.pixels[y * WIDE_WIDTH..(y + 1) * WIDE_WIDTH];
            assert!(row[..72].iter().all(|pixel| *pixel == 0));
            assert!(row[72..328].iter().all(|pixel| *pixel == 0x00AB_CDEF));
            assert!(row[328..].iter().all(|pixel| *pixel == 0));
        }
        // Rows past the region's bottom go too; a classic view inside it is untouched.
        let mut canvas = filled(CLASSIC_WIDTH, 1);
        mask_outside(&mut canvas, (0, 400), [0, 256, 256, 512]);
        let (inside, below) = canvas.pixels.split_at(112 * CLASSIC_WIDTH);
        assert!(inside.iter().all(|pixel| *pixel == 1));
        assert!(below.iter().all(|pixel| *pixel == 0));
        let mut canvas = filled(CLASSIC_WIDTH, 1);
        mask_outside(&mut canvas, (0, 256), [0, 256, 256, 512]);
        assert!(canvas.pixels.iter().all(|pixel| *pixel == 1));
    }

    #[test]
    fn a_page_box_goes_where_the_native_window_opens() {
        let dims = (224, 48);
        let origin = |placement, y| page_origin(placement, dims, y, CLASSIC_WIDTH);
        // The standard window sits along the bottom, centred.
        assert_eq!(origin(Placement::Bottom, 30), (8, 224 - 64 - 8));
        assert_eq!(origin(Placement::Bottom, 200), (8, 224 - 64 - 8));
        // $DA keeps out of the player's half of the screen.
        assert_eq!(origin(Placement::AwayFromPlayer, 111), (8, 224 - 64 - 8));
        assert_eq!(origin(Placement::AwayFromPlayer, 112), (8, 8));
        // $C2 puts the content at its tile column and row.
        let tile = Placement::Tile { column: 3, row: 3 };
        assert_eq!(
            page_origin(tile, (200, 48), 0, CLASSIC_WIDTH),
            (24 - 8, 24 - 8)
        );
        assert_eq!(origin(Placement::Tile { column: 0, row: 0 }, 0), (0, 0));
    }

    #[test]
    fn wide_pages_stay_in_the_classic_area() {
        let dims = (224, 48);
        let bottom = page_origin(Placement::Bottom, dims, 30, WIDE_WIDTH);
        assert_eq!(bottom, (72 + 8, 224 - 64 - 8));
        let tile = Placement::Tile { column: 3, row: 3 };
        assert_eq!(page_origin(tile, (200, 48), 0, WIDE_WIDTH), (72 + 16, 16));
    }

    #[test]
    fn presenting_uses_an_integer_scale_and_centres_the_image() {
        let canvas = filled(CLASSIC_WIDTH, 0x00FF_00FF);
        // A 3x window leaves a border, and the image sits inside it.
        let size = (CLASSIC_WIDTH * 3 + 40, VIEW_HEIGHT * 3 + 20);
        let mut target = vec![0u32; size.0 * size.1];
        present(&canvas, &mut target, size);
        let drawn = target.iter().filter(|pixel| **pixel != 0).count();
        assert_eq!(drawn, CLASSIC_WIDTH * 3 * VIEW_HEIGHT * 3, "exactly 3x");
        // The corners are border, the middle is image.
        assert_eq!(target[0], 0);
        assert_eq!(target[size.1 / 2 * size.0 + size.0 / 2], 0x00FF_00FF);
    }

    #[test]
    fn a_wide_view_fills_a_16_by_9_window_at_an_integer_scale() {
        let canvas = filled(WIDE_WIDTH, 0x00FF_00FF);
        let size = (1920, 1080);
        let mut target = vec![0u32; size.0 * size.1];
        present(&canvas, &mut target, size);
        let drawn = target.iter().filter(|pixel| **pixel != 0).count();
        assert_eq!(drawn, WIDE_WIDTH * 4 * VIEW_HEIGHT * 4);
    }

    #[test]
    fn a_window_keeps_its_height_and_fits_the_view_across() {
        assert_eq!(fitted_width(WIDE_WIDTH, 672), 1200);
        assert_eq!(fitted_width(CLASSIC_WIDTH, 672), 768);
        assert_eq!(fitted_width(WIDE_WIDTH, 1080), 1928);
        assert_eq!(fitted_width(WIDE_WIDTH, u32::MAX), u32::MAX);
    }

    #[test]
    fn presenting_into_a_window_smaller_than_the_view_still_draws() {
        let canvas = filled(CLASSIC_WIDTH, 0x0012_3456);
        let size = (100, 80);
        let mut target = vec![0u32; size.0 * size.1];
        present(&canvas, &mut target, size);
        assert!(target.contains(&0x0012_3456));
    }

    #[test]
    fn filling_clips_to_the_view_instead_of_panicking() {
        let mut canvas = Canvas::new(CLASSIC_WIDTH);
        fill(&mut canvas, (-4, -4), (8, 8), 0x00AA_BBCC);
        assert_eq!(canvas.pixels[0], 0x00AA_BBCC);
        // Entirely outside, in every direction.
        for at in [
            (-100, 0),
            (signed(CLASSIC_WIDTH) + 4, 0),
            (0, signed(VIEW_HEIGHT) + 4),
        ] {
            fill(&mut canvas, at, (8, 8), 0x0011_2233);
        }
        assert!(!canvas.pixels.contains(&0x0011_2233));
    }

    #[test]
    fn a_bmp_that_is_not_twenty_four_bit_top_down_is_refused() {
        assert!(decode_bmp(&[]).is_none());
        assert!(decode_bmp(&[0u8; 64]).is_none());
    }

    #[test]
    fn a_synthetic_bmp_round_trips_its_pixels() {
        // Two by two, top-down, with four-byte row padding.
        let mut bytes = vec![0u8; 26];
        bytes[10..14].copy_from_slice(&26u32.to_le_bytes());
        bytes[18..22].copy_from_slice(&2i32.to_le_bytes());
        bytes[22..26].copy_from_slice(&(-2i32).to_le_bytes());
        let mut header = bytes.clone();
        header.resize(30, 0);
        header[28..30].copy_from_slice(&24u16.to_le_bytes());
        // Rewrite the offset now that the header is longer.
        header[10..14].copy_from_slice(&32u32.to_le_bytes());
        header.resize(32, 0);
        // Row 0: red, green. Row 1: blue, white. BGR order, padded to 8 bytes.
        header.extend_from_slice(&[0, 0, 255, 0, 255, 0, 0, 0]);
        header.extend_from_slice(&[255, 0, 0, 255, 255, 255, 0, 0]);
        let background = decode_bmp(&header).expect("a well-formed BMP");
        assert_eq!((background.width, background.height), (2, 2));
        assert_eq!(background.pixels[0], 0x00FF_0000);
        assert_eq!(background.pixels[1], 0x0000_FF00);
        assert_eq!(background.pixels[2], 0x0000_00FF);
        assert_eq!(background.pixels[3], 0x00FF_FFFF);
    }

    #[test]
    fn the_background_blit_stops_at_the_map_edge() {
        let background = Background {
            pixels: vec![0x0000_0001; 300 * 300],
            width: 300,
            height: 300,
            high: Vec::new(),
        };
        let mut canvas = Canvas::new(CLASSIC_WIDTH);
        // A camera near the edge leaves the far side untouched rather than
        // wrapping or reading out of bounds; so does one left of the origin.
        draw_background(&mut canvas, &background, (200, 200));
        assert_eq!(canvas.pixels[0], 0x0000_0001);
        assert_eq!(canvas.pixels[VIEW_HEIGHT * CLASSIC_WIDTH - 1], 0);
        let mut canvas = Canvas::new(WIDE_WIDTH);
        draw_background(&mut canvas, &background, (-72, 0));
        assert_eq!(canvas.pixels[71], 0);
        assert_eq!(canvas.pixels[72], 0x0000_0001);
    }

    fn raster(width: usize, height: usize, offset: (i16, i16)) -> crysta_runtime::art::Raster {
        crysta_runtime::art::Raster {
            width,
            height,
            offset,
            pixels: vec![0xFF12_3456; width * height],
        }
    }

    #[test]
    fn a_sprite_lands_at_its_origin_plus_offset_minus_the_camera() {
        let background = Background {
            pixels: vec![0; 512 * 512],
            width: 512,
            height: 512,
            high: Vec::new(),
        };
        let mut canvas = Canvas::new(CLASSIC_WIDTH);
        // Origin (100, 100), offset (-8, -16), camera (50, 40): the top-left
        // pixel lands at view (42, 44).
        draw_sprite(
            &mut canvas,
            &background,
            (50, 40),
            &raster(16, 16, (-8, -16)),
            (100, 100),
        );
        assert_eq!(canvas.pixels[44 * CLASSIC_WIDTH + 42], 0x0012_3456);
        assert_eq!(canvas.pixels[43 * CLASSIC_WIDTH + 42], 0);
        assert_eq!(canvas.pixels[44 * CLASSIC_WIDTH + 41], 0);
        assert_eq!(
            canvas.pixels[(44 + 15) * CLASSIC_WIDTH + 42 + 15],
            0x0012_3456
        );
        assert_eq!(canvas.pixels[(44 + 16) * CLASSIC_WIDTH + 42 + 16], 0);
    }

    #[test]
    fn transparent_pixels_and_high_background_are_left_alone() {
        let mut high = vec![false; 512 * 512];
        // The world pixel (100, 100) is under a high-priority tile.
        high[100 * 512 + 100] = true;
        let background = Background {
            pixels: vec![0; 512 * 512],
            width: 512,
            height: 512,
            high,
        };
        let mut canvas = filled(CLASSIC_WIDTH, 0x00AB_CDEF);
        let mut sprite = raster(2, 1, (0, 0));
        sprite.pixels[1] = 0; // transparent
        draw_sprite(&mut canvas, &background, (0, 0), &sprite, (100, 100));
        // Occluded: the background shows through.
        assert_eq!(canvas.pixels[100 * CLASSIC_WIDTH + 100], 0x00AB_CDEF);
        // Transparent: untouched too.
        assert_eq!(canvas.pixels[100 * CLASSIC_WIDTH + 101], 0x00AB_CDEF);
        // But the same sprite one pixel over draws.
        draw_sprite(&mut canvas, &background, (0, 0), &sprite, (101, 100));
        assert_eq!(canvas.pixels[100 * CLASSIC_WIDTH + 101], 0x0012_3456);
    }

    #[test]
    fn a_sprite_off_the_view_or_before_the_camera_is_clipped_not_wrapped() {
        let background = Background {
            pixels: vec![0; 64 * 64],
            width: 64,
            height: 64,
            high: Vec::new(),
        };
        let mut canvas = Canvas::new(CLASSIC_WIDTH);
        // Above and left of the world origin entirely.
        draw_sprite(
            &mut canvas,
            &background,
            (0, 0),
            &raster(8, 8, (-8, -8)),
            (0, 0),
        );
        assert!(canvas.pixels.iter().all(|pixel| *pixel == 0));
        // Behind the camera.
        draw_sprite(
            &mut canvas,
            &background,
            (32, 32),
            &raster(8, 8, (0, 0)),
            (10, 10),
        );
        assert!(canvas.pixels.iter().all(|pixel| *pixel == 0));
    }

    #[test]
    fn a_page_is_boxed_at_the_bottom_with_its_background_index_as_box_colour() {
        let mut canvas = Canvas::new(CLASSIC_WIDTH);
        // A 4x2 page: indices 0..3 across the top row, all background below.
        let indexed = [0u8, 1, 2, 3, 3, 3, 3, 3];
        let origin = page_origin(Placement::Bottom, (4, 2), 0, CLASSIC_WIDTH);
        draw_page(&mut canvas, &indexed, (4, 2), 3, origin);
        let box_width = 4 + 2 * PAGE_MARGIN;
        let box_height = 2 + 2 * PAGE_MARGIN;
        let left = (CLASSIC_WIDTH - box_width) / 2;
        let top = VIEW_HEIGHT - box_height - PAGE_MARGIN;
        // Border corner, box interior, then the page's pixels.
        assert_eq!(canvas.pixels[top * CLASSIC_WIDTH + left], PAGE_BORDER);
        assert_eq!(
            canvas.pixels[(top + 1) * CLASSIC_WIDTH + left + 1],
            PAGE_BOX
        );
        let row = (top + PAGE_MARGIN) * CLASSIC_WIDTH + left + PAGE_MARGIN;
        assert_eq!(canvas.pixels[row], PAGE_PALETTE[0]);
        assert_eq!(canvas.pixels[row + 1], PAGE_PALETTE[1]);
        assert_eq!(canvas.pixels[row + 2], PAGE_PALETTE[2]);
        assert_eq!(
            canvas.pixels[row + 3],
            PAGE_BOX,
            "the background index is the box"
        );
        // Above the box nothing was touched.
        assert_eq!(canvas.pixels[(top - 1) * CLASSIC_WIDTH + left], 0);
    }

    #[test]
    fn an_oversized_page_stays_inside_its_box() {
        let mut canvas = Canvas::new(WIDE_WIDTH);
        let (width, height) = (300, 300);
        draw_page(
            &mut canvas,
            &vec![1; width * height],
            (width, height),
            3,
            (72, 0),
        );
        let at = |x: usize, y: usize| canvas.pixels[y * WIDE_WIDTH + x];
        // The box is the classic 256 across and the view's 224 down.
        assert_eq!(at(72 + 8, 8), PAGE_PALETTE[1]);
        assert_eq!(at(72 + 247, 215), PAGE_PALETTE[1]);
        assert_eq!(at(72 + 248, 8), PAGE_BOX, "right margin");
        assert_eq!(at(72 + 8, 216), PAGE_BOX, "bottom margin");
        assert_eq!(at(72 + 256, 8), 0, "outside the classic area");
    }

    #[test]
    fn the_choice_cursor_points_right_inside_the_page() {
        let mut canvas = Canvas::new(CLASSIC_WIDTH);
        draw_cursor(&mut canvas, (10, 20), [16, 8]);
        let at = |x: usize, y: usize| canvas.pixels[y * CLASSIC_WIDTH + x];
        let (x, y) = (10 + PAGE_MARGIN + 16, 20 + PAGE_MARGIN + 8);
        // Widest on its middle row, a single pixel at the tips.
        assert_eq!(at(x + 3, y + 3), PAGE_PALETTE[1]);
        assert_eq!(at(x + 4, y + 3), 0);
        assert_eq!(at(x, y), PAGE_PALETTE[1]);
        assert_eq!(at(x + 1, y), 0);
    }

    #[test]
    fn a_malformed_page_draws_nothing() {
        let mut canvas = Canvas::new(CLASSIC_WIDTH);
        draw_page(&mut canvas, &[1, 1], (4, 2), 3, (8, 8));
        draw_page(&mut canvas, &[], (0, 0), 3, (8, 8));
        assert!(canvas.pixels.iter().all(|pixel| *pixel == 0));
    }
}
