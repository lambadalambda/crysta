//! Composing one 256x224 frame, without a window in sight.
//!
//! Everything here is a pure function over decoded pixels, so the renderer can
//! be tested without opening a window or owning a GPU.

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
    })
}

/// Top-left of the view that keeps `(x, y)` centred without leaving the map.
///
/// A map smaller than the view is pinned at zero rather than centred, so the
/// camera never reports a negative origin.
#[must_use]
pub fn camera(position: (u16, u16), map: (usize, usize)) -> (usize, usize) {
    let centre = |value: u16, view: usize, extent: usize| {
        let half = view / 2;
        let wanted = usize::from(value).saturating_sub(half);
        wanted.min(extent.saturating_sub(view))
    };
    (
        centre(position.0, VIEW_WIDTH, map.0),
        centre(position.1, VIEW_HEIGHT, map.1),
    )
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
    fn the_camera_centres_the_player_and_stops_at_the_edges() {
        let map = (1024, 1280);
        // Centred well inside the map.
        assert_eq!(camera((512, 640), map), (512 - 128, 640 - 112));
        // Clamped at the top-left rather than going negative.
        assert_eq!(camera((0, 0), map), (0, 0));
        assert_eq!(camera((10, 10), map), (0, 0));
        // Clamped at the bottom-right rather than running past the map.
        assert_eq!(camera((5000, 5000), map), (1024 - 256, 1280 - 224));
    }

    #[test]
    fn a_map_smaller_than_the_view_pins_the_camera_at_zero() {
        assert_eq!(camera((8, 8), (128, 64)), (0, 0));
        assert_eq!(camera((500, 500), (128, 64)), (0, 0));
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
        };
        let mut frame = vec![0u32; VIEW_WIDTH * VIEW_HEIGHT];
        // A camera near the edge leaves the far side untouched rather than
        // wrapping or reading out of bounds.
        draw_background(&mut frame, &background, (200, 200));
        assert_eq!(frame[0], 0x0000_0001);
        assert_eq!(frame[VIEW_HEIGHT * VIEW_WIDTH - 1], 0);
    }
}
