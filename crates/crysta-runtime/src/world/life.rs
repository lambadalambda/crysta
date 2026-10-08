//! Ark's life from frame to frame (`docs/combat.md` §5): the pending life
//! a heal or the regeneration adds a point a frame (`$85:E956`), the way
//! down at no life whatever took it (`$85:E15B`), and on a tower floor the
//! regeneration and the low-life warning (`$85:E14B`). A fall's damage
//! (`$84:D4F4`) is here too.

use super::World;
use crate::scene::{DigitKind, Digits};

/// The sound of a point of life added, and of the low-life warning.
const FILL_SOUND: u8 = 0x29;
const WARNING: u8 = 0x1C;
/// `$048A` bit 14: no life upkeep at all (the underworld's plane); bit
/// 15, a tower floor; bit 10, the fill runs whatever Ark does.
const NO_UPKEEP: u16 = 0x4000;
const TOWER_FLOOR: u16 = 0x8000;
const ALWAYS_FILL: u16 = 0x0400;
/// The weapons that regenerate, and the frames between their points.
const REGENERATING: [(u8, u16); 2] = [(0x81, 0xFF), (0x9C, 0x3F)];

/// What the frame's upkeep reads.
#[allow(clippy::struct_excessive_bools)] // independent engine bits, not a state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Upkeep {
    pub(super) life: u16,
    pub(super) max_life: u16,
    /// `$04CE`: life still to add.
    pub(super) pending: u16,
    /// `$42`, the frame counter.
    pub(super) frames: u16,
    /// `$048A`.
    pub(super) mode: u16,
    /// `$07EF`: the regeneration's word.
    pub(super) regenerates: bool,
    pub(super) weapon: Option<u8>,
    /// `$097C & $8000`, which holds the fill (`$85:E94E`).
    pub(super) action_holds: bool,
    /// Ark's `+$04 & $40`, which skips the check (`$85:E14B`).
    pub(super) out_of_check: bool,
    /// Whether Ark may go down now: not already, and in no action that
    /// holds it off (`$097C & $8E17`, `$0986 & $1400`).
    pub(super) may_go_down: bool,
}

/// What it changes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Upkept {
    pub(super) life: u16,
    pub(super) pending: u16,
    pub(super) sound: Option<u8>,
    pub(super) down: bool,
}

/// One frame of the upkeep: the fill (`$85:E941`), then the check
/// (`$85:E14B`).
#[allow(clippy::verbose_bit_mask)] // the ROM's `AND` masks of `$42`
pub(super) fn upkeep(now: Upkeep) -> Upkept {
    let mut out = Upkept {
        life: now.life,
        pending: now.pending,
        ..Upkept::default()
    };
    let fills = now.mode & ALWAYS_FILL != 0 || !now.action_holds;
    if out.life != 0 && out.pending > 0 && fills {
        out.pending -= 1;
        if now.frames & 3 == 0 {
            out.sound = Some(FILL_SOUND);
        }
        out.life = (out.life + 1).min(now.max_life);
    }
    if now.out_of_check || now.mode & NO_UPKEEP != 0 {
        return out;
    }
    if out.life == 0 {
        out.down = now.may_go_down;
        return out;
    }
    if now.mode & TOWER_FLOOR == 0 {
        return out;
    }
    let regenerates = now.regenerates
        && REGENERATING
            .iter()
            .any(|&(weapon, every)| now.weapon == Some(weapon) && now.frames & every == 0);
    if regenerates && now.max_life != out.life {
        out.pending += 1;
        out.sound = Some(FILL_SOUND);
    }
    if now.max_life / 4 >= out.life && now.frames & 0x7F == 0 {
        out.sound = Some(WARNING);
    }
    out
}

/// A fall's damage (`$84:D4F4`): a sixteenth of the life or of half the
/// most life, whichever is more, at least 4.
pub(super) const fn fall_damage(life: u16, max_life: u16) -> u16 {
    let base = if max_life / 2 >= life {
        max_life / 2
    } else {
        life
    };
    let damage = base / 16;
    if damage < 4 {
        4
    } else {
        damage
    }
}

impl World<'_> {
    /// A frame of the upkeep, on game frames.
    pub(super) fn life_frame(&mut self) {
        let stats = self.globals.slot.stats();
        let upkept = upkeep(Upkeep {
            life: stats.life,
            max_life: stats.max_life,
            pending: self.pending_life,
            frames: self.globals.frames,
            mode: self
                .globals
                .scratch
                .get(&crate::actors::MAP_MODE)
                .copied()
                .unwrap_or(0),
            regenerates: self.globals.slot.regenerates(),
            weapon: self.globals.slot.weapon().map(|(item, _)| item),
            action_holds: self
                .globals
                .scratch
                .get(&crate::actors::PLAYER_ACTION)
                .is_some_and(|action| action & 0x8000 != 0),
            out_of_check: self.globals.ark_flags & 0x0040 != 0,
            may_go_down: self.down.is_none() && !self.holds_life(),
        });
        if upkept.life != stats.life {
            self.globals.slot.set_life(upkept.life);
        }
        self.pending_life = upkept.pending;
        if let Some(sound) = upkept.sound {
            self.globals.audio.sound_port3(sound);
        }
        if upkept.down {
            self.down = Some(0);
        }
    }

    /// Whether an action holds the way down off: a fall, a jump, the rope,
    /// a hit's push, a map change.
    fn holds_life(&self) -> bool {
        self.fall.is_some()
            || self.jump.is_some()
            || self.rope.is_some()
            || self.hurt.is_some()
            || self.in_transition()
    }

    /// A fall's cost: the damage and its digits over Ark (`$84:9FA3`,
    /// `COP D5 00 36`).
    pub(super) fn fall_cost(&mut self) {
        let stats = self.globals.slot.stats();
        let damage = fall_damage(stats.life, stats.max_life);
        self.globals
            .slot
            .set_life(stats.life.saturating_sub(damage));
        let (x, y) = self.position();
        self.globals.digits.push(Digits {
            at: (x, y.saturating_sub(24)),
            amount: damage,
            kind: DigitKind::Ark,
            age: 0,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLOOR: Upkeep = Upkeep {
        life: 40,
        max_life: 40,
        pending: 0,
        frames: 1,
        mode: 0x8000,
        regenerates: false,
        weapon: Some(0x81),
        action_holds: false,
        out_of_check: false,
        may_go_down: true,
    };

    #[test]
    fn pending_life_comes_a_point_a_frame_with_a_sound_every_fourth() {
        let now = Upkeep {
            life: 10,
            pending: 2,
            frames: 4,
            ..FLOOR
        };
        let out = upkeep(now);
        assert_eq!(
            (out.life, out.pending, out.sound),
            (11, 1, Some(FILL_SOUND))
        );
        let out = upkeep(Upkeep { frames: 5, ..now });
        assert_eq!(out.sound, None);
        // Never past the most life.
        let out = upkeep(Upkeep { life: 40, ..now });
        assert_eq!((out.life, out.pending), (40, 1));
        // Held by `$097C & $8000`, unless the map's mode fills always.
        let held = Upkeep {
            action_holds: true,
            ..now
        };
        assert_eq!(upkeep(held).pending, 2);
        let always = Upkeep {
            mode: 0x8400,
            ..held
        };
        assert_eq!(upkeep(always).pending, 1);
    }

    #[test]
    fn no_life_sends_ark_down_unless_an_action_holds_it_off() {
        let none = Upkeep { life: 0, ..FLOOR };
        assert!(upkeep(none).down);
        assert!(
            !upkeep(Upkeep {
                may_go_down: false,
                ..none
            })
            .down
        );
        // Not out of the check (Ark's `+$04 & $40`).
        let out = Upkeep {
            out_of_check: true,
            ..none
        };
        assert!(!upkeep(out).down);
        // Off a tower floor too, but not where the upkeep is off.
        assert!(upkeep(Upkeep { mode: 0, ..none }).down);
        assert!(
            !upkeep(Upkeep {
                mode: NO_UPKEEP,
                ..none
            })
            .down
        );
    }

    #[test]
    fn the_warning_sounds_every_128_frames_at_a_quarter_of_the_life() {
        let low = Upkeep {
            life: 10,
            frames: 0x80,
            ..FLOOR
        };
        assert_eq!(upkeep(low).sound, Some(WARNING));
        assert_eq!(
            upkeep(Upkeep {
                frames: 0x81,
                ..low
            })
            .sound,
            None
        );
        assert_eq!(upkeep(Upkeep { life: 11, ..low }).sound, None);
        assert_eq!(upkeep(Upkeep { mode: 0, ..low }).sound, None);
    }

    #[test]
    fn the_regeneration_adds_pending_life_by_the_weapon() {
        let hurt = Upkeep {
            life: 30,
            frames: 0x100,
            regenerates: true,
            ..FLOOR
        };
        assert_eq!(upkeep(hurt).pending, 1);
        assert_eq!(
            upkeep(Upkeep {
                frames: 0x40,
                ..hurt
            })
            .pending,
            0
        );
        let fast = Upkeep {
            weapon: Some(0x9C),
            frames: 0x40,
            ..hurt
        };
        assert_eq!(upkeep(fast).pending, 1);
        assert_eq!(
            upkeep(Upkeep {
                regenerates: false,
                ..hurt
            })
            .pending,
            0
        );
        assert_eq!(upkeep(Upkeep { life: 40, ..hurt }).pending, 0);
    }

    #[test]
    fn a_fall_costs_a_sixteenth_of_the_life_or_of_half_the_most() {
        assert_eq!(fall_damage(40, 40), 4);
        assert_eq!(fall_damage(200, 200), 12);
        // Half the most life when the life is lower.
        assert_eq!(fall_damage(20, 200), 6);
        assert_eq!(fall_damage(3, 40), 4);
    }
}
