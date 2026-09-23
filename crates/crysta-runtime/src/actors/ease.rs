//! `COP ED` / `COP EE`: an eased move to a point relative to the actor.
//!
//! `COP ED` (`$80:9F4C`, `$87:CDBD`) keeps half the distance on each axis and
//! the midpoint. Each `COP EE` (`$80:9F93`) adds its speed to a phase; below
//! 128 the actor stands at the midpoint plus half the distance scaled by the
//! cosine table `$81:F462` (`$87:CE36`), so it starts and stops slowly, and
//! at 128 or more it stands at the midpoint plus the half distance and the
//! script goes on. An odd distance ends a pixel short, as natively.

/// The cosine table `$81:F462`: 256 signed bytes, 127 at phase 0.
const COSINE: usize = 0x01_F462;

/// An eased move under way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Ease {
    /// Half the distance on each axis (`$7F:2000`, `$7F:2002`).
    half: (u16, u16),
    /// The midpoint (`$7F:2004`, `$7F:2006`).
    middle: (u16, u16),
    /// Whether the move goes right, and down (`$7F:200C` bits 1 and 0).
    toward: (bool, bool),
    /// The phase (`$7F:2008`).
    phase: u16,
}

impl Ease {
    /// `COP ED` from `from` by `(dx, dy)`.
    pub(super) fn new(from: (u16, u16), (dx, dy): (i16, i16)) -> Self {
        // `$87:CDBD` tests the sign of `from - to`, which is `-d`.
        let half = (dx.unsigned_abs() / 2, dy.unsigned_abs() / 2);
        let toward = (dx > 0, dy > 0);
        Self {
            half,
            middle: (
                side(from.0, half.0, toward.0),
                side(from.1, half.1, toward.1),
            ),
            toward,
            phase: 0,
        }
    }

    /// One `COP EE` of `speed`: the position, and whether the move has ended.
    pub(super) fn step(&mut self, image: &[u8], speed: u8) -> Option<((u16, u16), bool)> {
        // A 16-bit sum; the arrival tests its bit 7, the table its low byte.
        self.phase = self.phase.wrapping_add(u16::from(speed));
        let arrived = self.phase & 0x80 != 0;
        // The native sums wrap at 16 bits, as these do.
        let offset = |half: u16, toward: bool, cosine: i8| -> u16 {
            // The multiplier takes the half distance a byte at a time; the low
            // byte's product keeps its high byte doubled, the high byte's
            // product is doubled whole.
            let magnitude = u32::from(cosine.unsigned_abs());
            let low = ((magnitude * u32::from(half & 0xFF)) << 1) >> 8;
            let high = (magnitude * u32::from(half >> 8)) << 1;
            let value = u16::try_from((low + high) & 0xFFFF).unwrap_or(0);
            let value = if cosine < 0 {
                value.wrapping_neg()
            } else {
                value
            };
            if toward {
                value.wrapping_neg()
            } else {
                value
            }
        };
        let position = if arrived {
            // `$80:9FE6` / `A000` take the half's low byte.
            (
                side(self.middle.0, self.half.0 & 0xFF, self.toward.0),
                side(self.middle.1, self.half.1 & 0xFF, self.toward.1),
            )
        } else {
            let cosine = i8::from_ne_bytes([*image.get(COSINE + usize::from(self.phase & 0xFF))?]);
            (
                self.middle
                    .0
                    .wrapping_add(offset(self.half.0, self.toward.0, cosine)),
                self.middle
                    .1
                    .wrapping_add(offset(self.half.1, self.toward.1, cosine)),
            )
        };
        Some((position, arrived))
    }
}

/// `at` moved by `by` toward the target's side.
const fn side(at: u16, by: u16, toward: bool) -> u16 {
    if toward {
        at.wrapping_add(by)
    } else {
        at.wrapping_sub(by)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An image with a synthetic cosine table.
    #[allow(clippy::cast_possible_truncation)] // 127 * cos stays within i8.
    fn cosine_image() -> Vec<u8> {
        let mut image = vec![0; COSINE + 256];
        for phase in 0..256u16 {
            let angle = f64::from(phase) * std::f64::consts::TAU / 256.0;
            let value = (127.0 * angle.cos()).round() as i8;
            image[COSINE + usize::from(phase)] = value.to_ne_bytes()[0];
        }
        image
    }

    #[test]
    fn a_move_starts_where_the_actor_stands_and_ends_at_the_target() {
        let image = cosine_image();
        let mut ease = Ease::new((136, 208), (0, 24));
        let mut path = Vec::new();
        loop {
            let (position, arrived) = ease.step(&image, 3).unwrap();
            path.push(position);
            if arrived {
                break;
            }
        }
        assert_eq!(path.len(), 43, "128 / 3, rounded up");
        assert_eq!(path.last(), Some(&(136, 232)));
        assert!(
            path.windows(2).all(|pair| pair[0].1 <= pair[1].1),
            "monotonic"
        );
        assert!(path[0].1 - 208 <= 1, "starts slowly");
    }

    #[test]
    fn an_odd_distance_ends_a_pixel_short_and_left_or_up_works() {
        let image = cosine_image();
        let mut ease = Ease::new((100, 100), (-7, -3));
        let end = std::iter::repeat_with(|| ease.step(&image, 64).unwrap())
            .find(|&(_, arrived)| arrived)
            .unwrap()
            .0;
        assert_eq!(end, (94, 98), "half 3 and 1, not 3.5 and 1.5");
    }
}
