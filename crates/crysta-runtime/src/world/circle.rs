//! The circle window (`COP 63`, `$80:9CA5`; `docs/tower-five.md`): an
//! actor that draws window 1 per line around its caller, radius `$0474 &
//! $7E`. On `$123` the window takes BG1 off the main screen (`W12SEL $02`,
//! `TMW $15`): inside it only the backdrop and the sprites show, so
//! Shadowkeeper's head appears in the dark.

use super::World;
use crate::actors::CIRCLE_RADIUS;

/// The window as the hosts draw it, in map coordinates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Circle {
    /// The centre: the caller's place, 16 above it (`$87:A4D0`).
    centre: (i32, i32),
    /// The half-width for each distance from the centre row, 0 to r.
    widths: Vec<u16>,
}

impl Circle {
    /// The columns map row `y` shows inside the window, from the first to
    /// past the last; `None` outside it. The `r` rows above the centre row
    /// are `d = cy - y` from it, the centre row and the `r - 1` below it
    /// `d = y - cy + 1` (the HDMA tables `$7E:6000` and `$7E:6101`).
    #[must_use]
    pub fn span(&self, y: i32) -> Option<(i32, i32)> {
        let (cx, cy) = self.centre;
        let radius = i32::try_from(self.widths.len()).ok()? - 1;
        let distance = if (cy - radius..cy).contains(&y) {
            cy - y
        } else if (cy..cy + radius).contains(&y) {
            y - cy + 1
        } else {
            return None;
        };
        let width = i32::from(*self.widths.get(usize::try_from(distance).ok()?)?);
        Some((cx - width, cx + width))
    }
}

/// The half-widths for distances 0 to `radius`, by the midpoint loop at
/// `$87:A5B3..A647` with its 16-bit error.
pub(super) fn half_widths(radius: u16) -> Vec<u16> {
    let mut widths = vec![0; usize::from(radius) + 1];
    let (mut error, mut c, mut k) = (radius, radius, 0u16);
    let (mut near, mut far) = (0usize, usize::from(radius));
    loop {
        // Pass A: rows near the centre take the half-width `c`.
        loop {
            if let Some(width) = widths.get_mut(near) {
                *width = c;
            }
            near += 1;
            let step = (2 * k).wrapping_sub(1);
            k += 1;
            let carry = error >= step;
            error = error.wrapping_sub(step);
            if !(carry || step == 0xFFFF) {
                break;
            }
        }
        // Pass B: the row `far` from the centre takes `k - 1`.
        if let Some(width) = widths.get_mut(far) {
            *width = k - 1;
        }
        far = far.saturating_sub(1);
        error = error.wrapping_add(2 * c.saturating_sub(1));
        c = c.saturating_sub(1);
        if c < k {
            return widths;
        }
    }
}

impl World<'_> {
    /// The circle window, while its radius is not zero and its caller is
    /// here.
    #[must_use]
    pub fn circle(&self) -> Option<Circle> {
        let radius = self.script_word(CIRCLE_RADIUS)? & 0x7E;
        let caller = self.globals.circle?;
        let actor = self
            .actors
            .iter()
            .find(|actor| actor.id == caller && !actor.is_gone())?;
        (radius != 0).then(|| Circle {
            centre: (
                i32::from(actor.position.0),
                i32::from(actor.position.1) - 16,
            ),
            widths: half_widths(radius),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_half_widths_follow_the_native_tables() {
        // r = 48, read from the native `$7E:6000..63FF` at frame 61416.
        let mut expected = vec![48; 9];
        expected.extend([47; 4]);
        expected.extend([46; 4]);
        expected.extend([45; 3]);
        expected.extend([
            44, 44, 43, 43, 42, 42, 41, 40, 40, 39, 38, 37, 37, 36, 35, 34, 33, 32, 30, 29, 28, 26,
            25, 23, 21, 19, 16, 12, 8,
        ]);
        assert_eq!(half_widths(48), expected);
    }

    #[test]
    fn a_row_takes_its_distance_from_the_centre_row() {
        let circle = Circle {
            centre: (128, 96),
            widths: vec![4, 3, 1],
        };
        assert_eq!(circle.span(93), None);
        assert_eq!(circle.span(94), Some((127, 129)));
        assert_eq!(circle.span(95), Some((125, 131)));
        assert_eq!(circle.span(96), Some((125, 131)));
        assert_eq!(circle.span(97), Some((127, 129)));
        assert_eq!(circle.span(98), None);
    }
}
