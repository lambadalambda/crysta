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

/// The ring window (`COP 64`, `$80:9CE6`; `$87:A26B`, `docs/tower-five.md`):
/// an ellipse half as tall as it is wide around its caller, as
/// `$87:A2C7..A44C` builds its tables each frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ring {
    /// The centre: the caller's x, and the row the top table ends on, 14
    /// above the caller (measured on `$11D`'s native frames).
    centre: (i32, i32),
    /// The top table (`$7E:6000`), from the top row to the centre row:
    /// (left, right) from the centre's x, both edges in.
    above: Rows,
    /// The bottom one (`$7E:6100`), from the centre row down.
    below: Rows,
}

impl Ring {
    /// The ring around a caller at `caller` whose tables were built from
    /// `built`, the radius as the effect ran (before this frame's script
    /// raised it); `None` while no tables are built.
    #[must_use]
    pub fn new(caller: (i32, i32), built: u16) -> Option<Self> {
        let (above, below) = ring_rows(built)?;
        Some(Self {
            centre: (caller.0, caller.1 - 14),
            above,
            below,
        })
    }

    /// The columns map row `y` shows inside the window, from the first to
    /// past the last; `None` outside it. The top table's rows end on the
    /// centre row; below it the bottom table shows from its third entry
    /// (its second falls on the centre row, under the top one's last).
    #[must_use]
    pub fn span(&self, y: i32) -> Option<(i32, i32)> {
        let (cx, cy) = self.centre;
        let half = i32::try_from(self.above.len()).ok()? - 1;
        let (left, right) = if (cy - half..=cy).contains(&y) {
            *self.above.get(usize::try_from(y - (cy - half)).ok()?)?
        } else if y > cy {
            *self.below.get(usize::try_from(y - cy + 1).ok()?)?
        } else {
            return None;
        };
        Some((cx + left, cx + right + 1))
    }
}

/// A window table: each row's (left, right) from the centre's x.
type Rows = Vec<(i32, i32)>;

/// The ring's tables for `radius` (`$0474`), by the doubled midpoint loop
/// at `$87:A3C2..A449`: the top one (`$7E:6000`) from its top row, its
/// last entry the centre row's, and the bottom one (`$7E:6100`) from the
/// centre row down, `(r & $FE) / 2 + 1` rows each; `None` below 4
/// (`$87:A2CE`). Screen edge clamps are left to the host.
pub(super) fn ring_rows(radius: u16) -> Option<(Rows, Rows)> {
    if radius < 4 {
        return None;
    }
    let width = radius & 0xFE;
    let half = usize::from(width / 2);
    // The tables as the routine indexes them: top by X from the centre up,
    // bottom by Y from the centre down, each pass writing both.
    let (mut top, mut bottom) = (vec![(0, 0); half + 1], vec![(0, 0); half + 1]);
    let (mut error, mut c, mut k) = (width, i32::from(width), 0i32);
    let (mut near, mut far) = (0usize, 0usize);
    loop {
        // Pass A: the rows near the centre row take the half-width `c`.
        loop {
            let row = (-c, c - 1);
            if let Some(slot) = half.checked_sub(near).and_then(|at| top.get_mut(at)) {
                *slot = row;
            }
            if let Some(slot) = bottom.get_mut(near) {
                *slot = row;
            }
            near += 1;
            let step = u16::try_from(2 * k - 1).unwrap_or(0xFFFF);
            k += 2;
            let borrow = error < step;
            error = error.wrapping_sub(step);
            if borrow && step != 0xFFFF {
                break;
            }
        }
        // Pass B: the rows far from it take `k`.
        let row = (1 - k, k - 2);
        if let Some(slot) = top.get_mut(far) {
            *slot = row;
        }
        if let Some(slot) = half.checked_sub(far).and_then(|at| bottom.get_mut(at)) {
            *slot = row;
        }
        far += 1;
        error = error.wrapping_add(u16::try_from(2 * c - 1).unwrap_or(0));
        c -= 2;
        if c < k {
            break;
        }
    }
    Some((top, bottom))
}

impl World<'_> {
    /// The ring window, while its caller is here and its tables are built.
    #[must_use]
    pub fn ring(&self) -> Option<Ring> {
        let (caller, built) = self.globals.ring?;
        let actor = self
            .actors
            .iter()
            .find(|actor| actor.id == caller && !actor.is_gone())?;
        let (x, y) = actor.position;
        Ring::new((i32::from(x), i32::from(y)), built)
    }

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
    fn the_ring_rows_follow_the_native_tables() {
        // `$11D`, the caller at x 128: the left and right edges of the top
        // table as `$7E:6000`/`6200` held them (2026-10-17), and the bottom
        // one from `$6101`/`6301`.
        let lefts = |rows: &[(i32, i32)]| rows.iter().map(|row| 128 + row.0).collect::<Vec<_>>();
        let rights = |rows: &[(i32, i32)]| rows.iter().map(|row| 128 + row.1).collect::<Vec<_>>();
        let (top, bottom) = ring_rows(42).unwrap();
        assert_eq!(
            lefts(&top),
            [
                117, 111, 107, 103, 99, 97, 96, 94, 94, 92, 92, 90, 90, 88, 88, 88, 86, 86, 86, 86,
                86, 86
            ]
        );
        assert_eq!(
            rights(&top),
            [
                138, 144, 148, 152, 156, 158, 159, 161, 161, 163, 163, 165, 165, 167, 167, 167,
                169, 169, 169, 169, 169, 169
            ]
        );
        assert_eq!(
            lefts(&bottom[1..]),
            [
                86, 86, 86, 86, 86, 88, 88, 88, 90, 90, 92, 92, 94, 94, 96, 97, 99, 103, 107, 111,
                117
            ]
        );
        let (top, _) = ring_rows(18).unwrap();
        assert_eq!(
            lefts(&top),
            [121, 117, 113, 114, 112, 112, 110, 110, 110, 110]
        );
        assert_eq!(
            rights(&top),
            [134, 138, 142, 141, 143, 143, 145, 145, 145, 145]
        );
        let (top, bottom) = ring_rows(5).unwrap();
        assert_eq!(
            (lefts(&top), rights(&top)),
            (vec![123, 124, 124], vec![132, 131, 131])
        );
        assert_eq!(lefts(&bottom[1..]), [124, 123]);
        assert_eq!(ring_rows(3), None);
    }

    #[test]
    fn the_ring_shows_its_tables_around_the_centre_row() {
        // Built from 43, the caller at (128,128): native `$11D`'s frame at
        // `$0474` = 44 shows rows 93..134 (`tests/local_towers.rs`).
        let ring = Ring::new((128, 128), 43).unwrap();
        assert_eq!(ring.span(92), None);
        assert_eq!(ring.span(93), Some((117, 139)));
        assert_eq!(ring.span(114), Some((86, 170)), "the centre row");
        assert_eq!(ring.span(115), Some((86, 170)));
        assert_eq!(ring.span(134), Some((117, 139)));
        assert_eq!(ring.span(135), None);
        assert_eq!(Ring::new((128, 128), 3), None);
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
