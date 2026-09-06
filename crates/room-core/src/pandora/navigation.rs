//! Finite ordered exit admission and source-backed Town cell operands.
use super::{Anchor, CollisionKey, MotionKey, MotionPose, PandoraData, Travel};
use crate::{
    slice::{select_exit, Exit, SliceError},
    Direction,
};
use alloc::vec::Vec;

/// Source map and ordinal in its complete ordered exit list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExitKey {
    /// Source map ID (not destination lookup).
    pub map_id: u16,
    /// Zero-based source record ordinal.
    pub index: u16,
}
/// Complete source list, including unsupported records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapExits {
    /// Source map ID.
    pub map_id: u16,
    /// Ordered decoded twelve-byte source records; no pointer is executed.
    pub records: Vec<Exit>,
}
/// Exact source selection qualified by an immutable travel motion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TravelExit {
    /// Finite route.
    pub travel: Travel,
    /// Exact source record.
    pub exit: ExitKey,
}
/// Independently opened wooden doors on the Town sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TownDoor {
    /// North resident door; mask bit0.
    North,
    /// Home door to D; mask bit1.
    Home,
}
impl TownDoor {
    /// Bit in the read-only Town open mask.
    #[must_use]
    pub const fn mask(self) -> u8 {
        1 << self as u8
    }
}
/// Source tile/attribute replacement, preserving scene occupancy bit15.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellPatch {
    /// Row-major cell index.
    pub cell: u16,
    /// Closed source word, without occupancy.
    pub closed: u16,
    /// Open source operand, without occupancy.
    pub open: u16,
}
/// Compiler-authenticated interaction witness and two-cell door operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TownDoorSpec {
    /// Door identity.
    pub door: TownDoor,
    /// Source exit guarded by this door.
    pub exit: ExitKey,
    /// Exact source-qualified approach and facing.
    pub interaction: Anchor,
    /// Upper then lower source cells.
    pub patches: [CellPatch; 2],
}
/// Immutable bounded navigation extension; validate with `PandoraData::with_navigation`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationSpec {
    /// Complete lists for A,13,C,D,E,20, in that order.
    pub maps: Vec<MapExits>,
    /// Exactly one binding for each of the six travel variants, in any order.
    pub travels: Vec<TravelExit>,
    /// North then Home.
    pub doors: [TownDoorSpec; 2],
}
impl NavigationSpec {
    pub(crate) fn records(&self, map: u16) -> Option<&[Exit]> {
        self.maps
            .iter()
            .find(|m| m.map_id == map)
            .map(|m| m.records.as_slice())
    }
    pub(crate) fn record(&self, key: ExitKey) -> Option<Exit> {
        self.records(key.map_id)?
            .get(usize::from(key.index))
            .copied()
    }
    #[cfg(test)]
    pub(crate) fn door(&self, door: TownDoor) -> &TownDoorSpec {
        &self.doors[door as usize]
    }
    pub(crate) fn binding(&self, travel: Travel) -> Option<ExitKey> {
        self.travels
            .iter()
            .find(|t| t.travel == travel)
            .map(|t| t.exit)
    }
    pub(crate) fn selected(&self, key: ExitKey, anchor: Anchor) -> bool {
        self.records(key.map_id)
            .and_then(|r| select_exit(r, anchor.position))
            .is_some_and(|(i, _)| i == usize::from(key.index))
    }
    pub(crate) fn validate(&self, data: &PandoraData) -> Result<(), SliceError> {
        if self
            .maps
            .iter()
            .map(|m| m.map_id)
            .ne([0xa, 0x13, 0xc, 0xd, 0xe, 0x20])
            || self.travels.len() != 6
        {
            return Err(SliceError::Data);
        }
        for map in &self.maps {
            let (width, height) = match map.map_id {
                0xa => (64, 80),
                0x13 => (64, 32),
                _ => (32, 64),
            };
            if map.records.is_empty()
                || map.records.len() > 64
                || map.records.iter().any(|e| {
                    let [x, y, w, h, ..] = e.0;
                    // Source exit rectangles are predicates, not grid slices:
                    // Town $818DB3 intentionally extends beyond the sheet width.
                    // Keep bounded origins/nonempty byte dimensions; the shared
                    // selector retains native wrapping and never indexes this extent.
                    w == 0 || h == 0 || u16::from(x) >= width || u16::from(y) >= height
                })
            {
                return Err(SliceError::Data);
            }
        }
        for (i, binding) in self.travels.iter().enumerate() {
            if self.travels[..i]
                .iter()
                .any(|b| b.travel == binding.travel || b.exit == binding.exit)
            {
                return Err(SliceError::Data);
            }
            let (source, destination) = binding.travel.maps();
            let exit = self.record(binding.exit).ok_or(SliceError::Data)?;
            let (_, motion) = data
                .motion(MotionKey::Travel(binding.travel))
                .ok_or(SliceError::Data)?;
            let anchor = motion.trigger.ok_or(SliceError::Data)?;
            if binding.exit.map_id != source
                || u16::from_le_bytes([exit.0[4], exit.0[5]]) != destination
                || exit.0[6] != 0
                || !self.selected(binding.exit, anchor)
            {
                return Err(SliceError::Data);
            }
            if matches!(
                binding.travel,
                Travel::TownToResident | Travel::ResidentToTown | Travel::TownToHouse
            ) {
                validate_ordinary(motion, exit)?;
            } else if exit.0[7] != 14 || anchor.facing != Direction::Up {
                return Err(SliceError::Data);
            }
        }
        for (spec, (door, travel)) in self.doors.iter().zip([
            (TownDoor::North, Travel::TownToResident),
            (TownDoor::Home, Travel::TownToHouse),
        ]) {
            let (_, motion) = data
                .motion(MotionKey::Travel(travel))
                .ok_or(SliceError::Data)?;
            let trigger = motion.trigger.ok_or(SliceError::Data)?;
            let [upper, lower] = spec.patches;
            let room = data.room(CollisionKey::Town);
            if spec.door != door
                || Some(spec.exit) != self.binding(travel)
                || spec.interaction.facing != Direction::Up
                || trigger.facing != Direction::Up
                || spec.interaction.position
                    != (
                        trigger.position.0,
                        trigger.position.1.checked_add(16).ok_or(SliceError::Data)?,
                    )
                || upper.cell.checked_add(64) != Some(lower.cell)
                || upper.cell % 64 != lower.cell % 64
                || (upper.closed, lower.closed, upper.open, lower.open)
                    != (0x1cf2, 0x1cf3, 0x1cf6, 0x00f7)
                || trigger.position != ((lower.cell % 64) * 16 + 8, (lower.cell / 64) * 16 + 16)
                || spec.patches.iter().any(|p| {
                    room.cells().get(usize::from(p.cell)).map(|w| w & 0x7fff) != Some(p.closed)
                })
                || room
                    .validate_position(spec.interaction.position.0, spec.interaction.position.1)
                    .is_err()
            {
                return Err(SliceError::Data);
            }
        }
        if self.doors[0]
            .patches
            .iter()
            .any(|a| self.doors[1].patches.iter().any(|b| a.cell == b.cell))
        {
            return Err(SliceError::Data);
        }
        Ok(())
    }
}
fn validate_ordinary(motion: &super::MotionSpec, exit: Exit) -> Result<(), SliceError> {
    let direction = match exit.0[7] {
        5 => Direction::Down,
        6 => Direction::Up,
        _ => return Err(SliceError::Data),
    };
    let raw = (
        u16::from_le_bytes([exit.0[8], exit.0[9]]),
        u16::from_le_bytes([exit.0[10], exit.0[11]]),
    );
    let loaded = (
        raw.0.checked_add(8).ok_or(SliceError::Data)?,
        raw.1
            .checked_add(if direction == Direction::Up { 32 } else { 0 })
            .ok_or(SliceError::Data)?,
    );
    let trigger = motion.trigger.ok_or(SliceError::Data)?;
    if trigger.facing != direction || motion.frames.len() != 35 {
        return Err(SliceError::Data);
    }
    for (i, frame) in motion.frames.iter().enumerate() {
        let elapsed = u8::try_from(i + 1).map_err(|_| SliceError::Data)?;
        let position =
            crate::transition::doorway_position(trigger.position, loaded, direction, elapsed)
                .ok_or(SliceError::Data)?;
        if frame.reload != (elapsed == 18)
            || frame.map_id
                != if elapsed <= 17 {
                    motion.key.maps().0
                } else {
                    motion.key.maps().1
                }
            || frame.pose
                != MotionPose::Absolute(Anchor {
                    position,
                    facing: direction,
                })
        {
            return Err(SliceError::Data);
        }
    }
    Ok(())
}
impl PandoraData {
    /// Attach one compiler-authenticated navigation contract without changing `new` arity.
    /// The enclosing aggregate identity must hash every list, binding, patch and motion.
    /// # Errors
    /// Rejects duplicate attachment, missing routes, inconsistent source selection,
    /// malformed door operands or ordinary motion clocks other than 17/load/17.
    pub fn with_navigation(mut self, navigation: NavigationSpec) -> Result<Self, SliceError> {
        if self.navigation.is_some() {
            return Err(SliceError::Data);
        }
        navigation.validate(&self)?;
        self.navigation = Some(navigation);
        Ok(self)
    }
}
