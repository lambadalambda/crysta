//! Ark's spear thrust (`docs/combat.md`): A with a weapon equipped plays
//! resource 4's list for the facing (`$00` Down, `$01` Up, `$02` Right,
//! mirrored for Left) for 16 frames from the frame after the press, its
//! attack box live from the second; and the hit scan on enemies.

use super::{Step, World, WorldError};
use crate::combat;
use crate::scene::{DigitKind, Digits};
use assets::sprites::boxes::{self, Record, Rect};
use room_core::Direction;

/// Frames of a thrust: its records (4, 4, 2, 2, 2) and the last held 2 more.
const THRUST: u16 = 16;
/// The equipment table (`$8D:BC92`, European `$8D:BB5B`): four bytes an
/// item from `$80`, the power in the first word's low 10 bits.
const EQUIPMENT: usize = 0x0D_BC92;
/// The sound of a hit (`$85:D4C8`).
const HIT_SOUND: u8 = 0x09;

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

    /// Equips `item` as the armor (`$064C`).
    pub fn equip_armor(&mut self, item: u8) {
        self.globals.slot.set_armor(item);
    }

    /// The armor Ark wears (`$064C`), if any.
    #[must_use]
    pub fn armor(&self) -> Option<u8> {
        self.globals.slot.armor()
    }

    /// A frame Ark's own state holds him: carrying a pot, down, pushed or
    /// thrusting; the enemies' hit scan on him follows.
    pub(super) fn held_frame(
        &mut self,
        direction: Option<Direction>,
        lift: bool,
    ) -> Result<Option<Step>, WorldError> {
        let step = match self.pot_frame(direction, lift)? {
            Some(step) => Some(step),
            None => match self.down_frame()? {
                Some(step) => Some(step),
                None => match self.hurt_frame()? {
                    Some(step) => Some(step),
                    None => self.thrust_frame()?,
                },
            },
        };
        if step.is_some() {
            self.hurt_ark();
        }
        Ok(step)
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
    fn thrust_frame(&mut self) -> Result<Option<Step>, WorldError> {
        let Some(mut thrust) = self.thrust.take() else {
            return Ok(None);
        };
        let age = thrust.age.map_or(0, |age| age + 1);
        if age < THRUST {
            thrust.age = Some(age);
            self.thrust = Some(thrust);
        }
        self.run_actors()?;
        self.strike_foes();
        Ok(Some(Step::Stayed))
    }

    /// The hit scan (`$85:D281`): the thrust's attack box against each
    /// enemy's body box, edges included; a hit that does damage takes it
    /// and pushes the enemy away from Ark (a 0 is only a strike); one that
    /// takes no damage (`+$06 & $0020`) is passed over.
    fn strike_foes(&mut self) {
        let Some(attack) = self.attack_box() else {
            return;
        };
        let stats = self.globals.slot.stats();
        let at = self.position();
        for actor in &mut self.actors {
            let (Some(body), Some(profile)) =
                (actor.body_box(), actor.foe.as_ref().map(|foe| foe.profile))
            else {
                continue;
            };
            if !boxes::overlap(attack, body) || actor.unharmed() {
                continue;
            }
            self.globals.random.step();
            let roll = combat::Roll {
                critical: self.globals.random.word().to_le_bytes()[0] & 0x7F,
                counter: self.globals.frames,
            };
            let damage = combat::ark_damage(&stats, combat::Kind::Thrust, &profile, roll);
            if damage.amount == 0 {
                continue;
            }
            self.globals.audio.sound_port3(HIT_SOUND);
            actor.take_hit(damage.amount, away(at, actor.position), self.image);
            self.globals.digits.push(Digits {
                at: (actor.position.0, u16::try_from(body.1.max(0)).unwrap_or(0)),
                amount: damage.amount,
                kind: if damage.critical {
                    DigitKind::Critical
                } else {
                    DigitKind::Normal
                },
                age: 0,
            });
        }
    }

    /// Ark's thrust pose: resource 4's list, its age, and the mirror.
    #[must_use]
    pub fn attack_pose(&self) -> Option<(u8, u16, bool)> {
        let thrust = self.thrust?;
        let (list, mirrored) = list(thrust.facing);
        Some((list, thrust.age?, mirrored))
    }

    /// The thrust's attack box on the map, from its second frame on.
    #[must_use]
    pub fn attack_box(&self) -> Option<Rect> {
        let thrust = self.thrust?;
        let age = thrust.age.filter(|&age| age >= 1)?;
        let (list, mirrored) = list(thrust.facing);
        let record = boxes::at_age(&self.thrust_records.0[usize::from(list)], u32::from(age))?;
        Some(boxes::place(record.attack, self.position(), mirrored))
    }
}

/// The way a hit pushes its target (`$85:F8D1`): away from the attacker,
/// along the larger offset.
pub(super) fn away(attacker: (u16, u16), target: (u16, u16)) -> Direction {
    let dx = i32::from(target.0) - i32::from(attacker.0);
    let dy = i32::from(target.1) - i32::from(attacker.1);
    match (dx.abs() > dy.abs(), dx < 0, dy < 0) {
        (true, true, _) => Direction::Left,
        (true, false, _) => Direction::Right,
        (false, _, true) => Direction::Up,
        (false, _, false) => Direction::Down,
    }
}
