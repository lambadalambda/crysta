//! Movement streams: the per-axis records a pose's movement selector names
//! in a movement resource (`$80:BBDB` starts them, `$80:F251` steps them).
//!
//! A resource sits at a WRAM base (`$7F:4000 + ...`): the common one at
//! `$7F:6000` (`$AB:F037`), or an actor's own, from its descriptor's
//! movement pointer. Selector `s` holds an X and a Y stream pointer at
//! `base + 4s`. A stream is `(count, velocity)` pairs, each velocity applied
//! `count + 1` frames, ended by `$FFFF` and a pointer to loop back to; an
//! actor whose `+$06` has bit `$80` stops there instead. Velocities are
//! whole pixels, negated on a flipped axis. Each time the pose's display
//! list starts over, the streams start over too (`$7F:0014`/`0016` keep
//! their first pointers): the town walker's up walk moves 16 pixels in
//! each 31-tick repetition, 64 in four. The Y axis is never flipped here:
//! vertical flips (`+$08` bit `$8000`) are not modelled.

use std::rc::Rc;

/// A movement resource and where it sits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Resource {
    pub(super) base: u16,
    pub(super) bytes: Rc<[u8]>,
}

impl Resource {
    fn word(&self, address: u16) -> Option<u16> {
        let at = usize::from(address.checked_sub(self.base)?);
        let bytes = self.bytes.get(at..at + 2)?;
        Some(u16::from_le_bytes([bytes[0], bytes[1]]))
    }
}

/// One axis: the record pointer (0 when stopped) and its countdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Axis {
    pointer: u16,
    counter: i32,
    flipped: bool,
}

impl Axis {
    /// One frame's velocity; `None` when the stream leaves the resource.
    fn step(&mut self, resource: &Resource, stops: bool) -> Option<i16> {
        if self.pointer == 0 {
            return Some(0);
        }
        self.counter -= 1;
        if self.counter < 0 {
            self.pointer = self.pointer.wrapping_add(2);
            let mut count = resource.word(self.pointer)?;
            if count & 0x8000 != 0 {
                if stops {
                    self.pointer = 0;
                    return Some(0);
                }
                self.pointer = resource.word(self.pointer.wrapping_add(2))?;
                count = resource.word(self.pointer)?;
            }
            self.counter = i32::from(count);
            self.pointer = self.pointer.wrapping_add(2);
        }
        let velocity = resource.word(self.pointer)?.cast_signed();
        Some(if self.flipped {
            velocity.wrapping_neg()
        } else {
            velocity
        })
    }
}

/// A selector's two streams under way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Motion {
    resource: Resource,
    x: Axis,
    y: Axis,
    /// The streams as they started, for each repetition of the pose.
    start: (Axis, Axis),
    /// `+$06` bit `$80`: stop at a loop instead of looping.
    stops: bool,
    /// Ticks of the pose's display list, and ticks into it.
    list: Option<u16>,
    tick: u16,
}

impl Motion {
    /// Starts `selector`'s streams, the X one flipped when the actor is,
    /// for a pose whose display list lasts `list` ticks.
    pub(super) fn start(
        resource: Resource,
        selector: u8,
        hflip: bool,
        stops: bool,
        list: Option<u16>,
    ) -> Option<Self> {
        let at = resource.base.checked_add(u16::from(selector) * 4)?;
        let axis = |pointer, flipped| Axis {
            pointer,
            counter: 0,
            flipped,
        };
        let (x, y) = (
            axis(resource.word(at)?, hflip),
            axis(resource.word(at + 2)?, false),
        );
        Some(Self {
            x,
            y,
            start: (x, y),
            resource,
            stops,
            list: list.filter(|&ticks| ticks > 0),
            tick: 0,
        })
    }

    /// One frame's move.
    pub(super) fn step(&mut self) -> Option<(i16, i16)> {
        if self.list == Some(self.tick) {
            (self.x, self.y) = self.start;
            self.tick = 0;
        }
        self.tick += 1;
        Some((
            self.x.step(&self.resource, self.stops)?,
            self.y.step(&self.resource, self.stops)?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The town walker's own resource (`$D2:7FB1`), as native WRAM holds it
    /// at `$7F:4000`: selector 9 is the hop down.
    fn town() -> Resource {
        let words: [u16; 53] = [
            0, 0, 0, 0, 0, 0, 0, 0x4028, 0, 0x4030, 0x4038, 0, 0, 0x4040, 0, 0x4048, 0x4050, 0, 0,
            0x4058, 0xFFFF, 0x1F, 1, 0xFFFF, 0x402A, 0x1E, 0xFFFF, 0xFFFF, 0x4032, 0x1F, 1, 0xFFFF,
            0x403A, 0x17, 2, 0xFFFF, 0x4042, 0x17, 0xFFFE, 0xFFFF, 0x404A, 0x17, 2, 0xFFFF, 0x4052,
            1, 0, 3, 2, 7, 1, 0xFFFF, 0x405A,
        ];
        Resource {
            base: 0x4000,
            bytes: words.iter().flat_map(|word| word.to_le_bytes()).collect(),
        }
    }

    #[test]
    fn the_hop_rests_two_frames_then_moves_eight_and_eight_pixels_and_loops() {
        let mut motion = Motion::start(town(), 9, false, false, None).unwrap();
        let ys: Vec<i16> = (0..16).map(|_| motion.step().unwrap().1).collect();
        assert_eq!(ys, [0, 0, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0]);
        assert_eq!(ys.iter().sum::<i16>(), 16);
    }

    #[test]
    fn each_repetition_of_the_pose_starts_the_streams_over() {
        // A four-tick list: rest, rest, 2, 2 -- then from the start again.
        let mut motion = Motion::start(town(), 9, false, false, Some(4)).unwrap();
        let ys: Vec<i16> = (0..8).map(|_| motion.step().unwrap().1).collect();
        assert_eq!(ys, [0, 0, 2, 2, 0, 0, 2, 2]);
    }

    #[test]
    fn a_stopping_actor_ends_at_the_loop_and_a_flip_negates_x() {
        let mut motion = Motion::start(town(), 9, false, true, None).unwrap();
        let moved: i16 = (0..40).map(|_| motion.step().unwrap().1).sum();
        assert_eq!(moved, 16, "no second hop");
        // Selector 5 moves X by 1 a frame; flipped, by -1.
        let mut right = Motion::start(town(), 5, false, false, None).unwrap();
        let mut left = Motion::start(town(), 5, true, false, None).unwrap();
        assert_eq!(right.step().unwrap().0, 1);
        assert_eq!(left.step().unwrap().0, -1);
    }
}
