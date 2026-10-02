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
/// The game over's music (`COP 30 3B`) and its frame, then the text's
/// frame, Japanese and European (`docs/combat.md` §9).
const DOWN_MUSIC: u16 = 111;
const DOWN_TRACK: u8 = 0x3B;
const DOWN_TEXT: [u16; 2] = [180, 172];
/// "Ark's senses faded away..." (`$84:DD52`, European `$84:DD11`): a
/// frameless page over the map.
const DOWN_TEXTS: [u32; 2] = [0x84_DD52, 0x84_DD11];

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
    /// A frame of Ark down at no life (`$84:DC59`): he collapses, the game
    /// over's music plays, then the text; once it is typed out he wakes
    /// where the game was saved (`$0600..$0607`) with his life full.
    pub(super) fn down_frame(&mut self) -> Result<Option<Step>, WorldError> {
        let Some(frames) = self.down.take() else {
            return Ok(None);
        };
        let europe = assets::layout::per_revision(self.image, 0, 1);
        let text = DOWN_TEXT[europe];
        // He stays down through the transfer's fade, until the load.
        let frame = if frames == u16::MAX - 1 {
            frames
        } else {
            frames.saturating_add(1)
        };
        self.down = Some(frame);
        if frame == DOWN_MUSIC {
            self.globals.audio.play(DOWN_TRACK, false);
        }
        if frame == text {
            let pages = assets::text::HouseDialogue::decode_at(self.image, DOWN_TEXTS[europe])
                .unwrap_or_default();
            if !self.globals.dialogue.request(pages) {
                self.down = Some(frame - 1);
            }
        } else if frame > text && frame < u16::MAX - 1 && !self.globals.dialogue.busy() {
            let stats = self.globals.slot.stats();
            self.globals.slot.set_life(stats.max_life);
            self.globals.dialogue.request(Vec::new());
            self.globals.transfer = Some(crate::scene::Transfer {
                map: self.globals.slot.map(),
                position: self.globals.slot.position(),
                mode: 0,
            });
            // Once: he lies on until the load.
            self.down = Some(u16::MAX - 1);
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
