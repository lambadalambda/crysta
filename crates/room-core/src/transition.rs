//! Source-selected, endpoint-qualified 17 + load + 17 logical doorway policy.
use crate::{
    house::{Doorway, DOORWAYS},
    Direction,
};

/// Shared checked 17 departure / load / 17 arrival clock, including handoff0.
pub(crate) fn doorway_position(
    handoff: (u16, u16),
    loaded: (u16, u16),
    direction: Direction,
    elapsed: u8,
) -> Option<(u16, u16)> {
    if elapsed > 35 {
        return None;
    }
    let (x, y) = if elapsed <= 17 { handoff } else { loaded };
    let n = u16::from(if elapsed <= 17 { elapsed } else { elapsed - 18 });
    Some(match direction {
        Direction::Down => (x, y.checked_add(n)?),
        Direction::Up => (x, y.checked_sub(n)?),
        Direction::Left => (x.checked_sub(n)?, y),
        Direction::Right => (x.checked_add(n)?, y),
    })
}

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
            .chain(core::iter::once(&crate::house::EXTERIOR))
            .position(|s| s.source == map && s.index == index)?;
        let spec = Self::route_spec(u8::try_from(route).ok()?)?;
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
        Self::route_spec(self.route).expect("validated route")
    }
    fn route_spec(route: u8) -> Option<Doorway> {
        if usize::from(route) == DOORWAYS.len() {
            Some(crate::house::EXTERIOR)
        } else {
            DOORWAYS.get(usize::from(route)).copied()
        }
    }
    pub(crate) fn exterior(self) -> bool {
        usize::from(self.route) == DOORWAYS.len()
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
        doorway_position(
            self.handoff,
            self.spec().anchor,
            self.direction(),
            self.elapsed,
        )
        .expect("validated doorway clock")
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
        let spec = Self::route_spec(route)?;
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
    fn checked_clock_covers_both_phases_and_all_directions() {
        for (direction, departure, arrival) in [
            (Direction::Down, (100, 117), (200, 217)),
            (Direction::Up, (100, 83), (200, 183)),
            (Direction::Left, (83, 100), (183, 200)),
            (Direction::Right, (117, 100), (217, 200)),
        ] {
            for (elapsed, expected) in [
                (0, (100, 100)),
                (17, departure),
                (18, (200, 200)),
                (35, arrival),
            ] {
                assert_eq!(
                    doorway_position((100, 100), (200, 200), direction, elapsed),
                    Some(expected)
                );
            }
            for elapsed in [36, 255] {
                assert_eq!(
                    doorway_position((100, 100), (200, 200), direction, elapsed),
                    None
                );
            }
            let edge = if matches!(direction, Direction::Up | Direction::Left) {
                (0, 0)
            } else {
                (u16::MAX, u16::MAX)
            };
            assert_eq!(doorway_position(edge, (200, 200), direction, 1), None);
            assert_eq!(doorway_position((100, 100), edge, direction, 19), None);
        }
    }
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
