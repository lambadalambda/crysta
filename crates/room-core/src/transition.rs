//! Endpoint-qualified doorway preview, deliberately not a native actor scheduler.
//!
//! The 17+load+17 logical-update policy preserves measured endpoints. It does
//! not reproduce loading stalls, COP scheduling or reference video-frame timing.

/// An explicitly opted-in semantic doorway pair, accepted only at its qualified handoff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Transition {
    elapsed: u8,
    handoff_y: u16,
}
impl Transition {
    pub(crate) fn start(map_id: u16, position: (u16, u16)) -> Option<Self> {
        if !matches!(
            (map_id, position),
            (15, (392, 208 | 209)) | (16, (392, 336))
        ) {
            return None;
        }
        Some(Self {
            elapsed: 0,
            handoff_y: position.1,
        })
    }
    pub(crate) fn source_map(self) -> u16 {
        if self.handoff_y == 336 {
            16
        } else {
            15
        }
    }
    pub(crate) fn handoff(self) -> (u16, u16) {
        (392, self.handoff_y)
    }
    pub(crate) fn direction(self) -> crate::Direction {
        if self.handoff_y == 336 {
            crate::Direction::Up
        } else {
            crate::Direction::Down
        }
    }
    pub(crate) fn advance(&mut self) {
        self.elapsed = self.elapsed.saturating_add(1).min(35);
    }
    pub(crate) fn position(self) -> (u16, u16) {
        let elapsed = u16::from(self.elapsed);
        let y = match (self.handoff_y == 336, self.elapsed) {
            (false, 0..=17) => self.handoff_y + elapsed,
            (false, _) => 336 + elapsed - 18,
            (true, 0..=17) => 336 - elapsed,
            (true, _) => 208 - (elapsed - 18),
        };
        (392, y)
    }
    pub(crate) fn map_id(self) -> u16 {
        if self.elapsed <= 17 {
            self.source_map()
        } else {
            31 - self.source_map()
        }
    }
    pub(crate) fn complete(self) -> bool {
        self.elapsed == 35
    }
    pub(crate) fn elapsed(self) -> u8 {
        self.elapsed
    }
    pub(crate) fn encoded(self) -> u8 {
        self.elapsed
            + match self.handoff_y {
                336 => 64,
                208 => 128,
                _ => 0,
            }
    }
    pub(crate) fn restore(encoded: u8) -> Option<Self> {
        match encoded {
            0..=34 => Some(Self {
                elapsed: encoded,
                handoff_y: 209,
            }),
            64..=98 => Some(Self {
                elapsed: encoded - 64,
                handoff_y: 336,
            }),
            128..=162 => Some(Self {
                elapsed: encoded - 128,
                handoff_y: 208,
            }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_preview_preserves_endpoints_not_video_cadence() {
        let mut t = Transition::start(15, (392, 209)).unwrap();
        for _ in 0..17 {
            t.advance();
        }
        assert_eq!(t.position(), (392, 226));
        assert_eq!(t.map_id(), 15);
        t.advance();
        assert_eq!(t.position(), (392, 336));
        assert_eq!(t.map_id(), 16);
        for _ in 0..17 {
            t.advance();
        }
        assert_eq!(t.position(), (392, 353));
        assert!(t.complete());
        assert_eq!(t.elapsed(), 35);
        assert!(Transition::restore(35).is_none());
        assert_eq!(Transition::restore(18).unwrap().position(), (392, 336));
    }
    #[test]
    fn reverse_doorway_preserves_qualified_endpoints_and_encoding() {
        let mut t = Transition::start(16, (392, 336)).unwrap();
        assert_eq!(t.source_map(), 16);
        assert_eq!(t.handoff(), (392, 336));
        assert_eq!(t.direction(), crate::Direction::Up);
        for elapsed in 0..35 {
            assert_eq!(Transition::restore(t.encoded()), Some(t));
            assert_eq!(t.elapsed(), elapsed);
            t.advance();
            match elapsed + 1 {
                17 => assert_eq!((t.map_id(), t.position()), (16, (392, 319))),
                18 => assert_eq!((t.map_id(), t.position()), (15, (392, 208))),
                35 => assert_eq!((t.map_id(), t.position()), (15, (392, 191))),
                _ => (),
            }
        }
        assert!(t.complete());
        for byte in 35..64 {
            assert!(Transition::restore(byte).is_none());
        }
        for byte in (99..128).chain(163..=255) {
            assert!(Transition::restore(byte).is_none());
        }
        assert!(Transition::start(15, (392, 336)).is_none());
        assert!(Transition::start(16, (392, 209)).is_none());
        assert!(Transition::start(17, (392, 336)).is_none());
    }
    #[test]
    fn fresh_route_hands_off_at_208_without_inventing_a_walking_frame() {
        let mut t = Transition::start(15, (392, 208)).unwrap();
        assert_eq!(Transition::restore(t.encoded()), Some(t));
        for _ in 0..17 {
            t.advance();
        }
        assert_eq!((t.map_id(), t.position()), (15, (392, 225)));
        t.advance();
        assert_eq!((t.map_id(), t.position()), (16, (392, 336)));
        for _ in 0..17 {
            t.advance();
        }
        assert_eq!(t.position(), (392, 353));
        assert!(t.complete());
    }
    #[test]
    fn unrelated_handoff_is_not_accepted() {
        assert!(Transition::start(15, (391, 209)).is_none());
        assert!(Transition::start(15, (392, 207)).is_none());
    }
}
