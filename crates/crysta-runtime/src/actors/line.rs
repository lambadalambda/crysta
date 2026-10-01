//! The line move (`COP CC` `$80:AE38`, `COP CD` `$80:AF22`,
//! `docs/enemy-scripts.md` §6): a straight walk toward a target in equal
//! steps, the vector run one or more times over.

/// A line move under way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Line {
    /// The vector's size per axis, halved below 256, and its signs.
    size: (u16, u16),
    negative: (bool, bool),
    /// Steps a leg takes (`7F:2002` bits 0-13).
    steps: u16,
    /// The step under way and the distance it reached (`7F:2000`, `2008`).
    step: u16,
    reached: (u16, u16),
    /// Legs left (`7F:2014`).
    legs: u16,
    /// Frames left before the move ends early (`7F:200B`); 0 for none.
    limit: u8,
}

/// What a frame of the move does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Frame {
    /// Moves by the delta and goes on next frame.
    Moving(i16, i16),
    /// Moves by the delta and ends: the script goes on next frame.
    Ended(i16, i16),
}

impl Line {
    /// `COP CC a p c d s` from `at` toward `target`: legs `a + 1`, speed
    /// `c` (the long axis covers `c` pixels a step), frame limit `d`.
    pub(super) fn start(
        at: (u16, u16),
        target: (u16, u16),
        legs: u8,
        speed: u8,
        limit: u8,
    ) -> Self {
        let (mut ax, mut ay) = (target.0.wrapping_sub(at.0), target.1.wrapping_sub(at.1));
        let negative = (ax & 0x8000 != 0, ay & 0x8000 != 0);
        if negative.0 {
            ax = ax.wrapping_neg();
        }
        if negative.1 {
            ay = ay.wrapping_neg();
        }
        let mut halvings = 0;
        while ax.max(ay) >= 256 {
            ax >>= 1;
            ay >>= 1;
            halvings += 1;
        }
        let quotient = ax.max(ay).checked_div(u16::from(speed)).unwrap_or(0xFFFF);
        Self {
            size: (ax, ay),
            negative,
            steps: if quotient == 0 {
                0
            } else {
                (quotient + 1) & 0x3FFF
            },
            step: 0,
            reached: (0, 0),
            legs: (u16::from(legs) + 1) << halvings,
            limit: if limit < 0x80 { limit } else { 0 },
        }
    }

    /// One frame (`COP CD`).
    pub(super) fn frame(&mut self) -> Frame {
        let sign = |value: u16, negative: bool| {
            let value = value.cast_signed();
            if negative {
                value.wrapping_neg()
            } else {
                value
            }
        };
        let (mut dx, mut dy) = (0, 0);
        loop {
            if self.step == self.steps {
                // The leg's rest: none unless it had no steps.
                dx = sign(self.size.0.wrapping_sub(self.reached.0), self.negative.0);
                dy = sign(self.size.1.wrapping_sub(self.reached.1), self.negative.1);
                if self.steps == 0 {
                    return Frame::Ended(dx, dy);
                }
            } else {
                let divisor = (self.steps - 1) & 0xFF;
                let along = |size: u16| {
                    (u32::from(self.step) * u32::from(size))
                        .checked_div(u32::from(divisor))
                        .map_or(0xFFFF, |value| u16::try_from(value).unwrap_or(0xFFFF))
                };
                let (qx, qy) = (along(self.size.0), along(self.size.1));
                let low = |value: u16| i16::from(i8::from_ne_bytes([value.to_le_bytes()[0]]));
                if qx != self.reached.0 {
                    dx =
                        low(sign(qx.wrapping_sub(self.reached.0), self.negative.0).cast_unsigned());
                    self.reached.0 = qx;
                }
                if qy != self.reached.1 {
                    dy =
                        low(sign(qy.wrapping_sub(self.reached.1), self.negative.1).cast_unsigned());
                    self.reached.1 = qy;
                }
                let timed_out = self.limit > 0 && {
                    self.limit -= 1;
                    self.limit == 0
                };
                if !timed_out {
                    self.step += 1;
                    return Frame::Moving(dx, dy);
                }
                // The limit ends every leg left.
                return Frame::Ended(dx, dy);
            }
            self.legs = self.legs.saturating_sub(1);
            if self.legs == 0 {
                return Frame::Ended(dx, dy);
            }
            (self.step, self.reached) = (0, (0, 0));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The moves until the end, the last frame's included.
    fn run(mut line: Line) -> Vec<(i16, i16)> {
        let mut moves = Vec::new();
        loop {
            match line.frame() {
                Frame::Moving(dx, dy) => moves.push((dx, dy)),
                Frame::Ended(dx, dy) => {
                    moves.push((dx, dy));
                    return moves;
                }
            }
        }
    }

    fn sum(moves: &[(i16, i16)]) -> (i16, i16) {
        moves
            .iter()
            .fold((0, 0), |(x, y), (dx, dy)| (x + dx, y + dy))
    }

    #[test]
    fn the_flyer_chases_one_pixel_a_frame_for_24_frames() {
        // `CC 00 08 01 18 FF` to (+40,+30): still, then (1,0),(1,1),...
        let moves = run(Line::start((184, 352), (224, 382), 0, 1, 0x18));
        assert_eq!(moves.len(), 24);
        assert_eq!(&moves[..5], &[(0, 0), (1, 0), (1, 1), (1, 1), (1, 1)]);
        assert_eq!(sum(&moves), (23, 17));
    }

    #[test]
    fn the_hiball_lunges_five_times_the_vector() {
        // `CC 04 08 03 00 FF` to (-20,+6): 5 legs of 7 frames, 36 frames.
        let moves = run(Line::start((184, 352), (164, 358), 4, 3, 0));
        assert_eq!(moves.len(), 36);
        assert_eq!(
            &moves[..7],
            &[(0, 0), (-3, 1), (-3, 1), (-4, 1), (-3, 1), (-3, 1), (-4, 1)]
        );
        assert_eq!(moves[35], (0, 0));
        assert_eq!(sum(&moves), (-100, 30));
    }

    #[test]
    fn a_long_vector_is_halved_and_run_twice() {
        // (+300,-10) at speed 4: 2 legs of 38 frames, 77 frames.
        let moves = run(Line::start((184, 352), (484, 342), 0, 4, 0));
        assert_eq!(moves.len(), 77);
        assert_eq!(sum(&moves), (300, -10));
        // With a limit of 10: 9 steps, the end on frame 10.
        let moves = run(Line::start((184, 352), (484, 342), 0, 4, 10));
        assert_eq!(moves.len(), 10);
    }
}
