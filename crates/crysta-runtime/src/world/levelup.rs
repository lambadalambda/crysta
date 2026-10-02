//! A level gained (`docs/combat.md` §9, `$85:EA99`, helper `$85:F35B`):
//! one level a pass. Ark stands for the victory (sound `$4E` 10 frames in),
//! the window names the new level 80 frames in, then each stat that rose,
//! one text after the other; the window closes, Ark recovers for 35
//! frames, and the new stats count, Ark immune for 60 frames. The enemies
//! run on; Ark cannot be hurt.
//!
//! Not modelled: Ark's victory and recovery poses (resource 0 list `$1D`,
//! resource 2 list `$20`) and the window's scrolling lines (each text shows
//! on its own). Poses tracked: `meta/issues/ark-underworld-poses.md`.

use super::{Step, World, WorldError};
use crate::combat::Level;
use crate::scene::Presses;
use assets::layout::per_revision;

/// The victory's sound (port 3) and its frame.
const VICTORY: u16 = 10;
const VICTORY_SOUND: u8 = 0x4E;
/// The level's text.
const FIRST_TEXT: u16 = 80;
/// Frames after a text is typed out before the next.
const PAUSE: u16 = 3;
/// The recovery, and Ark's immunity after it (`7F:1020 = $FFC4`; one more,
/// as the hit scan counts down before it tests).
const RECOVERY: u16 = 35;
const IMMUNE: u16 = 61;
/// The texts, Japanese and European: the level, life, strength, defense,
/// luck, and the close.
const TEXTS: [[u32; 6]; 2] = [
    [
        0x92_8000, 0x92_8020, 0x92_803E, 0x92_805B, 0x92_8078, 0x92_8095,
    ],
    [
        0x92_8046, 0x92_805F, 0x92_8073, 0x92_8084, 0x92_809B, 0x92_80AF,
    ],
];
const CLOSE: usize = 5;

/// A level being gained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LevelUp {
    from: Level,
    to: Level,
    frame: u16,
    /// The texts left: index into [`TEXTS`] and the value `$0982` prints.
    texts: Vec<(usize, u16)>,
    /// Frames since the text showing was typed out.
    shown: Option<u16>,
    /// Frames of the recovery, once the window closed.
    recovering: Option<u16>,
}

impl LevelUp {
    /// From level `from` to `to` (one level).
    pub(super) fn new(from: Level, to: Level, level: u8) -> Self {
        let rises = [
            to.life.saturating_sub(from.life),
            to.attack.saturating_sub(from.attack),
            to.defense.saturating_sub(from.defense),
            u16::from(to.luck.saturating_sub(from.luck)),
        ];
        let texts = std::iter::once((0, u16::from(level)))
            .chain(
                rises
                    .into_iter()
                    .enumerate()
                    .filter(|&(_, rise)| rise > 0)
                    .map(|(index, rise)| (index + 1, rise)),
            )
            .chain(std::iter::once((CLOSE, 0)))
            .collect();
        Self {
            from,
            to,
            frame: 0,
            texts,
            shown: None,
            recovering: None,
        }
    }
}

impl World<'_> {
    /// Whether a level gained holds Ark.
    #[must_use]
    pub const fn levelling(&self) -> bool {
        self.level_up.is_some()
    }

    /// A frame of the level gained: the world runs on, Ark held.
    pub(super) fn level_up_frame(&mut self, presses: Presses) -> Result<Option<Step>, WorldError> {
        let Some(mut level) = self.level_up.take() else {
            return Ok(None);
        };
        self.globals.dialogue.press(presses);
        level.frame = level.frame.saturating_add(1);
        if level.frame == VICTORY {
            self.globals.audio.sound_port3(VICTORY_SOUND);
        }
        let mut done = false;
        if let Some(frames) = &mut level.recovering {
            *frames += 1;
            done = *frames >= RECOVERY;
        } else if level.frame >= FIRST_TEXT {
            self.level_text(&mut level);
        }
        if done {
            // A second level comes on a new pass, once Ark is free.
            self.globals.slot.raise_level(&level.from, &level.to);
            self.ark_immune = IMMUNE;
        } else {
            self.level_up = Some(level);
        }
        self.run_actors()?;
        Ok(Some(Step::Stayed))
    }

    /// The texts in turn, each once the one before is typed out.
    fn level_text(&mut self, level: &mut LevelUp) {
        if self.globals.dialogue.busy() {
            return;
        }
        match &mut level.shown {
            Some(frames) if *frames < PAUSE => {
                *frames += 1;
                return;
            }
            _ => {}
        }
        let Some(&(text, value)) = level.texts.first() else {
            level.recovering = Some(0);
            return;
        };
        let source = TEXTS[per_revision(self.image, 0, 1)][text];
        let pages =
            assets::text::HouseDialogue::decode_reading(self.image, source, |at| match at {
                0x0982 => Some(value.to_le_bytes()[0]),
                0x0983 => Some(value.to_le_bytes()[1]),
                _ => None,
            })
            .unwrap_or_default();
        if self.globals.dialogue.request(pages) {
            level.texts.remove(0);
            level.shown = Some(0);
            // The close: the recovery starts at once.
            if text == CLOSE {
                level.recovering = Some(0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level(life: u16, attack: u16, defense: u16, luck: u8) -> Level {
        Level {
            exp: 0,
            life,
            attack,
            defense,
            luck,
        }
    }

    #[test]
    fn the_texts_name_the_level_then_each_stat_that_rose() {
        let up = LevelUp::new(level(28, 8, 5, 3), level(33, 9, 5, 4), 2);
        assert_eq!(up.texts, [(0, 2), (1, 5), (2, 1), (4, 1), (CLOSE, 0)]);
    }
}
