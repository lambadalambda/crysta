//! Source-selected, endpoint-qualified 17 + load + 17 logical doorway policy.
use crate::{
    house::{Doorway, DOORWAYS},
    Direction,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Transition {
    route: u8,
    elapsed: u8,
    handoff: (u16, u16),
}
impl Transition {
    #[cfg(test)]
    pub(crate) fn start(map: u16, position: (u16, u16)) -> Option<Self> {
        Self::select(map, 0, position)
    }
    pub(crate) fn select(map: u16, index: usize, position: (u16, u16)) -> Option<Self> {
        let route = DOORWAYS
            .iter()
            .position(|s| s.source == map && s.index == index)?;
        let spec = DOORWAYS[route];
        let valid = match spec.direction {
            Direction::Down => {
                position.0 == spec.handoff.0
                    && (spec.handoff.1..=spec.handoff.1 + 1).contains(&position.1)
            }
            Direction::Up => position == spec.handoff,
            Direction::Left | Direction::Right => {
                let exit = crate::house::exits(map)?[index];
                position.0 == spec.handoff.0
                    && exit.fine(position.0 - 8, position.1.checked_sub(16)?)
            }
        };
        let route = u8::try_from(route).ok()?;
        valid.then_some(Self {
            route,
            elapsed: 0,
            handoff: position,
        })
    }
    fn spec(self) -> Doorway {
        DOORWAYS[usize::from(self.route)]
    }
    pub(crate) fn source_map(self) -> u16 {
        self.spec().source
    }
    pub(crate) fn handoff(self) -> (u16, u16) {
        self.handoff
    }
    pub(crate) fn direction(self) -> Direction {
        self.spec().direction
    }
    pub(crate) fn advance(&mut self) {
        self.elapsed = self.elapsed.saturating_add(1).min(35);
    }
    pub(crate) fn position(self) -> (u16, u16) {
        let (x, y) = if self.elapsed <= 17 {
            self.handoff
        } else {
            self.spec().anchor
        };
        let n = u16::from(if self.elapsed <= 17 {
            self.elapsed
        } else {
            self.elapsed - 18
        });
        match self.direction() {
            Direction::Down => (x, y + n),
            Direction::Up => (x, y - n),
            Direction::Left => (x - n, y),
            Direction::Right => (x + n, y),
        }
    }
    pub(crate) fn map_id(self) -> u16 {
        if self.elapsed <= 17 {
            self.source_map()
        } else {
            self.spec().destination
        }
    }
    pub(crate) fn complete(self) -> bool {
        self.elapsed == 35
    }
    pub(crate) fn elapsed(self) -> u8 {
        self.elapsed
    }
    pub(crate) fn route(self) -> u8 {
        self.route
    }
    pub(crate) fn index(self) -> usize {
        self.spec().index
    }
    pub(crate) fn restore(route: u8, elapsed: u8, handoff: (u16, u16)) -> Option<Self> {
        let spec = DOORWAYS.get(usize::from(route))?;
        let mut t = Self::select(spec.source, spec.index, handoff)?;
        if elapsed >= 35 {
            return None;
        }
        t.elapsed = elapsed;
        Some(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_endpoints_and_every_owned_snapshot() {
        for (route, spec) in DOORWAYS.iter().enumerate() {
            let mut t = Transition::select(spec.source, spec.index, spec.handoff).unwrap();
            for elapsed in 0..35 {
                assert_eq!(
                    Transition::restore(u8::try_from(route).unwrap(), elapsed, t.handoff()),
                    Some(t)
                );
                t.advance();
                if elapsed == 17 {
                    assert_eq!(t.position(), spec.anchor);
                }
            }
            assert_eq!(t.position(), spec.endpoint);
            assert_eq!(t.map_id(), spec.destination);
            assert!(t.complete());
            assert!(Transition::restore(u8::try_from(route).unwrap(), 35, t.handoff()).is_none());
        }
    }
    #[test]
    fn legacy_both_outbound_handoffs_and_reverse_are_preserved() {
        for y in [208, 209] {
            let mut t = Transition::start(15, (392, y)).unwrap();
            for _ in 0..17 {
                t.advance();
            }
            assert_eq!(t.position(), (392, y + 17));
            t.advance();
            assert_eq!(t.position(), (392, 336));
            for _ in 0..17 {
                t.advance();
            }
            assert_eq!(t.position(), (392, 353));
        }
        assert!(Transition::start(15, (391, 209)).is_none());
        assert!(Transition::start(15, (392, 207)).is_none());
        assert!(Transition::start(16, (392, 336)).is_some());
        assert!(Transition::start(17, (392, 336)).is_none());
    }
}
