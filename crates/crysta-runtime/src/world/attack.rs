//! Ark's spear thrust (`docs/combat.md`): A with a weapon equipped plays
//! resource 4's list for the facing (`$00` Down, `$01` Up, `$02` Right,
//! mirrored for Left) for 16 frames from the frame after the press, its
//! attack box live from the second; the dash attack, A in a dash; and the
//! hit scan on enemies.

use super::pose::ArkPose;
use super::{Step, World, WorldError};
use crate::combat;
use crate::scene::{DigitKind, Digits, Presses};
use assets::sprites::boxes::{self, Record, Rect};
use room_core::Direction;

/// Frames of a thrust: its records (4, 4, 2, 2, 2) and the last held 2 more.
const THRUST: u16 = 16;
/// The equipment table (`$8D:BC92`, European `$8D:BB5B`): four bytes an
/// item from `$80`, the power in the first word's low 10 bits.
const EQUIPMENT: usize = 0x0D_BC92;
/// The sound of a hit (`$85:D4C8`).
const HIT_SOUND: u8 = 0x09;
/// The dash attack (`$84:9549`.., native traces `g_jp_*`): from the press,
/// 18 frames of resource 4's list at 3 pixels a frame, then 7 of resource
/// 3's sliding these, then a standing frame; attacking to the slide's
/// first frame, sound `$06` on the second.
const DASH_ATTACK: u16 = 18;
const DASH_STRIDE: i16 = 3;
const DASH_SLIDE: [i16; 7] = [1, 2, 1, 2, 1, 2, 1];
const DASH_SOUND: u8 = 0x06;

/// A thrust under way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Thrust {
    facing: Direction,
    /// Frames shown; `None` on the press's own frame.
    age: Option<u16>,
}

/// A dash attack under way: its age from the press, `None` until its
/// first frame runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DashAttack {
    facing: Direction,
    age: Option<u16>,
}

/// Where a dash attack is at an age.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// Resource 4's list, at 3 pixels a frame; the age in it.
    Attack(u16),
    /// Resource 3's slide; the age in it.
    Slide(u16),
    /// The standing frame.
    Stand,
}

impl Phase {
    /// The phase at `age`, `None` past the standing frame.
    fn at(age: u16) -> Option<Self> {
        let Some(slide) = age.checked_sub(DASH_ATTACK) else {
            return Some(Self::Attack(age));
        };
        match usize::from(slide).cmp(&DASH_SLIDE.len()) {
            std::cmp::Ordering::Less => Some(Self::Slide(slide)),
            std::cmp::Ordering::Equal => Some(Self::Stand),
            std::cmp::Ordering::Greater => None,
        }
    }

    /// The pixels moved.
    fn stride(self) -> i16 {
        match self {
            Self::Attack(_) => DASH_STRIDE,
            Self::Slide(age) => DASH_SLIDE[usize::from(age)],
            Self::Stand => 0,
        }
    }
}

impl DashAttack {
    fn phase(self) -> Phase {
        Phase::at(self.age.unwrap_or(0)).unwrap_or(Phase::Stand)
    }

    /// The resource and list shown, and the frames into it.
    fn shown(self) -> (u8, u8, u16) {
        let (by, _) = list(self.facing);
        match self.phase() {
            Phase::Attack(age) => (4, 0x0C + by, age),
            Phase::Slide(age) => (3, 0x36 + by, age),
            Phase::Stand => (0, by, 0),
        }
    }

    /// Ark's pose in it.
    pub(super) fn pose(self) -> ArkPose {
        let (resource, shown, age) = self.shown();
        ArkPose::new(resource, shown, list(self.facing).1, age, true)
    }

    /// The live attack's records and the age in them: the attack's, and
    /// the slide's first frame.
    fn records(self, records: &AttackRecords) -> Option<(&[Vec<Record>; 3], u16)> {
        match self.phase() {
            Phase::Attack(age) => Some((&records.dash, age)),
            Phase::Slide(0) => Some((&records.slide, 0)),
            _ => None,
        }
    }
}

/// The attacks' records, each for Down, Up, Right: the thrust's, the dash
/// attack's and its slide's.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct AttackRecords {
    thrust: [Vec<Record>; 3],
    dash: [Vec<Record>; 3],
    slide: [Vec<Record>; 3],
}

impl AttackRecords {
    pub(super) fn from_rom(image: &[u8]) -> Self {
        let lists = |resource: u8, first: u8| {
            std::array::from_fn(|list| {
                let list = first + u8::try_from(list).unwrap_or(0);
                boxes::ark_list(image, resource, list).unwrap_or_default()
            })
        };
        Self {
            thrust: lists(4, 0),
            dash: lists(4, 0x0C),
            slide: lists(3, 0x36),
        }
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
                    None => match self.thrust_frame()? {
                        Some(step) => Some(step),
                        None => self.dash_attack_frame(direction)?,
                    },
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

    /// Whether the dash takes this frame's A from talking, doors, lifts and
    /// thrusts: in it, on its setup frame (`e_a_setup2`), or on the double
    /// tap's own frame, where the dash wins (`e_a_setup`).
    pub(super) fn dash_takes_a(&self, direction: Option<Direction>) -> bool {
        let tap = room_core::PadInput {
            direction,
            cross: direction.and(self.cross),
        };
        matches!(
            self.run,
            Some(room_core::run::Run::Dash { .. } | room_core::run::Run::Tap { .. })
        ) || (self.run.is_none() && !self.holds_pot() && self.walking.double_taps(tap))
    }

    /// A in a held dash attacks from the next frame, if the dash is still
    /// held: the loop reads the frame before (`$84:9078`, `$84:9549`). B
    /// first.
    pub(super) fn arm_dash_attack(&mut self, presses: Presses, free: bool) {
        let held = |run: Option<room_core::run::Run>| run.is_some_and(room_core::run::Run::held);
        if self.dash_attack_armed && free && held(self.run) {
            self.dash_attack();
        }
        // A on the setup frame counts too (native `e_a_setup2`).
        let setup = matches!(self.run, Some(room_core::run::Run::Tap { .. }));
        self.dash_attack_armed =
            presses.confirm && !presses.cancel && free && (setup || held(self.run));
    }

    /// Starts the dash attack: A in a dash with a weapon equipped.
    pub(super) fn dash_attack(&mut self) {
        if self.globals.slot.weapon().is_some() {
            self.dash_attack = Some(DashAttack {
                facing: self.facing,
                age: None,
            });
            self.run = None;
        }
    }

    /// A frame of the dash attack: Ark moves by its stride while the
    /// actors run; past its standing frame the ordinary frame goes on.
    fn dash_attack_frame(
        &mut self,
        direction: Option<Direction>,
    ) -> Result<Option<Step>, WorldError> {
        let Some(mut attack) = self.dash_attack.take() else {
            return Ok(None);
        };
        let age = attack.age.map_or(0, |age| age + 1);
        let Some(phase) = Phase::at(age) else {
            return Ok(None);
        };
        attack.age = Some(age);
        if age == 1 {
            self.globals.audio.sound_port3(DASH_SOUND);
        }
        // A refused move leaves Ark where he is.
        let stride = phase.stride();
        let _ = room_core::run::push(&mut self.walking, &self.room.room, attack.facing, stride);
        if matches!(phase, Phase::Stand) {
            // The standing frame is the walk's input frame (`g_jp_*`).
            let input = room_core::PadInput {
                direction,
                cross: self.cross,
            };
            let _ = self.walking.step(&self.room.room, input);
        }
        if let Some(actor) = &mut self.player_actor {
            actor.position = self.walking.position();
        }
        self.dash_attack = Some(attack);
        // As a dash: an exit takes him, a pit or a lip ends the attack.
        if let Some(step) = self.take_exit()? {
            self.dash_attack = None;
            return Ok(Some(step));
        }
        self.ground_test();
        self.run_actors()?;
        self.strike_foes();
        Ok(Some(if stride == 0 {
            Step::Stayed
        } else {
            Step::Walked
        }))
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
        let Some(attack) = self.attack_box().filter(|_| !self.hits_off()) else {
            return;
        };
        let kind = if self.dash_attack.is_some() {
            combat::Kind::Dash
        } else {
            combat::Kind::Thrust
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
            let damage = combat::ark_damage(&stats, kind, &profile, roll);
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

    /// The attack box on the map: the thrust's from its second frame on,
    /// the dash attack's to its slide's first frame.
    #[must_use]
    pub fn attack_box(&self) -> Option<Rect> {
        let (records, facing, age) = if let Some(thrust) = self.thrust {
            let age = thrust.age.filter(|&age| age >= 1)?;
            (&self.attack_records.thrust, thrust.facing, age)
        } else {
            let attack = self.dash_attack?;
            let (records, age) = attack.records(&self.attack_records)?;
            (records, attack.facing, age)
        };
        let (list, mirrored) = list(facing);
        let record = boxes::at_age(&records[usize::from(list)], u32::from(age))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dash_attack_runs_then_slides_then_stands() {
        let strides: Vec<Option<i16>> = (0..27)
            .map(|age| Phase::at(age).map(Phase::stride))
            .collect();
        assert_eq!(strides[..18], [Some(3); 18]);
        assert_eq!(
            strides[18..],
            [1, 2, 1, 2, 1, 2, 1, 0]
                .map(Some)
                .into_iter()
                .chain([None])
                .collect::<Vec<_>>()[..]
        );
        let shown = |age| {
            DashAttack {
                facing: Direction::Left,
                age: Some(age),
            }
            .shown()
        };
        assert_eq!(
            [shown(0), shown(17), shown(18), shown(24), shown(25)],
            [
                (4, 0x0E, 0),
                (4, 0x0E, 17),
                (3, 0x38, 0),
                (3, 0x38, 6),
                (0, 2, 0)
            ]
        );
    }
}
