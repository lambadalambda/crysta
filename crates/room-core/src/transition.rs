//! Endpoint-qualified doorway preview, deliberately not a native actor scheduler.
//!
//! The 17+load+17 logical-update policy preserves measured endpoints. It does
//! not reproduce loading stalls, COP scheduling or reference video-frame timing.

/// One explicitly opted-in semantic doorway, accepted only at its qualified handoff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Transition {
    elapsed: u8,
}
impl Transition {
    pub(crate) fn start(position: (u16, u16)) -> Option<Self> {
        (position == (392, 209)).then_some(Self { elapsed: 0 })
    }
    pub(crate) fn advance(&mut self) {
        self.elapsed = self.elapsed.saturating_add(1).min(35);
    }
    pub(crate) fn position(self) -> (u16, u16) {
        match self.elapsed {
            0..=17 => (392, 209 + u16::from(self.elapsed)),
            n => (392, 336 + u16::from(n - 18)),
        }
    }
    pub(crate) fn map_id(self) -> u16 {
        if self.elapsed <= 17 {
            15
        } else {
            16
        }
    }
    pub(crate) fn complete(self) -> bool {
        self.elapsed == 35
    }
    pub(crate) fn elapsed(self) -> u8 {
        self.elapsed
    }
    pub(crate) fn restore(elapsed: u8) -> Option<Self> {
        (elapsed < 35).then_some(Self { elapsed })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_preview_preserves_endpoints_not_video_cadence() {
        let mut t = Transition::start((392, 209)).unwrap();
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
    fn unrelated_handoff_is_not_accepted() {
        assert!(Transition::start((391, 209)).is_none());
        assert!(Transition::start((392, 208)).is_none());
    }
}
