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
/// Frames Ark is held by a hit, the pad locked: the push's script
/// (`$84:80B2`..), its stand, then his own (`$84:8018`..).
const HURT: u16 = 28;
/// Of those, the frames of the push's pose and stream: 10 pixels, 16 still,
/// one more as the stream loops before `COP 8E` sees its end.
const HURT_PUSH: usize = 27;
/// Frames before Ark can be hit again (`7F:1020`).
pub(super) const ARK_IMMUNE: u16 = 43;
/// The sound of Ark hurt (port 2, `$87:CDAD`).
const HURT_SOUND: u8 = 0x07;
/// An immune hit's sound (port 2) and frames out of reach (`$85:D70C`).
const GUARD_SOUND: u8 = 0x09;
const GUARD_IMMUNE: u16 = 16;
/// The armors' status blocks (`$8D:BD92`), from item `$A0` on.
const ARMOR_BLOCKS: usize = 0x0D_BD92;
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
    /// The push's stream, a move a frame.
    moves: [(i16, i16); HURT_PUSH],
}

impl Hurt {
    /// A push `away`, then facing `toward`: the stream of [`Self::list`].
    pub(super) fn new(image: &[u8], away: Direction, toward: Direction) -> Self {
        let (_, stream, hflip) = Self::list(away);
        let mut moves = [(0, 0); HURT_PUSH];
        for (slot, step) in moves
            .iter_mut()
            .zip(crate::actors::common_moves(image, stream, hflip, HURT_PUSH))
        {
            *slot = step;
        }
        Self {
            away,
            toward,
            frame: 0,
            moves,
        }
    }

    /// Resource 0's push list for the way he goes (`$84:80B2`..`80CC`):
    /// `$0D` down, `$0C` up, `$0E` sideways, mirrored to the right.
    const fn list(away: Direction) -> (u8, u8, bool) {
        match away {
            Direction::Down => (0x0D, 0x37, false),
            Direction::Up => (0x0C, 0x36, false),
            Direction::Left => (0x0E, 0x38, false),
            Direction::Right => (0x0E, 0x38, true),
        }
    }

    /// The push's pose, from the frame after the hit.
    pub(super) const fn pose(self) -> Option<super::pose::ArkPose> {
        let (list, _, hflip) = Self::list(self.away);
        if self.frame == 0 || self.frame as usize > HURT_PUSH {
            return None;
        }
        Some(super::pose::ArkPose::new(
            0,
            list,
            hflip,
            self.frame - 1,
            false,
        ))
    }
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
        if let Some(&(dx, dy)) = hurt.moves.get(usize::from(hurt.frame) - 1) {
            self.push_by((dx, dy));
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
    /// ... while the push lasts; or a script blinks him (the Cadet's
    /// paralysis, `$97:C2C8`).
    #[must_use]
    pub fn ark_blinks(&self) -> bool {
        self.hurt.is_some_and(|hurt| hurt.frame % 2 == 0)
            || self.globals.ark_flags & 0x8000 != 0
            || self.fall.is_some_and(super::fall::Fall::landed)
            || self.landing.is_some_and(super::landing::Landing::hidden)
    }

    /// The enemies' hit scan on Ark (`$85:D30C`): an attack box touching
    /// his body costs life and pushes him away. Runs on game frames only.
    pub(super) fn hurt_ark(&mut self) {
        self.ark_immune = self.ark_immune.saturating_sub(1);
        if self.ark_immune > 0 || self.hurt.is_some() || self.down.is_some() || self.hits_off() {
            return;
        }
        let body = boxes::place(ARK_BODY, self.position(), false);
        let Some((index, profile, from, kind)) =
            self.actors.iter().enumerate().find_map(|(index, actor)| {
                let attack = actor.hurt_box()?;
                let profile = actor.foe.as_ref()?.profile;
                boxes::overlap(attack, body).then_some((
                    index,
                    profile,
                    actor.position,
                    actor.attack_kind(),
                ))
            })
        else {
            return;
        };
        // The attacker's contact callback runs next frame (`$85:D65E`): the
        // Guardner's bolt takes Ark that way.
        if self.actors[index].contact().is_some() {
            self.touched = Some(index);
        }
        let stats = self.globals.slot.stats();
        let hit = combat::enemy_hit(
            &profile,
            kind,
            &self.ark_side(),
            self.globals.frames,
            &mut || {
                self.globals.random.step();
                self.globals.random.word().to_le_bytes()[0]
            },
        );
        if hit.immune {
            // `$85:D70C`: a guard sound, 16 frames out of reach.
            self.globals.audio.sound_port2(GUARD_SOUND);
            self.ark_immune = GUARD_IMMUNE;
            return;
        }
        if let Some(element) = hit.status {
            self.take_status(element);
        }
        self.globals
            .slot
            .set_life(stats.life.saturating_sub(hit.amount));
        self.globals.digits.push(Digits {
            at: (self.position().0, self.position().1.saturating_sub(24)),
            amount: hit.amount,
            kind: if hit.critical {
                DigitKind::Critical
            } else {
                DigitKind::Ark
            },
            age: 0,
        });
        self.globals.audio.sound_port2(HURT_SOUND);
        self.thrust = None;
        self.hurt = Some(Hurt::new(
            self.image,
            away(from, self.position()),
            away(self.position(), from),
        ));
        self.ark_immune = ARK_IMMUNE;
    }

    /// Whether a script turned the hit scans off (`$049A`).
    pub(super) fn hits_off(&self) -> bool {
        self.globals
            .scratch
            .get(&crate::actors::HITS_OFF)
            .is_some_and(|&off| off != 0)
    }

    /// Ark as an enemy's hit reads him: his stats, his types and his
    /// statuses from the slot (`$064E`..), and what his armor blocks
    /// (`$8D:BD92`, European `$8D:BC5B`).
    fn ark_side(&self) -> combat::ArkSide {
        let slot = &self.globals.slot;
        let armor_blocks = slot
            .armor()
            .and_then(|armor| armor.checked_sub(0xA0))
            .and_then(|index| {
                let table =
                    assets::layout::per_revision(self.image, ARMOR_BLOCKS, ARMOR_BLOCKS - 0x137);
                let at = table + usize::from(index) * 2;
                self.image.get(at..at + 2)
            })
            .map_or(0, |word| u16::from_le_bytes([word[0], word[1]]));
        combat::ArkSide {
            stats: slot.stats(),
            resist: [slot.word_at(0x064E), slot.word_at(0x0650)],
            weak: [slot.word_at(0x0652), slot.word_at(0x0654)],
            immune: slot.word_at(0x0652) | slot.word_at(0x0654),
            statuses: self.statuses(),
            armor_blocks,
        }
    }

    /// Ark's life and the most he can have.
    #[must_use]
    pub fn life(&self) -> (u16, u16) {
        let stats = self.globals.slot.stats();
        (stats.life, stats.max_life)
    }
}
