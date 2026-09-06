//! Immutable source exit identities and endpoint-qualified house doorway specs.
use crate::{slice::Exit, Direction};
pub(crate) const MAPS: [u16; 6] = [11, 12, 13, 15, 16, 17];
pub(crate) fn exits(map: u16) -> Option<&'static [Exit]> {
    Some(match map {
        11 => &[
            Exit([7, 12, 1, 4, 12, 0, 0, 5, 128, 0, 80, 1]), // $818DC0
        ],
        12 => &[
            Exit([7, 28, 1, 4, 13, 0, 0, 5, 112, 0, 96, 2]), // $818DCD
            Exit([14, 25, 1, 2, 16, 0, 0, 8, 32, 1, 160, 1]), // $818DD9
            Exit([8, 16, 1, 5, 11, 0, 0, 6, 112, 0, 176, 0]), // $818DE5
            Exit([11, 21, 1, 1, 14, 0, 0, 14, 144, 0, 96, 3]), // $818DF1
        ],
        13 => &[
            Exit([7, 44, 1, 4, 10, 0, 0, 5, 240, 1, 240, 2]), // $818DFE
            Exit([7, 32, 1, 6, 12, 0, 0, 6, 112, 0, 176, 1]), // $818E0A
            Exit([14, 41, 1, 2, 17, 0, 0, 8, 32, 1, 160, 2]), // $818E16
        ],
        15 => &[
            Exit([24, 12, 1, 2, 16, 0, 0, 5, 128, 1, 80, 1]), // $818E3C
            Exit([27, 5, 1, 1, 34, 1, 0, 0, 120, 1, 96, 0]),  // $818E48
        ],
        16 => &[
            Exit([24, 20, 1, 1, 15, 0, 0, 6, 128, 1, 176, 0]), // $818E55
            Exit([17, 25, 1, 2, 12, 0, 0, 7, 208, 0, 160, 1]), // $818E61
            Exit([22, 29, 1, 1, 17, 0, 0, 5, 96, 1, 80, 2]),   // $818E6D
        ],
        17 => &[
            Exit([22, 36, 1, 1, 16, 0, 0, 6, 96, 1, 192, 1]), // $818E7A
            Exit([17, 41, 1, 2, 13, 0, 0, 7, 208, 0, 160, 2]), // $818E86
        ],
        _ => return None,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Doorway {
    pub source: u16,
    pub index: usize,
    pub destination: u16,
    pub direction: Direction,
    pub handoff: (u16, u16),
    pub anchor: (u16, u16),
    pub endpoint: (u16, u16),
}
// Source queue placement plus player (+8,+16); native settled endpoints.
// Index is the source list ordinal, NOT an unordered destination lookup.
pub(crate) const DOORWAYS: [Doorway; 12] = [
    Doorway {
        source: 15,
        index: 0,
        destination: 16,
        direction: Direction::Down,
        handoff: (392, 208),
        anchor: (392, 336),
        endpoint: (392, 353),
    },
    Doorway {
        source: 16,
        index: 0,
        destination: 15,
        direction: Direction::Up,
        handoff: (392, 336),
        anchor: (392, 208),
        endpoint: (392, 191),
    },
    Doorway {
        source: 11,
        index: 0,
        destination: 12,
        direction: Direction::Down,
        handoff: (120, 208),
        anchor: (136, 336),
        endpoint: (136, 353),
    },
    Doorway {
        source: 12,
        index: 0,
        destination: 13,
        direction: Direction::Down,
        handoff: (120, 464),
        anchor: (120, 608),
        endpoint: (120, 625),
    },
    Doorway {
        source: 12,
        index: 1,
        destination: 16,
        direction: Direction::Right,
        handoff: (232, 432),
        anchor: (280, 432),
        endpoint: (297, 432),
    },
    Doorway {
        source: 12,
        index: 2,
        destination: 11,
        direction: Direction::Up,
        handoff: (136, 336),
        anchor: (120, 208),
        endpoint: (120, 191),
    },
    Doorway {
        source: 13,
        index: 1,
        destination: 12,
        direction: Direction::Up,
        handoff: (120, 608),
        anchor: (120, 464),
        endpoint: (120, 447),
    },
    Doorway {
        source: 13,
        index: 2,
        destination: 17,
        direction: Direction::Right,
        handoff: (232, 672),
        anchor: (280, 688),
        endpoint: (297, 688),
    },
    Doorway {
        source: 16,
        index: 1,
        destination: 12,
        direction: Direction::Left,
        handoff: (280, 416),
        anchor: (232, 432),
        endpoint: (215, 432),
    },
    Doorway {
        source: 16,
        index: 2,
        destination: 17,
        direction: Direction::Down,
        handoff: (360, 480),
        anchor: (360, 592),
        endpoint: (360, 609),
    },
    Doorway {
        source: 17,
        index: 0,
        destination: 16,
        direction: Direction::Up,
        handoff: (360, 592),
        anchor: (360, 480),
        endpoint: (360, 463),
    },
    Doorway {
        source: 17,
        index: 1,
        destination: 13,
        direction: Direction::Left,
        handoff: (280, 688),
        anchor: (232, 688),
        endpoint: (215, 688),
    },
];
