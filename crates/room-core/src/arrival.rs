//! Two measured Crysta return-arrival profiles, relative to player initialization.
//!
//! These are lossless completed-frame change points, not a native scheduler.
//! Callers must bind the exact qualified exit record and mode before admission.
//! Departure and map-loader timing, arbitrary event states, diagonal artwork and
//! native input-history restoration are outside this bounded host policy.

/// Exact return edge whose initialized-to-free sequence was captured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReturnRoute {
    /// `$1E → $0A`, record `$818F9B`, mode0, selector5.
    Town,
    /// `$19 → $17`, record `$818F42`, mode0, selector14.
    Stairs,
}

/// Ownership boundaries observed independently of whether XY changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArrivalPhase {
    /// Player reconstructed; arrival initialization still owns the player.
    Initialized,
    /// Native arrival controller active, including holds at the endpoint.
    Forced,
    /// Stationary recovery; ordinary input must not move the player yet.
    Recovery,
    /// Ordinary continuation reached. Walking may start on the next advance.
    Free,
}

/// Immutable route selection plus a bounded completed-frame cursor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Arrival {
    route: ReturnRoute,
    elapsed: u8,
}

struct Profile {
    positions: &'static [(u8, u16, u16)],
    ownership: u8,
    recovery: u8,
    free: u8,
}

impl Arrival {
    /// Starts at cursor0, the already-installed initialized player position.
    #[must_use]
    pub const fn new(route: ReturnRoute) -> Self {
        Self { route, elapsed: 0 }
    }

    fn profile(self) -> Profile {
        match self.route {
            ReturnRoute::Town => Profile {
                positions: TOWN,
                ownership: 14,
                recovery: 35,
                free: 36,
            },
            ReturnRoute::Stairs => Profile {
                positions: STAIRS,
                ownership: 9,
                recovery: 77,
                free: 78,
            },
        }
    }

    /// Completed-frame advances since initialization; cursor0 is a sample too.
    #[must_use]
    pub const fn elapsed(self) -> u8 {
        self.elapsed
    }

    /// Observed position, holding the last change point without interpolation.
    ///
    /// # Panics
    /// Only if an internal profile is incorrectly edited to omit cursor0.
    #[must_use]
    pub fn position(self) -> (u16, u16) {
        let &(_, x, y) = self
            .profile()
            .positions
            .iter()
            .rev()
            .find(|&&(at, _, _)| at <= self.elapsed)
            .expect("profiles include cursor0");
        (x, y)
    }

    /// Current measured ownership phase, distinct from endpoint attainment.
    #[must_use]
    pub fn phase(self) -> ArrivalPhase {
        let profile = self.profile();
        if self.elapsed >= profile.free {
            ArrivalPhase::Free
        } else if self.elapsed >= profile.recovery {
            ArrivalPhase::Recovery
        } else if self.elapsed >= profile.ownership {
            ArrivalPhase::Forced
        } else {
            ArrivalPhase::Initialized
        }
    }

    /// Whether ordinary walking, interaction and exit scanning must stay suspended.
    #[must_use]
    pub fn owns_player(self) -> bool {
        self.phase() != ArrivalPhase::Free
    }

    /// Consumes one arrival advance only; saturates at the free boundary.
    pub fn advance(&mut self) {
        self.elapsed = self.elapsed.saturating_add(1).min(self.profile().free);
    }
}

// Pinned by tools/arrival-qualification/evidence.json and portable replay tests.
const TOWN: &[(u8, u16, u16)] = &[
    (0, 792, 752),
    (15, 792, 753),
    (18, 792, 754),
    (20, 792, 755),
    (21, 792, 756),
    (22, 792, 757),
    (23, 792, 758),
    (24, 792, 759),
    (25, 792, 760),
    (26, 792, 761),
    (27, 792, 762),
    (28, 792, 763),
    (29, 792, 764),
    (30, 792, 765),
    (31, 792, 766),
    (32, 792, 767),
    (33, 792, 768),
    (34, 792, 769),
];
const STAIRS: &[(u8, u16, u16)] = &[
    (0, 442, 345),
    (14, 444, 346),
    (18, 446, 347),
    (22, 448, 348),
    (27, 450, 349),
    (32, 452, 350),
    (36, 453, 351),
    (40, 454, 352),
    (44, 455, 353),
    (48, 455, 354),
    (52, 456, 355),
    (56, 456, 356),
    (60, 456, 357),
    (61, 456, 358),
    (62, 456, 359),
    (63, 456, 360),
    (66, 456, 361),
    (67, 456, 362),
    (68, 456, 363),
    (69, 456, 364),
    (72, 456, 365),
    (73, 456, 366),
    (74, 456, 367),
    (75, 456, 368),
];
