//! Composing one 256x224 frame, without a window in sight.
//!
//! Everything here is a pure function over decoded pixels, so the renderer can
//! be tested without opening a window or owning a GPU.

use assets::text::Placement;

/// Native view width in pixels.
pub const VIEW_WIDTH: usize = 256;
/// Native view height in pixels.
pub const VIEW_HEIGHT: usize = 224;

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
}

/// Blits a sprite raster at a world position, occluded by the background.
///
/// This is the first-background compositor the browser viewer qualified,
/// narrowed to what the slice draws: every sprite is ordinary priority, so a
/// pixel under an opaque high-priority tile is the tile's. Later sprites in
/// the draw list overwrite earlier ones where both are opaque.
pub fn draw_sprite(
    frame: &mut [u32],
    background: &Background,
    camera: (usize, usize),
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
            let (Ok(column_i), Ok(row_i)) = (i64::try_from(column), i64::try_from(row)) else {
                continue;
            };
            let world_x = i64::from(at.0) + i64::from(raster.offset.0) + column_i;
            let world_y = i64::from(at.1) + i64::from(raster.offset.1) + row_i;
            let (Ok(world_x), Ok(world_y)) = (usize::try_from(world_x), usize::try_from(world_y))
            else {
                continue;
            };
            if background.occludes(world_x, world_y) {
                continue;
            }
            let (Some(x), Some(y)) = (world_x.checked_sub(camera.0), world_y.checked_sub(camera.1))
            else {
                continue;
            };
            if x < VIEW_WIDTH && y < VIEW_HEIGHT {
                frame[y * VIEW_WIDTH + x] = pixel & 0x00FF_FFFF;
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
#[must_use]
pub fn page_origin(
    placement: Placement,
    (width, height): (usize, usize),
    player_screen_y: usize,
) -> (usize, usize) {
    let box_width = (width + 2 * PAGE_MARGIN).min(VIEW_WIDTH);
    let box_height = (height + 2 * PAGE_MARGIN).min(VIEW_HEIGHT);
    let centred = (VIEW_WIDTH - box_width) / 2;
    let bottom = VIEW_HEIGHT.saturating_sub(box_height + PAGE_MARGIN);
    let top = match placement {
        Placement::AwayFromPlayer if player_screen_y >= VIEW_HEIGHT / 2 => PAGE_MARGIN,
        Placement::Bottom | Placement::AwayFromPlayer => bottom,
        Placement::Tile { column, row } => {
            return (
                (usize::from(column) * 8).saturating_sub(PAGE_MARGIN),
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
    frame: &mut [u32],
    indexed: &[u8],
    (width, height): (usize, usize),
    background_index: u8,
    (left, top): (usize, usize),
) {
    if width == 0 || height == 0 || indexed.len() < width * height {
        return;
    }
    let box_width = (width + 2 * PAGE_MARGIN).min(VIEW_WIDTH);
    let box_height = (height + 2 * PAGE_MARGIN).min(VIEW_HEIGHT);
    let (left_i, top_i) = (
        i32::try_from(left).unwrap_or(0),
        i32::try_from(top).unwrap_or(0),
    );
    let (box_w, box_h) = (
        i32::try_from(box_width).unwrap_or(0),
        i32::try_from(box_height).unwrap_or(0),
    );
    fill(frame, (left_i, top_i), (box_w, box_h), PAGE_BORDER);
    fill(
        frame,
        (left_i + 1, top_i + 1),
        (box_w - 2, box_h - 2),
        PAGE_BOX,
    );
    let mut palette = PAGE_PALETTE;
    if let Some(slot) = palette.get_mut(usize::from(background_index)) {
        *slot = PAGE_BOX;
    }
    for row in 0..height {
        let y = top + PAGE_MARGIN + row;
        if y >= VIEW_HEIGHT {
            break;
        }
        for column in 0..width {
            let x = left + PAGE_MARGIN + column;
            if x >= VIEW_WIDTH {
                break;
            }
            let index = usize::from(indexed[row * width + column]) & 3;
            frame[y * VIEW_WIDTH + x] = palette[index];
        }
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

/// Blits the visible window of `background` into a 256x224 frame.
pub fn draw_background(frame: &mut [u32], background: &Background, camera: (usize, usize)) {
    for row in 0..VIEW_HEIGHT {
        let source = camera.1 + row;
        if source >= background.height {
            break;
        }
        for column in 0..VIEW_WIDTH {
            let across = camera.0 + column;
            if across >= background.width {
                break;
            }
            frame[row * VIEW_WIDTH + column] =
                background.pixels[source * background.width + across];
        }
    }
}

/// Fills a rectangle, clipped to the frame.
pub fn fill(frame: &mut [u32], at: (i32, i32), size: (i32, i32), colour: u32) {
    for row in 0..size.1 {
        let Ok(y) = usize::try_from(at.1 + row) else {
            continue;
        };
        if y >= VIEW_HEIGHT {
            continue;
        }
        for column in 0..size.0 {
            let Ok(x) = usize::try_from(at.0 + column) else {
                continue;
            };
            if x >= VIEW_WIDTH {
                continue;
            }
            frame[y * VIEW_WIDTH + x] = colour;
        }
    }
}

/// Scales `frame` into `target` by the largest integer factor that fits.
///
/// Nearest neighbour and integer only: a non-integer scale would resample
/// pixels the renderer went to some trouble to reproduce exactly.
pub fn present(frame: &[u32], target: &mut [u32], size: (usize, usize)) {
    let scale = (size.0 / VIEW_WIDTH).min(size.1 / VIEW_HEIGHT).max(1);
    let (drawn_width, drawn_height) = (VIEW_WIDTH * scale, VIEW_HEIGHT * scale);
    let (left, top) = (
        size.0.saturating_sub(drawn_width) / 2,
        size.1.saturating_sub(drawn_height) / 2,
    );
    target.fill(0);
    for row in 0..drawn_height.min(size.1) {
        let source = row / scale;
        for column in 0..drawn_width.min(size.0) {
            let pixel = frame[source * VIEW_WIDTH + column / scale];
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

    #[test]
    fn a_page_box_goes_where_the_native_window_opens() {
        let dims = (224, 48);
        // The standard window sits along the bottom, centred.
        assert_eq!(page_origin(Placement::Bottom, dims, 30), (8, 224 - 64 - 8));
        assert_eq!(page_origin(Placement::Bottom, dims, 200), (8, 224 - 64 - 8));
        // $DA keeps out of the player's half of the screen.
        assert_eq!(
            page_origin(Placement::AwayFromPlayer, dims, 111),
            (8, 224 - 64 - 8)
        );
        assert_eq!(page_origin(Placement::AwayFromPlayer, dims, 112), (8, 8));
        // $C2 puts the content at its tile column and row.
        let tile = Placement::Tile { column: 3, row: 3 };
        assert_eq!(page_origin(tile, (200, 48), 0), (24 - 8, 24 - 8));
        assert_eq!(
            page_origin(Placement::Tile { column: 0, row: 0 }, dims, 0),
            (0, 0)
        );
    }

    #[test]
    fn presenting_uses_an_integer_scale_and_centres_the_image() {
        let frame = vec![0x00FF_00FFu32; VIEW_WIDTH * VIEW_HEIGHT];
        // A 3x window leaves a border, and the image sits inside it.
        let size = (VIEW_WIDTH * 3 + 40, VIEW_HEIGHT * 3 + 20);
        let mut target = vec![0u32; size.0 * size.1];
        present(&frame, &mut target, size);
        let drawn = target.iter().filter(|pixel| **pixel != 0).count();
        assert_eq!(
            drawn,
            VIEW_WIDTH * 3 * VIEW_HEIGHT * 3,
            "exactly 3x, no more"
        );
        // The corners are border, the middle is image.
        assert_eq!(target[0], 0);
        assert_eq!(target[size.1 / 2 * size.0 + size.0 / 2], 0x00FF_00FF);
    }

    #[test]
    fn presenting_into_a_window_smaller_than_the_view_still_draws() {
        let frame = vec![0x0012_3456u32; VIEW_WIDTH * VIEW_HEIGHT];
        let size = (100, 80);
        let mut target = vec![0u32; size.0 * size.1];
        present(&frame, &mut target, size);
        assert!(target.contains(&0x0012_3456));
    }

    #[test]
    fn filling_clips_to_the_frame_instead_of_panicking() {
        let mut frame = vec![0u32; VIEW_WIDTH * VIEW_HEIGHT];
        fill(&mut frame, (-4, -4), (8, 8), 0x00AA_BBCC);
        assert_eq!(frame[0], 0x00AA_BBCC);
        // Entirely outside, in every direction.
        fill(&mut frame, (-100, 0), (8, 8), 0x0011_2233);
        fill(
            &mut frame,
            (i32::try_from(VIEW_WIDTH).unwrap() + 4, 0),
            (8, 8),
            0x0011_2233,
        );
        fill(
            &mut frame,
            (0, i32::try_from(VIEW_HEIGHT).unwrap() + 4),
            (8, 8),
            0x0011_2233,
        );
        assert!(!frame.contains(&0x0011_2233));
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
        let mut frame = vec![0u32; VIEW_WIDTH * VIEW_HEIGHT];
        // A camera near the edge leaves the far side untouched rather than
        // wrapping or reading out of bounds.
        draw_background(&mut frame, &background, (200, 200));
        assert_eq!(frame[0], 0x0000_0001);
        assert_eq!(frame[VIEW_HEIGHT * VIEW_WIDTH - 1], 0);
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
        let mut frame = vec![0u32; VIEW_WIDTH * VIEW_HEIGHT];
        // Origin (100, 100), offset (-8, -16), camera (50, 40): the top-left
        // pixel lands at view (42, 44).
        draw_sprite(
            &mut frame,
            &background,
            (50, 40),
            &raster(16, 16, (-8, -16)),
            (100, 100),
        );
        assert_eq!(frame[44 * VIEW_WIDTH + 42], 0x0012_3456);
        assert_eq!(frame[43 * VIEW_WIDTH + 42], 0);
        assert_eq!(frame[44 * VIEW_WIDTH + 41], 0);
        assert_eq!(frame[(44 + 15) * VIEW_WIDTH + 42 + 15], 0x0012_3456);
        assert_eq!(frame[(44 + 16) * VIEW_WIDTH + 42 + 16], 0);
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
        let mut frame = vec![0x00AB_CDEF; VIEW_WIDTH * VIEW_HEIGHT];
        let mut sprite = raster(2, 1, (0, 0));
        sprite.pixels[1] = 0; // transparent
        draw_sprite(&mut frame, &background, (0, 0), &sprite, (100, 100));
        // Occluded: the background shows through.
        assert_eq!(frame[100 * VIEW_WIDTH + 100], 0x00AB_CDEF);
        // Transparent: untouched too.
        assert_eq!(frame[100 * VIEW_WIDTH + 101], 0x00AB_CDEF);
        // But the same sprite one pixel over draws.
        draw_sprite(&mut frame, &background, (0, 0), &sprite, (101, 100));
        assert_eq!(frame[100 * VIEW_WIDTH + 101], 0x0012_3456);
    }

    #[test]
    fn a_sprite_off_the_view_or_before_the_camera_is_clipped_not_wrapped() {
        let background = Background {
            pixels: vec![0; 64 * 64],
            width: 64,
            height: 64,
            high: Vec::new(),
        };
        let mut frame = vec![0u32; VIEW_WIDTH * VIEW_HEIGHT];
        // Above and left of the world origin entirely.
        draw_sprite(
            &mut frame,
            &background,
            (0, 0),
            &raster(8, 8, (-8, -8)),
            (0, 0),
        );
        assert!(frame.iter().all(|pixel| *pixel == 0));
        // Behind the camera.
        draw_sprite(
            &mut frame,
            &background,
            (32, 32),
            &raster(8, 8, (0, 0)),
            (10, 10),
        );
        assert!(frame.iter().all(|pixel| *pixel == 0));
    }

    #[test]
    fn a_page_is_boxed_at_the_bottom_with_its_background_index_as_box_colour() {
        let mut frame = vec![0u32; VIEW_WIDTH * VIEW_HEIGHT];
        // A 4x2 page: indices 0..3 across the top row, all background below.
        let indexed = [0u8, 1, 2, 3, 3, 3, 3, 3];
        let origin = page_origin(Placement::Bottom, (4, 2), 0);
        draw_page(&mut frame, &indexed, (4, 2), 3, origin);
        let box_width = 4 + 2 * PAGE_MARGIN;
        let box_height = 2 + 2 * PAGE_MARGIN;
        let left = (VIEW_WIDTH - box_width) / 2;
        let top = VIEW_HEIGHT - box_height - PAGE_MARGIN;
        // Border corner, box interior, then the page's pixels.
        assert_eq!(frame[top * VIEW_WIDTH + left], PAGE_BORDER);
        assert_eq!(frame[(top + 1) * VIEW_WIDTH + left + 1], PAGE_BOX);
        let row = (top + PAGE_MARGIN) * VIEW_WIDTH + left + PAGE_MARGIN;
        assert_eq!(frame[row], PAGE_PALETTE[0]);
        assert_eq!(frame[row + 1], PAGE_PALETTE[1]);
        assert_eq!(frame[row + 2], PAGE_PALETTE[2]);
        assert_eq!(frame[row + 3], PAGE_BOX, "the background index is the box");
        // Above the box nothing was touched.
        assert_eq!(frame[(top - 1) * VIEW_WIDTH + left], 0);
    }

    #[test]
    fn a_malformed_page_draws_nothing() {
        let mut frame = vec![0u32; VIEW_WIDTH * VIEW_HEIGHT];
        draw_page(&mut frame, &[1, 1], (4, 2), 3, (8, 8));
        draw_page(&mut frame, &[], (0, 0), 3, (8, 8));
        assert!(frame.iter().all(|pixel| *pixel == 0));
    }
}
