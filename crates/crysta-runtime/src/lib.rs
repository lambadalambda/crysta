//! Free-roam simulation over the decoded Crysta slice.
//!
//! [`room_core`] already walks a cell grid deterministically, and
//! [`assets::maps`] already decodes one. This crate is the join: it builds a
//! [`room_core::Room`] for any map in the slice, with the material policy the
//! collision work qualified.
//!
//! The rooms built here carry **no sample halo**. A halo is what makes the
//! qualification preview a corridor that fails closed at its edges; free roam
//! wants every cell the material policy admits, and nothing narrower.

use assets::maps::visual::{StaticBackground, VisualMapError};
use room_core::{
    Direction, FrameInput, MaterialAlias, MaterialPolicyError, MaterialRule, Room, Unqualified,
    WalkingState,
};
use std::fmt;

pub mod actors;
pub mod art;
pub mod residents;
pub mod scene;
pub mod world;

/// Maps the static exit graph bounds the Crysta slice to.
pub const MAPS: std::ops::RangeInclusive<u16> = 0x000A..=0x0021;

/// A map built into a walkable room.
#[derive(Debug, Clone)]
pub struct MapRoom {
    /// Collision and walking data for the map.
    pub room: Room,
    /// Map identifier this was built from.
    pub map: u16,
    /// Grid width in cells.
    pub width: u16,
    /// Grid height in cells.
    pub height: u16,
    /// The map's metatile attribute table (resource 3), 512 bytes, which a
    /// tile patch reads its collision attribute from.
    pub attributes: Vec<u8>,
}

impl MapRoom {
    /// The same room with other cells, keeping its material policy and
    /// collision mode.
    ///
    /// # Errors
    /// Propagates a room or policy refusal.
    pub fn with_cells(&self, cells: Vec<u16>) -> Result<Self, RoomError> {
        let map = self.map;
        let mut room = Room::new(self.width, self.height, cells)
            .map_err(|source| RoomError::Refused { map, source })?
            .with_material_policy(qualified_policy(map, self.width, self.height))
            .map_err(|source| RoomError::Policy { map, source })?;
        if self.room.passive_directional_type8_special_bit_clear() {
            room = room.with_passive_directional_type8_special_bit_clear();
        } else if self.room.passive_directional_collision() {
            room = room.with_passive_directional_collision();
        }
        Ok(Self {
            room,
            attributes: self.attributes.clone(),
            ..*self
        })
    }

    /// The cell word a tile patch writes (`$8D:8DF8`): the tile's attribute,
    /// low seven bits, above the tile number.
    #[must_use]
    pub fn patch_word(&self, tile: u16) -> u16 {
        let tile = tile & 0x1FF;
        let attribute = self.attributes.get(usize::from(tile)).copied().unwrap_or(0);
        u16::from(attribute & 0x7F) << 9 | tile
    }

    /// Cells the player can begin a step from.
    ///
    /// Searched rather than derived, because a cell's material is only half the
    /// question: the core also decides whether a step out of it is admitted.
    #[must_use]
    pub fn walkable_cells(&self) -> usize {
        let mut count = 0;
        for row in 0..self.height {
            for column in 0..self.width {
                if self.is_standable(column, row) {
                    count += 1;
                }
            }
        }
        count
    }

    /// Labels each standable cell with the connected region it belongs to.
    ///
    /// Four-neighbour connectivity over cells the core admits a step from. See
    /// [`Regions`] for why this matters: a map is one region of a shared layer.
    #[must_use]
    pub fn regions(&self) -> Regions {
        let (width, height) = (usize::from(self.width), usize::from(self.height));
        let open: Vec<bool> = (0..height)
            .flat_map(|row| (0..width).map(move |column| (column, row)))
            .map(|(column, row)| {
                self.is_standable(
                    u16::try_from(column).unwrap_or(u16::MAX),
                    u16::try_from(row).unwrap_or(u16::MAX),
                )
            })
            .collect();
        let mut labels: Vec<Option<u16>> = vec![None; width * height];
        let mut count = 0u16;
        for start in 0..width * height {
            if !open[start] || labels[start].is_some() {
                continue;
            }
            let mut queue = std::collections::VecDeque::from([start]);
            labels[start] = Some(count);
            while let Some(index) = queue.pop_front() {
                let (column, row) = (index % width, index / width);
                for (next_column, next_row) in [
                    (column.wrapping_sub(1), row),
                    (column + 1, row),
                    (column, row.wrapping_sub(1)),
                    (column, row + 1),
                ] {
                    if next_column >= width || next_row >= height {
                        continue;
                    }
                    let neighbour = next_row * width + next_column;
                    if open[neighbour] && labels[neighbour].is_none() {
                        labels[neighbour] = Some(count);
                        queue.push_back(neighbour);
                    }
                }
            }
            count += 1;
        }
        Regions {
            labels,
            count,
            width: self.width,
        }
    }

    /// Whether the player can stand on a cell and step out of it.
    #[must_use]
    pub fn is_standable(&self, column: u16, row: u16) -> bool {
        let (x, y) = (column * 16 + 8, row * 16 + 8);
        [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ]
        .into_iter()
        .any(|direction| {
            let mut state = WalkingState::new(x, y);
            (0..24).any(|_| {
                state
                    .step(
                        &self.room,
                        FrameInput {
                            direction: Some(direction),
                        },
                    )
                    .is_ok_and(|output| (output.x, output.y) != (x, y))
            })
        })
    }
}

/// Connected walkable regions of a grid, labelled per cell.
///
/// Several maps share one cell grid: the six rooms of the opening house are one
/// 32x64 layer, and the six southern town houses are one 48x32 layer. A map is
/// a **region** of its layer, not a layer of its own, and the regions are
/// separated by collision, so the player cannot walk from one map's room into
/// another's. Which region belongs to which map is ROM data rather than a
/// judgement: it is where that map's arrivals land, and for the `$1A` layer the
/// six maps and six regions are a bijection.
#[derive(Debug, Clone)]
pub struct Regions {
    labels: Vec<Option<u16>>,
    count: u16,
    width: u16,
}

impl Regions {
    /// Number of distinct walkable regions.
    #[must_use]
    pub const fn count(&self) -> u16 {
        self.count
    }
    /// Region a cell belongs to, or `None` when it is not standable.
    #[must_use]
    pub fn at(&self, column: u16, row: u16) -> Option<u16> {
        self.labels
            .get(usize::from(row) * usize::from(self.width) + usize::from(column))
            .copied()
            .flatten()
    }
}

/// A map that could not be made into a room.
#[derive(Debug)]
pub enum RoomError {
    /// The map is outside the Crysta slice.
    OutsideSlice {
        /// Map identifier asked for.
        map: u16,
    },
    /// The map's background or attribute table did not decode.
    Background {
        /// Map identifier.
        map: u16,
        /// Underlying decode failure.
        source: VisualMapError,
    },
    /// The attribute table was not the expected 512 bytes.
    Attributes {
        /// Map identifier.
        map: u16,
        /// Length actually decoded.
        length: usize,
    },
    /// `room-core` refused the cell grid.
    ///
    /// Deliberately fatal. A room built from a map whose cells were not all
    /// classified would let the player walk through what the data calls solid.
    Refused {
        /// Map identifier.
        map: u16,
        /// The core's reason.
        source: Unqualified,
    },
    /// `room-core` refused the material policy.
    Policy {
        /// Map identifier.
        map: u16,
        /// The core's reason.
        source: MaterialPolicyError,
    },
}

impl fmt::Display for RoomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutsideSlice { map } => write!(f, "map {map:#06x} is outside the Crysta slice"),
            Self::Background { map, source } => write!(f, "map {map:#06x} background: {source}"),
            Self::Attributes { map, length } => {
                write!(
                    f,
                    "map {map:#06x} attribute table is {length} bytes, not 512"
                )
            }
            Self::Refused { map, source } => write!(f, "map {map:#06x} refused: {source:?}"),
            Self::Policy { map, source } => write!(f, "map {map:#06x} policy: {source}"),
        }
    }
}

impl std::error::Error for RoomError {}

/// Builds the walkable room for one Crysta map.
///
/// # Errors
/// Refuses a map outside the slice, a background that does not decode, and a
/// grid or policy `room-core` will not accept.
pub fn room(image: &[u8], map: u16) -> Result<MapRoom, RoomError> {
    if !MAPS.contains(&map) {
        return Err(RoomError::OutsideSlice { map });
    }
    let background = StaticBackground::from_rom(image, map)
        .map_err(|source| RoomError::Background { map, source })?;
    let decoded = background.resources()[3].decoded();
    let attributes: &[u8; 512] = decoded.try_into().map_err(|_| RoomError::Attributes {
        map,
        length: decoded.len(),
    })?;
    let cells: Vec<u16> = background
        .layer()
        .attributed_cells(attributes)
        .iter()
        .map(|cell| cell.raw())
        .collect();
    let width = u16::try_from(background.layer().width()).unwrap_or(u16::MAX);
    let height = u16::try_from(background.layer().height()).unwrap_or(u16::MAX);
    let built = Room::new(width, height, cells)
        .map_err(|source| RoomError::Refused { map, source })?
        .with_material_policy(qualified_policy(map, width, height))
        .map_err(|source| RoomError::Policy { map, source })?;
    Ok(MapRoom {
        room: built,
        map,
        width,
        height,
        attributes: attributes.to_vec(),
    })
}

/// Builds an opt-in passive directional candidate with `$097C & 4 == 0`.
///
/// This is for route discovery, not production enablement. [`room`] remains
/// conservative; the raw cells and qualified material policy are unchanged.
/// The caller asserts the full
/// [`Room::with_passive_directional_type8_special_bit_clear`] contract: the
/// ordinary special-player resolver, fixed (-8,-16) offsets and 16×16 bounds,
/// inactive action hooks (`$0980 & $0050 == 0`), and `$097C & $0004 == 0`.
/// Stop using this candidate if that contract changes. This is collision
/// geometry only, not native interaction qualification.
///
/// # Errors
/// As [`room`].
pub fn room_candidate(image: &[u8], map: u16) -> Result<MapRoom, RoomError> {
    let mut built = room(image, map)?;
    built.room = built
        .room
        .with_passive_directional_type8_special_bit_clear();
    Ok(built)
}

/// Classifications `room-core` already qualified, scoped to their own cells.
///
/// Its default table leaves attributes 5, 25 and 29 undecided, but it carries
/// aliases for them that a room must opt into. Installing them is what lets the
/// core agree with the measured town band and traverse the stairs instead of
/// refusing. The alias scopes are validated by `room-core`, so a wrong cell is
/// rejected rather than silently accepted.
#[must_use]
pub fn qualified_policy(map: u16, width: u16, height: u16) -> Vec<MaterialRule> {
    match map {
        0x000A => vec![MaterialRule {
            bounds: [0, 0, width, height],
            direction: None,
            alias: MaterialAlias::TownSolid25,
        }],
        0x000C => vec![
            MaterialRule {
                bounds: [11, 21, 12, 22],
                direction: Some(Direction::Up),
                alias: MaterialAlias::ClosedDoorPartial5,
            },
            MaterialRule {
                bounds: [11, 21, 12, 22],
                direction: Some(Direction::Up),
                alias: MaterialAlias::StairOpen29,
            },
        ],
        0x000E => vec![MaterialRule {
            bounds: [6, 53, 7, 54],
            direction: Some(Direction::Up),
            alias: MaterialAlias::StairOpen29,
        }],
        0x0020 => vec![MaterialRule {
            bounds: [22, 53, 23, 54],
            direction: Some(Direction::Up),
            alias: MaterialAlias::StairOpen29,
        }],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_slice_spans_twenty_four_maps() {
        assert_eq!(MAPS.count(), 24);
        assert!(MAPS.contains(&0x000A) && MAPS.contains(&0x0021));
        assert!(!MAPS.contains(&0x0009) && !MAPS.contains(&0x0022));
    }

    #[test]
    fn only_the_qualified_maps_carry_rules() {
        // Every other map gets an empty policy, so adding a map is data rather
        // than a new behavioural branch.
        for map in MAPS {
            let rules = qualified_policy(map, 64, 80);
            let expected = match map {
                0x000A | 0x000E | 0x0020 => 1,
                0x000C => 2,
                _ => 0,
            };
            assert_eq!(rules.len(), expected, "map {map:#06x}");
        }
    }

    #[test]
    fn the_town_rule_covers_the_whole_grid() {
        let rules = qualified_policy(0x000A, 64, 80);
        assert_eq!(rules[0].bounds, [0, 0, 64, 80]);
        assert_eq!(rules[0].direction, None, "the band is not directional");
    }

    #[test]
    fn the_stair_rules_are_scoped_and_directional() {
        // A stair alias applies upward out of two specific cells. Scoping is
        // what keeps it from making every stair-attributed cell passable.
        for (map, bounds) in [(0x000Eu16, [6, 53, 7, 54]), (0x0020, [22, 53, 23, 54])] {
            let rules = qualified_policy(map, 64, 80);
            assert_eq!(rules[0].bounds, bounds);
            assert_eq!(rules[0].direction, Some(Direction::Up));
            assert_eq!(rules[0].alias, MaterialAlias::StairOpen29);
        }
    }

    #[test]
    fn a_map_outside_the_slice_is_refused_without_a_rom() {
        let error = room(&[], 0x0009).unwrap_err();
        assert!(matches!(error, RoomError::OutsideSlice { map: 0x0009 }));
        assert!(error.to_string().contains("outside the Crysta slice"));
    }
}
