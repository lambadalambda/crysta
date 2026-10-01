//! Enemies hurting Ark (`docs/combat.md` §5): the hit scan on his body, his
//! push and blink, and going down at no life.

use super::attack::away;
use super::{Step, World, WorldError};
use crate::combat;
use crate::scene::{DigitKind, Digits};
use assets::sprites::boxes;
use room_core::Direction;

/// Ark's body box (dx, w, dy, h).
const ARK_BODY: [i8; 4] = [-5, 10, -16, 14];
/// Frames Ark is pushed by a hit, the pad locked (`$84:80B2`..).
const HURT: u16 = 26;
/// Of those, the frames that move him.
const HURT_PUSH: u16 = 10;
/// Frames before Ark can be hit again (`7F:1020`).
const ARK_IMMUNE: u16 = 43;
/// The sound of Ark hurt.
const HURT_SOUND: u8 = 0x07;
/// Frames Ark lies down before he wakes, after the push (natively 707 from
/// the hit to the load).
const DOWN: u16 = 650;

/// Ark pushed by a hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Hurt {
    /// The way he is pushed.
    away: Direction,
    /// The way back to the attacker, which he faces afterwards.
    toward: Direction,
    frame: u16,
}

impl World<'_> {
    /// A frame of Ark down at no life (`$84:DC59`): he lies there, then
    /// wakes where the game was saved (`$0600..$0607`) with his life full.
    pub(super) fn down_frame(&mut self) -> Result<Option<Step>, WorldError> {
        let Some(frames) = self.down.take() else {
            return Ok(None);
        };
        // He stays down through the transfer's fade, until the load.
        self.down = Some(frames.saturating_add(1).min(DOWN));
        if frames + 1 == DOWN {
            let stats = self.globals.slot.stats();
            self.globals.slot.set_life(stats.max_life);
            self.globals.transfer = Some(crate::scene::Transfer {
                map: self.globals.slot.map(),
                position: self.globals.slot.position(),
                mode: 0,
            });
        }
        self.run_actors()?;
        Ok(Some(Step::Stayed))
    }

    /// A frame of Ark pushed by a hit: he moves away for ten frames, as
    /// walls let him, then stands until the push ends and faces the
    /// attacker, or goes down at no life.
    pub(super) fn hurt_frame(&mut self) -> Result<Option<Step>, WorldError> {
        let Some(mut hurt) = self.hurt.take() else {
            return Ok(None);
        };
        hurt.frame += 1;
        if hurt.frame <= HURT_PUSH {
            let input = room_core::FrameInput {
                direction: Some(hurt.away),
            };
            let _ = self.walking.step(&self.room.room, input);
        }
        if hurt.frame < HURT {
            self.hurt = Some(hurt);
        } else if self.globals.slot.stats().life == 0 {
            self.down = Some(0);
        } else {
            self.face(hurt.toward);
        }
        self.run_actors()?;
        Ok(Some(Step::Stayed))
    }

    /// Whether Ark blinks out this frame: the hit's second frame, fourth,
    /// ... while the push lasts.
    #[must_use]
    pub fn ark_blinks(&self) -> bool {
        self.hurt.is_some_and(|hurt| hurt.frame % 2 == 0)
    }

    /// The enemies' hit scan on Ark (`$85:D30C`): an attack box touching
    /// his body costs life and pushes him away. Runs on game frames only.
    pub(super) fn hurt_ark(&mut self) {
        self.ark_immune = self.ark_immune.saturating_sub(1);
        if self.ark_immune > 0 || self.hurt.is_some() || self.down.is_some() {
            return;
        }
        let body = boxes::place(ARK_BODY, self.position(), false);
        let Some((profile, from)) = self.actors.iter().find_map(|actor| {
            let attack = actor.hurt_box()?;
            let profile = actor.foe.as_ref()?.profile;
            boxes::overlap(attack, body).then_some((profile, actor.position))
        }) else {
            return;
        };
        let stats = self.globals.slot.stats();
        let damage = combat::enemy_damage(&profile, 0, &stats, self.globals.frames);
        self.globals
            .slot
            .set_life(stats.life.saturating_sub(damage));
        self.globals.digits.push(Digits {
            at: (self.position().0, self.position().1.saturating_sub(24)),
            amount: damage,
            kind: DigitKind::Ark,
            age: 0,
        });
        self.globals.audio.sound_port3(HURT_SOUND);
        self.thrust = None;
        self.hurt = Some(Hurt {
            away: away(from, self.position()),
            toward: away(self.position(), from),
            frame: 0,
        });
        self.ark_immune = ARK_IMMUNE;
    }

    /// Ark's life and the most he can have.
    #[must_use]
    pub fn life(&self) -> (u16, u16) {
        let stats = self.globals.slot.stats();
        (stats.life, stats.max_life)
    }
}
