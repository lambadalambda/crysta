//! The end of Chapter 1 (`docs/underworld-end.md` §2): the Hole's exit
//! leads to the vortex `$201`, a Mode 7 scene, and on to the title card
//! `$129`, "Chapter 2 / Resurrection of the World", which sets flag `$06F`
//! and goes to Chapter 2's first map `$128`.
//!
//! A first, still version: dark for the vortex, then the card as the ROM
//! runs it, and the end of what the runtime plays. Not modelled: the vortex
//! (`$90:8436`) and the card's picture; the European title, which the page
//! geometry refuses (an 8-pixel advance, guess), shows no text.
//! Tracked: `meta/issues/chapter-end-vortex-card.md`.

use super::{Step, World};
use assets::layout::per_revision;

/// The vortex's map, which the Hole's exit names.
pub(super) const VORTEX_MAP: u16 = 0x0201;
/// Dark frames in place of the vortex (guess: about its length).
const VORTEX: u16 = 600;
/// The card (`$90:86E2`): flag `$06F` and music `$1D`, 60 frames, the
/// title's text (Japanese and European) until it closes, 720 frames.
const CHAPTER_TWO: u16 = 0x806F;
const CARD_MUSIC: u8 = 0x1D;
const BEFORE_TEXT: u16 = 60;
const TITLE: [u32; 2] = [0x90_87D7, 0x90_88A2];
const CARD: u16 = 720;

/// The chapter's end under way: frames into the vortex, then the card's
/// step and frames into it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct ChapterEnd {
    frame: u16,
    card: Card,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Card {
    #[default]
    Vortex,
    BeforeText(u16),
    Text,
    After(u16),
    Over,
}

impl World<'_> {
    /// Whether Chapter 1 is over: the title card has been shown.
    #[must_use]
    pub fn chapter_over(&self) -> bool {
        self.chapter.is_some_and(|end| end.card == Card::Over)
    }

    /// Whether the chapter's end holds the screen.
    #[must_use]
    pub const fn ending_chapter(&self) -> bool {
        self.chapter.is_some()
    }

    /// A frame of the chapter's end: dark, then the title card.
    pub(super) fn chapter_frame(&mut self) -> Option<Step> {
        let mut end = self.chapter?;
        end.card = match end.card {
            Card::Vortex => {
                end.frame += 1;
                if end.frame < VORTEX {
                    Card::Vortex
                } else {
                    self.globals.write_flag(CHAPTER_TWO);
                    self.globals.audio.play(CARD_MUSIC, false);
                    Card::BeforeText(0)
                }
            }
            Card::BeforeText(frames) if frames + 1 < BEFORE_TEXT => Card::BeforeText(frames + 1),
            Card::BeforeText(_) => {
                let source = TITLE[per_revision(self.image, 0, 1)];
                let pages =
                    assets::text::HouseDialogue::decode_at(self.image, source).unwrap_or_default();
                self.globals.dialogue.request(pages);
                Card::Text
            }
            Card::Text if self.globals.dialogue.busy() => Card::Text,
            Card::Text => Card::After(0),
            Card::After(frames) if frames + 1 < CARD => Card::After(frames + 1),
            Card::After(_) | Card::Over => Card::Over,
        };
        self.chapter = Some(end);
        Some(Step::Stayed)
    }
}
