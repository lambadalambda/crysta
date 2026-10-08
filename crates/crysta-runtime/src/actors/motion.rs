//! Movement streams: the per-axis records a pose's movement selector names
//! in a movement resource (`$80:BBDB` starts them, `$80:F251` steps them).
//!
//! A resource sits at a WRAM base (`$7F:4000 + ...`): the common one at
//! `$7F:6000` (`$AB:F037`), or an actor's own, from its descriptor's
//! movement pointer. Selector `s` holds an X and a Y stream pointer at
//! `base + 4s`. A stream is `(count, velocity)` pairs, each velocity applied
//! `count + 1` frames, ended by `$FFFF` and a pointer to loop back to; an
//! actor whose `+$06` has bit `$80` stops there instead. Velocities are
//! whole pixels, negated on a flipped axis: X while the actor is mirrored
//! (`+$08` bit `$4000`), Y while it is flipped vertically (bit `$8000`),
//! both read each frame (`$80:F297`, `F2F3`). Each time the pose's display
//! list starts over, the streams start over too (`$7F:0014`/`0016` keep
//! their first pointers): the town walker's up walk moves 16 pixels in
//! each 31-tick repetition, 64 in four.

use std::rc::Rc;

/// A movement resource and where it sits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Resource {
    /// The address of the first byte.
    base: u16,
    bytes: Rc<[u8]>,
    /// The selector table's address (`$7F:0022`).
    table: u16,
    /// What stream pointers are relative to (`$7F:0026`): 0 in WRAM.
    shift: u16,
}

impl Resource {
    /// A resource in WRAM at `base`, its pointers absolute.
    pub(super) fn wram(base: u16, bytes: Rc<[u8]>) -> Self {
        Self {
            base,
            bytes,
            table: base,
            shift: 0,
        }
    }

    /// A resource in a ROM bank, `COP B0 FF` (`$80:A99B`): the table at
    /// `table`, the stream pointers in it and their loops relative to it
    /// (`$80:F313`).
    pub(super) fn rom(bank: Rc<[u8]>, table: u16) -> Self {
        Self {
            base: 0,
            bytes: bank,
            table,
            shift: table,
        }
    }

    fn word(&self, address: u16) -> Option<u16> {
        let at = usize::from(address.checked_sub(self.base)?);
        let bytes = self.bytes.get(at..at + 2)?;
        Some(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    /// The word a stream pointer names.
    fn at(&self, pointer: u16) -> Option<u16> {
        self.word(pointer.wrapping_add(self.shift))
    }
}

/// One axis: the record pointer (0 when stopped) and its countdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Axis {
    pointer: u16,
    counter: i32,
}

impl Axis {
    /// One frame's velocity; `None` when the stream leaves the resource.
    fn step(&mut self, resource: &Resource, stops: bool, flipped: bool) -> Option<i16> {
        if self.pointer == 0 {
            return Some(0);
        }
        self.counter -= 1;
        if self.counter < 0 {
            self.pointer = self.pointer.wrapping_add(2);
            let mut count = resource.at(self.pointer)?;
            if count & 0x8000 != 0 {
                if stops {
                    self.pointer = 0;
                    return Some(0);
                }
                self.pointer = resource.at(self.pointer.wrapping_add(2))?;
                count = resource.at(self.pointer)?;
            }
            self.counter = i32::from(count);
            self.pointer = self.pointer.wrapping_add(2);
        }
        let velocity = resource.at(self.pointer)?.cast_signed();
        Some(if flipped {
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
    /// Starts `selector`'s streams for a pose whose display list lasts
    /// `list` ticks.
    pub(super) fn start(
        resource: Resource,
        selector: u8,
        stops: bool,
        list: Option<u16>,
    ) -> Option<Self> {
        let at = resource.table.checked_add(u16::from(selector) * 4)?;
        let axis = |pointer| Axis {
            pointer,
            counter: 0,
        };
        let (x, y) = (axis(resource.word(at)?), axis(resource.word(at + 2)?));
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

    /// Whether either stream still runs (`7F:0010 | 7F:0012`, as `COP E4`
    /// waits on it).
    pub(super) const fn running(&self) -> bool {
        self.x.pointer != 0 || self.y.pointer != 0
    }

    /// Stops the blocked axes' streams (`7F:0010`/`0012` = 0); the pose's
    /// next repetition starts them again.
    pub(super) fn stop(&mut self, (x, y): (bool, bool)) {
        if x {
            self.x.pointer = 0;
        }
        if y {
            self.y.pointer = 0;
        }
    }

    /// One frame's move, the axes negated by the actor's `(mirrored,
    /// flipped)`.
    pub(super) fn step(&mut self, (hflip, vflip): (bool, bool)) -> Option<(i16, i16)> {
        if self.list == Some(self.tick) {
            (self.x, self.y) = self.start;
            self.tick = 0;
        }
        self.tick += 1;
        Some((
            self.x.step(&self.resource, self.stops, hflip)?,
            self.y.step(&self.resource, self.stops, vflip)?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UPRIGHT: (bool, bool) = (false, false);

    /// The town walker's own resource (`$D2:7FB1`), as native WRAM holds it
    /// at `$7F:4000`: selector 9 is the hop down.
    fn town() -> Resource {
        let words: [u16; 53] = [
            0, 0, 0, 0, 0, 0, 0, 0x4028, 0, 0x4030, 0x4038, 0, 0, 0x4040, 0, 0x4048, 0x4050, 0, 0,
            0x4058, 0xFFFF, 0x1F, 1, 0xFFFF, 0x402A, 0x1E, 0xFFFF, 0xFFFF, 0x4032, 0x1F, 1, 0xFFFF,
            0x403A, 0x17, 2, 0xFFFF, 0x4042, 0x17, 0xFFFE, 0xFFFF, 0x404A, 0x17, 2, 0xFFFF, 0x4052,
            1, 0, 3, 2, 7, 1, 0xFFFF, 0x405A,
        ];
        Resource::wram(
            0x4000,
            words.iter().flat_map(|word| word.to_le_bytes()).collect(),
        )
    }

    #[test]
    fn the_hop_rests_two_frames_then_moves_eight_and_eight_pixels_and_loops() {
        let mut motion = Motion::start(town(), 9, false, None).unwrap();
        let ys: Vec<i16> = (0..16).map(|_| motion.step(UPRIGHT).unwrap().1).collect();
        assert_eq!(ys, [0, 0, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0]);
        assert_eq!(ys.iter().sum::<i16>(), 16);
    }

    #[test]
    fn each_repetition_of_the_pose_starts_the_streams_over() {
        // A four-tick list: rest, rest, 2, 2 -- then from the start again.
        let mut motion = Motion::start(town(), 9, false, Some(4)).unwrap();
        let ys: Vec<i16> = (0..8).map(|_| motion.step(UPRIGHT).unwrap().1).collect();
        assert_eq!(ys, [0, 0, 2, 2, 0, 0, 2, 2]);
    }

    #[test]
    fn a_stopping_actor_ends_at_the_loop_and_a_flip_negates_x() {
        let mut motion = Motion::start(town(), 9, true, None).unwrap();
        let moved: i16 = (0..40).map(|_| motion.step(UPRIGHT).unwrap().1).sum();
        assert_eq!(moved, 16, "no second hop");
        // Selector 5 moves X by 1 a frame; mirrored, by -1.
        let mut right = Motion::start(town(), 5, false, None).unwrap();
        assert_eq!(right.step(UPRIGHT).unwrap().0, 1);
        assert_eq!(right.step((true, false)).unwrap().0, -1);
    }

    #[test]
    fn a_rom_resource_takes_its_stream_pointers_from_its_table() {
        // `COP B0 FF $8000 bank` (`$80:A99B`): the table at $8000, and the
        // stream pointers it holds and their loops relative to it
        // (`$80:F313`): X at $8010, 2 frames of 3, looping.
        let mut bank = vec![0u8; 0x8020];
        for (at, word) in [
            (0x8000, 0x10),
            (0x8012, 1),
            (0x8014, 3),
            (0x8016, 0xFFFF),
            (0x8018, 0x12),
        ] {
            bank[at..at + 2].copy_from_slice(&u16::to_le_bytes(word));
        }
        let resource = Resource::rom(bank.into(), 0x8000);
        let mut motion = Motion::start(resource, 0, false, None).unwrap();
        let xs: Vec<i16> = (0..6).map(|_| motion.step(UPRIGHT).unwrap().0).collect();
        assert_eq!(xs, [3; 6]);
    }

    #[test]
    fn the_flips_are_read_each_frame_and_a_vertical_one_negates_y() {
        // `$80:F297`/`F2F3`: `+$08` bits `$4000` and `$8000`, each frame.
        let mut motion = Motion::start(town(), 9, false, None).unwrap();
        let ys: Vec<i16> = [UPRIGHT, UPRIGHT, (false, true), (false, true), UPRIGHT]
            .into_iter()
            .map(|flips| motion.step(flips).unwrap().1)
            .collect();
        assert_eq!(ys, [0, 0, -2, -2, 2]);
    }
}
