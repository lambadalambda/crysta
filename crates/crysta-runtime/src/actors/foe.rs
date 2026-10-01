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
        if self.hidden {
            return None;
        }
        Some(self.place(self.record()?.body))
    }

    /// The attack box that hurts Ark, while the enemy is up and about.
    pub(crate) fn hurt_box(&self) -> Option<Rect> {
        let foe = self.foe.as_ref()?;
        if self.hidden || foe.knocked || foe.exploding.is_some() || foe.dead || foe.gem.is_some() {
            return None;
        }
        Some(self.place(self.record()?.attack))
    }

    /// Takes `damage` from a hit that pushes toward `away` (`$85:D578`,
    /// knockback `$85:E03D`): selector 0 pushes Down, 1 Up, 2 sideways
    /// (mirrored for Left), on the enemy's own movement.
    pub(crate) fn take_hit(&mut self, damage: u16, away: Direction, image: &[u8]) {
        let Some(foe) = &mut self.foe else {
            return;
        };
        foe.life = foe.life.saturating_sub(damage);
        foe.immune = IMMUNE;
        foe.knocked = true;
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
            .and_then(|resource| motion::Motion::start(resource, selector, mirrored, true, None));
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
            .and_then(motion::Motion::step);
        if let Some((dx, dy)) = step {
            self.position = (
                self.position.0.wrapping_add_signed(dx),
                self.position.1.wrapping_add_signed(dy),
            );
            return;
        }
        // The script goes on past the wait it was in.
        foe.knocked = false;
        self.motion = None;
        if foe.life == 0 {
            foe.exploding = Some(EXPLOSION);
            self.died = true;
            self.overlay = Some((helper(image), EXPLODING));
            self.pose_age = 0;
        } else if matches!(self.state, State::Waiting(_)) {
            self.state = State::Waiting(0);
        }
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
