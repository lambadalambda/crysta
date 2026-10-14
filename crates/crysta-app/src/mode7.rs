//! The world map in Mode 7 ([notes](../../../docs/world-map-mode7.md)):
//! each line samples one row of the plane, scaled about Ark's column by its
//! own factor, as ares runs `Background::runMode7` with M7B = M7C = 0. The
//! sky lines show the plane mirrored and 2x2 mosaic, subtracted from the
//! backdrop gradient; below them a fog colour is subtracted, thinning
//! downward. A wide view widens the lines; the classic one keeps the
//! colour window's black borders.

use crate::frame::{rgb, signed, Canvas, CLASSIC_WIDTH, VIEW_HEIGHT};
use assets::graphics::Bgr555;
use assets::maps::visual::mode7::Mode7View;
use assets::maps::visual::world::WorldMap;
use crysta_runtime::art::Raster;

/// Ark's point on the screen: M7HOFS = x − 128, M7VOFS = y − 160.
const CENTRE: (i32, i32) = (128, 160);
/// The plane wraps at 1024 pixels.
const PLANE: i32 = 1024;
/// The colour window's borders (W2 = 8..247, inverted).
const BORDERS: (i32, i32) = (8, 247);

/// A world map's plane and view.
pub struct Mode7 {
    /// The plane.
    pub map: WorldMap,
    /// The per-line tables.
    pub view: Mode7View,
    /// `$03`'s horizon band ([`crysta_runtime::art::horizon_band`]), fixed
    /// on the classic screen at (128,53).
    pub band: Option<Raster>,
}

/// The band's anchor on the classic screen: OAM y + 1 is the line, so a
/// row up.
const BAND: (i32, i32) = (128, 52);
/// Lines the band's colours replace the fog's on.
const BAND_LINES: std::ops::Range<usize> = 52..68;

/// The band's colour at a classic screen pixel, where it is opaque.
fn band_colour(band: &Raster, (x, y): (i32, i32)) -> Option<Bgr555> {
    let (bx, by) = (
        usize::try_from(x - BAND.0 - i32::from(band.offset.0)).ok()?,
        usize::try_from(y - BAND.1 - i32::from(band.offset.1)).ok()?,
    );
    if bx >= band.width {
        return None;
    }
    let pixel = *band.pixels.get(by * band.width + bx)?;
    // Back to 5 bits a channel: the raster's 8 keep them in the top bits.
    let channel = |shift: u32| u16::try_from((pixel >> (shift + 3)) & 31).unwrap_or(0);
    (pixel >> 24 != 0).then(|| Bgr555::new(channel(16) | channel(8) << 5 | channel(0) << 10))
}

/// The 13-bit signed difference ares keeps of the origin's offset.
const fn clip(n: i32) -> i32 {
    if n & 0x2000 != 0 {
        n | !1023
    } else {
        n & 1023
    }
}

/// Subtracts per 5-bit channel, clamped at 0 (`CGADSUB` subtract, no
/// halving).
pub(crate) fn subtract(a: Bgr555, b: Bgr555) -> Bgr555 {
    let channel = |shift: u16| {
        let (x, y) = (a.raw() >> shift & 31, b.raw() >> shift & 31);
        x.saturating_sub(y) << shift
    };
    Bgr555::new(channel(0) | channel(5) | channel(10))
}

/// A fixed colour of `intensity` in all three channels (`COLDATA`).
pub(crate) fn fixed(intensity: u8) -> Bgr555 {
    let i = u16::from(intensity & 31);
    Bgr555::new(i | i << 5 | i << 10)
}

/// Draws the view with the player at `(x, y)`.
pub fn draw(canvas: &mut Canvas, mode7: &Mode7, player: (u16, u16)) {
    // M7HOFS/M7VOFS (the camera) and M7X/M7Y (the origin).
    // The plane wraps; so does the player's place on it.
    let (hofs, vofs) = (
        (i32::from(player.0) & (PLANE - 1)) - CENTRE.0,
        (i32::from(player.1) & (PLANE - 1)) - CENTRE.1,
    );
    let (origin_x, origin_y) = (
        (hofs & (PLANE - 1)) + CENTRE.0,
        (vofs & (PLANE - 1)) + CENTRE.1,
    );
    let left = signed(canvas.width) / 2 - CENTRE.0;
    let classic = canvas.width == CLASSIC_WIDTH;
    let view = &mode7.view;
    for row in 0..VIEW_HEIGHT {
        let scale = i32::from(view.scale[row]);
        let line = signed(row) + 1;
        let sky = row < view.sky;
        // Sky lines: vertically flipped, with a 2x2 mosaic.
        let sample_y = if sky {
            255 - (line - ((line - 1) & 1))
        } else {
            line
        };
        let start_x = ((scale * clip(hofs - origin_x)) & !63) + (origin_x << 8);
        let start_y =
            ((scale * clip(vofs - origin_y)) & !63) + ((scale * sample_y) & !63) + (origin_y << 8);
        let plane_y = (start_y >> 8) & (PLANE - 1);
        for column in 0..canvas.width {
            let screen_x = signed(column) - left;
            if classic && !(BORDERS.0..=BORDERS.1).contains(&screen_x) {
                canvas.pixels[row * canvas.width + column] = 0;
                continue;
            }
            let xs = if sky { screen_x & !1 } else { screen_x };
            let plane_x = ((start_x + scale * xs) >> 8) & (PLANE - 1);
            let index = mode7.map.pixel(
                usize::try_from(plane_x).unwrap_or(0),
                usize::try_from(plane_y).unwrap_or(0),
            );
            let colour = match (sky, index) {
                (true, 0) => subtract(view.backdrop[row], fixed(view.fade[row])),
                (true, _) => subtract(view.backdrop[row], mode7.map.color(index)),
                (false, 0) => view.backdrop[row],
                (false, _) => {
                    // The horizon band's grey, where it is, instead of the fog.
                    let band = mode7
                        .band
                        .as_ref()
                        .filter(|_| BAND_LINES.contains(&row))
                        .and_then(|band| band_colour(band, (screen_x, signed(row))));
                    subtract(
                        mode7.map.color(index),
                        band.unwrap_or(fixed(view.fade[row])),
                    )
                }
            };
            canvas.pixels[row * canvas.width + column] = rgb(colour);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subtraction_clamps_each_channel() {
        let a = Bgr555::new(0x7227);
        assert_eq!(subtract(a, fixed(31)).raw(), 0);
        assert_eq!(subtract(Bgr555::new(0x7FFF), fixed(1)).raw(), 0x7BDE);
        assert_eq!((clip(-128), clip(0x1F80)), (-128, 0x380));
    }
}
