//! Ark's spear thrust (`docs/combat.md`): A with a weapon equipped plays
//! resource 4's list for the facing (`$00` Down, `$01` Up, `$02` Right,
//! mirrored for Left) for 16 frames from the frame after the press, its
//! attack box live from the second.

use super::{Step, World, WorldError};
use assets::sprites::boxes::{self, Record};
use room_core::Direction;

/// Frames of a thrust: its records (4, 4, 2, 2, 2) and the last held 2 more.
const THRUST: u16 = 16;
/// The equipment table (`$8D:BC92`, European `$8D:BB5B`): four bytes an
/// item from `$80`, the power in the first word's low 10 bits.
const EQUIPMENT: usize = 0x0D_BC92;

/// A thrust under way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Thrust {
    facing: Direction,
    /// Frames shown; `None` on the press's own frame.
    age: Option<u16>,
}

/// The thrust's records for each list (Down, Up, Right).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct ThrustRecords([Vec<Record>; 3]);

impl ThrustRecords {
    pub(super) fn from_rom(image: &[u8]) -> Self {
        Self(std::array::from_fn(|list| {
            boxes::ark_list(image, 4, u8::try_from(list).unwrap_or(0)).unwrap_or_default()
        }))
    }
}

/// The list for a facing, and whether it is mirrored.
const fn list(facing: Direction) -> (u8, bool) {
    match facing {
        Direction::Down => (0, false),
        Direction::Up => (1, false),
        Direction::Right => (2, false),
        Direction::Left => (2, true),
    }
}

impl World<'_> {
    /// Equips `item` as the weapon, its power from the equipment table.
    pub fn equip_weapon(&mut self, item: u8) {
        let at = assets::layout::per_revision(self.image, EQUIPMENT, EQUIPMENT - 0x137)
            + usize::from(item.wrapping_sub(0x80)) * 4;
        let power = self
            .image
            .get(at..at + 2)
            .map_or(0, |word| u16::from_le_bytes([word[0], word[1]]) & 0x3FF);
        self.globals.slot.set_weapon(item, power);
    }

    /// Starts a thrust, when a weapon is equipped. Returns whether it did.
    pub(super) fn thrust(&mut self) -> bool {
        if self.globals.slot.weapon().is_none() || self.thrust.is_some() {
            return false;
        }
        self.thrust = Some(Thrust {
            facing: self.facing,
            age: None,
        });
        true
    }

    /// A frame of a thrust under way: Ark stands in it while the actors run.
    pub(super) fn thrust_frame(&mut self) -> Result<Option<Step>, WorldError> {
        let Some(mut thrust) = self.thrust.take() else {
            return Ok(None);
        };
        let age = thrust.age.map_or(0, |age| age + 1);
        if age < THRUST {
            thrust.age = Some(age);
            self.thrust = Some(thrust);
        }
        self.run_actors()?;
        Ok(Some(Step::Stayed))
    }

    /// Ark's thrust pose: resource 4's list, its age, and the mirror.
    #[must_use]
    pub fn attack_pose(&self) -> Option<(u8, u16, bool)> {
        let thrust = self.thrust?;
        let (list, mirrored) = list(thrust.facing);
        Some((list, thrust.age?, mirrored))
    }

    /// The thrust's attack box on the map (left, top, right, bottom), from
    /// its second frame on.
    #[must_use]
    pub fn attack_box(&self) -> Option<(i32, i32, i32, i32)> {
        let thrust = self.thrust?;
        let age = thrust.age.filter(|&age| age >= 1)?;
        let (list, mirrored) = list(thrust.facing);
        let records = &self.thrust_records.0[usize::from(list)];
        let mut left = age;
        let record = records
            .iter()
            .find(|record| {
                let length = u16::from(record.duration) + 1;
                let inside = left < length;
                left = left.saturating_sub(length);
                inside
            })
            .or_else(|| records.last())?;
        let [dx, width, dy, height] = record.attack.map(i32::from);
        let (x, y) = (i32::from(self.position().0), i32::from(self.position().1));
        let left = if mirrored { x - dx - width } else { x + dx };
        Some((left, y + dy, left + width, y + dy + height))
    }
}
