//! An enemy's side of combat (`docs/combat.md`, `docs/enemy-scripts.md`):
//! its profile and life, the boxes of its poses, a hit's immunity and
//! knockback (`$85:E03D`), and its death.

use super::{motion, Actor, State};
use crate::combat::Profile;
use assets::sprites::boxes::{self, Record, Rect};
use room_core::Direction;
use std::rc::Rc;

/// Frames an enemy cannot be hit again after a hit (`7F:1020`).
const IMMUNE: u16 = 50;
/// Frames of the explosion (pose `$16`: 13 records of 2 frames).
const EXPLOSION: u16 = 26;
/// The explosion's and the gem's lists of the helper art.
const EXPLODING: u8 = 0x16;
/// A dropped gem: shown, then free to take, then blinking (`$85:E3CF`).
const GEM_SHOWN: u16 = 16;
const GEM_BLINKS: u16 = 256;
const GEM_GONE: u16 = 288;

/// `+$04` bits that keep a body out of the hit scan (`$85:D2CB`) as a
/// target, and as an attacker (`$85:D2A7`).
const NOT_TARGET: u16 = 0x0022;
const NOT_ATTACKING: u16 = 0x0010;
/// `+$06` bits: no knockback, no damage.
const NO_KNOCKBACK: u16 = 0x0010;
const NO_DAMAGE: u16 = 0x0020;

/// The helper art (`$A2:C000`, European `$A4:C000`).
pub(crate) fn helper(image: &[u8]) -> u32 {
    assets::layout::per_revision(image, 0xA2_C000, 0xA4_C000)
}

/// A gem an enemy dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Gem {
    pub(crate) amount: u16,
    pub(crate) age: u16,
}

impl Gem {
    /// Whether Ark may take it now.
    pub(crate) const fn takeable(self) -> bool {
        self.age >= GEM_SHOWN && self.age < GEM_GONE
    }
}

/// An enemy's combat state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Foe {
    pub(crate) profile: Profile,
    pub(crate) life: u16,
    pub(crate) immune: u16,
    /// Pushed by a hit; the script waits.
    pub(crate) knocked: bool,
    /// Frames of the explosion left, once dead.
    pub(crate) exploding: Option<u16>,
    /// Gone after its death.
    pub(crate) dead: bool,
    /// The gem it left, until taken or gone.
    pub(crate) gem: Option<Gem>,
    /// Whether it counts in `$0498`, the enemies a room waits on.
    pub(crate) counted: bool,
}

impl Foe {
    pub(crate) const fn new(profile: Profile, counted: bool) -> Self {
        Self {
            counted,
            life: profile.life,
            profile,
            immune: 0,
            knocked: false,
            exploding: None,
            dead: false,
            gem: None,
        }
    }

    /// Whether it still counts in `$0498`: alive, or not yet past the death
    /// check at its push's end.
    pub(crate) const fn alive(&self) -> bool {
        self.counted && (self.life > 0 || self.knocked)
    }

    /// Whether a hit may land now.
    pub(crate) const fn hittable(&self) -> bool {
        self.immune == 0 && !self.knocked && self.exploding.is_none() && !self.dead
    }
}

impl Actor {
    /// The record the pose shows now, if its boxes are known.
    fn record(&self) -> Option<&Record> {
        let records = self.boxes.as_ref()?.get(usize::from(self.selector))?;
        let total: u32 = records
            .iter()
            .map(|record| u32::from(record.duration) + 1)
            .sum();
        boxes::at_age(records, self.pose_age.checked_rem(total)?)
    }

    /// Entity `+$14`: the facing code of the record shown (`$80:EDA4`), a
    /// mirrored Right turned Left (`$80:EE05`); without boxes, the actor's
    /// facing.
    pub(crate) fn facing_code(&self) -> u8 {
        match self.record().map(|record| record.facing) {
            Some(3) if self.hflip => 2,
            Some(facing) => facing,
            None => super::sense::code(self.facing),
        }
    }

    /// A box of the pose shown, on the map.
    fn place(&self, shape: [i8; 4]) -> Rect {
        boxes::place(shape, self.position, self.hflip)
    }

    /// Whether it counts in `$0498`.
    pub(crate) fn counts(&self) -> bool {
        self.foe.as_ref().is_some_and(Foe::alive)
    }

    /// The body box an attack must touch, while the enemy can be hit.
    pub(crate) fn body_box(&self) -> Option<Rect> {
        self.foe.as_ref().filter(|foe| foe.hittable())?;
        if self.hidden || self.guard.0 & NOT_TARGET != 0 {
            return None;
        }
        Some(self.place(self.record()?.body))
    }

    /// The attack box that hurts Ark, while the enemy is up and about.
    pub(crate) fn hurt_box(&self) -> Option<Rect> {
        let foe = self.foe.as_ref()?;
        if self.hidden
            || self.guard.0 & NOT_ATTACKING != 0
            || foe.knocked
            || foe.exploding.is_some()
            || foe.dead
            || foe.gem.is_some()
        {
            return None;
        }
        Some(self.place(self.record()?.attack))
    }

    /// Whether a hit does it no damage (`+$06 & $0020`).
    pub(crate) const fn unharmed(&self) -> bool {
        self.guard.1 & NO_DAMAGE != 0
    }

    /// Takes `damage` from a hit that pushes toward `away` (`$85:D578`,
    /// knockback `$85:E03D`): selector 0 pushes Down, 1 Up, 2 sideways
    /// (mirrored for Left), on the enemy's own movement. A script that
    /// handles knockback itself (`+$06 & $0010`) is not pushed: the death
    /// check comes at once.
    pub(crate) fn take_hit(&mut self, damage: u16, away: Direction, image: &[u8]) {
        let steady = self.guard.1 & NO_KNOCKBACK != 0;
        let Some(mut foe) = self.foe.take() else {
            return;
        };
        foe.life = foe.life.saturating_sub(damage);
        foe.immune = IMMUNE;
        if steady {
            if foe.life == 0 {
                self.explode(&mut foe, image);
            } else {
                self.struck = true;
            }
            self.foe = Some(foe);
            self.mirror_life();
            return;
        }
        foe.knocked = true;
        self.foe = Some(foe);
        self.mirror_life();
        self.line = None;
        let (selector, mirrored) = match away {
            Direction::Down => (0, false),
            Direction::Up => (1, false),
            Direction::Left => (2, true),
            Direction::Right => (2, false),
        };
        self.hflip = mirrored;
        self.stream = None;
        self.motion = self
            .movement(image)
            .and_then(|resource| motion::Motion::start(resource, selector, true, None));
    }

    /// A frame of a hit's aftermath. Returns whether it took the frame from
    /// the script.
    pub(crate) fn foe_frame(&mut self, image: &[u8], random: &mut crate::random::Random) -> bool {
        let Some(mut foe) = self.foe.take() else {
            return false;
        };
        foe.immune = foe.immune.saturating_sub(1);
        // The overlay's and the push's poses go on as the script's would.
        self.pose_age = self.pose_age.wrapping_add(1);
        let busy = if let Some(mut gem) = foe.gem {
            gem.age += 1;
            self.hidden = gem.age >= GEM_BLINKS && gem.age % 2 == 1;
            if gem.age >= GEM_GONE {
                self.state = State::Gone;
                foe.gem = None;
            } else {
                foe.gem = Some(gem);
            }
            true
        } else if let Some(left) = foe.exploding {
            if left <= 1 {
                foe.exploding = None;
                foe.dead = true;
                self.drop_gem(&mut foe, image, random);
            } else {
                foe.exploding = Some(left - 1);
            }
            true
        } else if foe.knocked {
            self.push(&mut foe, image);
            foe.knocked || foe.exploding.is_some()
        } else {
            false
        };
        self.foe = Some(foe);
        busy
    }

    /// A frame of the push; when it ends, the death check (`$85:E218`).
    fn push(&mut self, foe: &mut Foe, image: &[u8]) {
        let step = self
            .motion
            .as_mut()
            .filter(|motion| motion.running())
            .and_then(|motion| motion.step((self.hflip, self.vflip)));
        if let Some(delta) = step {
            self.displace(delta);
            return;
        }
        // The script goes on past the wait it was in.
        foe.knocked = false;
        self.motion = None;
        if foe.life == 0 {
            self.explode(foe, image);
        } else if matches!(self.state, State::Waiting(_)) {
            self.state = State::Waiting(0);
        }
        // `$85:E233`: alive after the push, the struck callback.
        self.struck |= foe.life > 0;
    }

    /// Held for a fight the world drives (Shadowkeeper): its script stops,
    /// it becomes a counted enemy of its descriptor's profile with that
    /// descriptor's boxes, and takes no knockback.
    pub(crate) fn hold_as_boss(&mut self, image: &[u8], descriptor: Option<usize>) {
        self.state = State::Held;
        self.hidden = false;
        self.guard.1 |= NO_KNOCKBACK;
        let Some(descriptor) = descriptor else {
            return;
        };
        let profile = image
            .get(descriptor + 4)
            .and_then(|&index| crate::combat::profile(image, index));
        self.foe = profile.map(|profile| Foe::new(profile, true));
        if self.boxes.is_none() {
            self.boxes = super::cadence::pose_boxes(image, descriptor).map(Rc::new);
        }
    }

    /// Whether hits may land on it (`+$04` bit `$0020` clear).
    pub(crate) fn set_target(&mut self, target: bool) {
        if target {
            self.guard.0 &= !NOT_TARGET;
        } else {
            self.guard.0 |= NOT_TARGET;
        }
    }

    /// Its life, while an enemy.
    pub(crate) fn foe_life(&self) -> Option<u16> {
        self.foe.as_ref().map(|foe| foe.life)
    }

    /// A new life, the explosion called off.
    pub(crate) fn revive(&mut self, life: u16) {
        if let Some(foe) = &mut self.foe {
            (foe.life, foe.exploding, foe.dead, foe.knocked) = (life, None, false, false);
        }
        (self.died, self.overlay) = (false, None);
    }

    /// A jump to the death script (`$85:E27B`): an enemy explodes, as its
    /// death check would have it; anything else goes at once (the High
    /// Cadet's hidden controller, `$97:C712`).
    pub(crate) fn die(&mut self, image: &[u8]) {
        self.go_up(image, false);
    }

    /// The explosion, unless one is under way; gone at once without a foe.
    fn go_up(&mut self, image: &[u8], no_gem: bool) {
        let Some(mut foe) = self.foe.take() else {
            self.state = State::Gone;
            self.root_died |= self.root;
            return;
        };
        if foe.exploding.is_none() && !foe.dead {
            foe.life = 0;
            if no_gem {
                foe.profile.gems = 0;
            }
            self.explode(&mut foe, image);
        }
        self.foe = Some(foe);
    }

    /// Whether it headed a group and died since last asked: the group goes
    /// with it (`$85:E383`).
    pub(crate) fn take_root_death(&mut self) -> bool {
        std::mem::take(&mut self.root_died)
    }

    /// Its root died (`$85:E383`): a hidden member, or one out of the hit
    /// scan (`+$04` bit 1), goes at once; any other explodes and goes,
    /// leaving no gem (`$85:E353`).
    pub(crate) fn go_with_root(&mut self, image: &[u8]) {
        if self.hidden || self.guard.0 & 2 != 0 {
            self.state = State::Gone;
        } else {
            self.go_up(image, true);
        }
    }

    /// The death script's start (`$85:E27B`): the explosion.
    fn explode(&mut self, foe: &mut Foe, image: &[u8]) {
        foe.exploding = Some(EXPLOSION);
        self.died = true;
        self.root_died |= self.root;
        self.motion = None;
        self.stream = None;
        self.overlay = Some((helper(image), EXPLODING));
        self.pose_age = 0;
    }

    /// The drop roll after the explosion (`$85:E2E9`): a gem, or gone.
    fn drop_gem(&mut self, foe: &mut Foe, image: &[u8], random: &mut crate::random::Random) {
        random.step();
        let mask = u16::from(foe.profile.drop_mask);
        if foe.profile.gems == 0 || random.word() & mask != 0 {
            self.state = State::Gone;
            self.overlay = None;
            return;
        }
        let amount = foe.profile.gems;
        let pose = match amount {
            0..10 => 0x0B,
            10..100 => 0x0C,
            _ => 0x0D,
        };
        foe.gem = Some(Gem { amount, age: 0 });
        self.overlay = Some((helper(image), pose));
        self.pose_age = 0;
    }

    /// Takes the gem when Ark's probe (x, y - 8) is on it (`$85:E482`).
    pub(crate) fn take_gem(&mut self, player: (u16, u16)) -> Option<u16> {
        let gem = self.foe.as_ref()?.gem.filter(|gem| gem.takeable())?;
        let (x, y) = (i32::from(self.position.0), i32::from(self.position.1));
        let probe = (i32::from(player.0), i32::from(player.1) - 8);
        let inside = (x - 8..x + 8).contains(&probe.0) && (y - 16..y).contains(&probe.1);
        if !inside {
            return None;
        }
        self.state = State::Gone;
        if let Some(foe) = &mut self.foe {
            foe.gem = None;
        }
        Some(gem.amount)
    }
}

/// The boxes of every list, shared by the actors of one descriptor.
pub(crate) type Boxes = Rc<Vec<Vec<Record>>>;
