//! The text window's art: its BG3 frame, the blue shading of its colour 3
//! and the "press" prompt.
//!
//! `$85:9782` frames the content area one tile outside it: `$10`, `$11`
//! across, `$12` on top; `$13`/`$14` down the sides; `$15`, `$16`, `$17`
//! at the bottom; `$20` (all colour 3) inside, palette 0, priority set. The
//! characters are the packet `$A9:9000`'s, the colours `$B2:8B78`'s. While
//! a window is open, HDMA (`$85:80D3`, table `$85:8160`) rewrites colour 3
//! every scanline from `$85:81E4`, a 32-line shade repeating down the
//! screen. `$D5` pages show the prompt `$CB:7A98` (4 glyphs, 9 ticks each)
//! in the cell after their last glyph. The European ROM holds the same
//! bytes at `$AB:9000`, `$B4:90DB` and `$CD:7A98` ([`crate::layout::at`]).

use super::TextError;
use crate::compression;
use crate::graphics::{decode_glyph_2bpp, decode_tile_2bpp, Bgr555};

const CHARACTERS: u32 = 0xA9_9000;
const COLOURS: u32 = 0xB2_8B78;
const SHADE: u32 = 0x85_81E4;
const PROMPT: u32 = 0xCB_7A98;
/// Ticks each prompt glyph shows.
pub const PROMPT_TICKS: u64 = 9;

/// The window's art.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowArt {
    /// Frame tiles `$10..=$17`: top-left, top, top-right, left, right,
    /// bottom-left, bottom, bottom-right; 2bpp indices.
    pub frame: [[u8; 64]; 8],
    /// The interior tile `$20`.
    pub interior: [u8; 64],
    /// BG3 colours 0..12: palettes 0 (1 white, 2 edge), 1 (5 the default
    /// speaker colour, 6 its edge) and 2; each palette's colour 3 is the
    /// shade.
    pub colours: [Bgr555; 12],
    /// The prompt's four 16×16 glyphs.
    pub prompt: Vec<[u8; 256]>,
    shade: [Bgr555; 32],
}

impl WindowArt {
    /// Decodes the art.
    ///
    /// # Errors
    /// Refuses a packet or table outside the image or malformed.
    pub fn from_rom(image: &[u8]) -> Result<Self, TextError> {
        let located = |japan: u32| {
            crate::layout::at(image, japan)
                .ok_or_else(|| super::invalid(japan, "unrecorded in this revision"))
        };
        let (characters_at, colours_at, shade_at, prompt_at) = (
            located(CHARACTERS)?,
            located(COLOURS)?,
            located(SHADE)?,
            located(PROMPT)?,
        );
        let start = usize::try_from(characters_at & 0x3F_FFFF).unwrap_or(usize::MAX);
        let characters = image
            .get(start..)
            .and_then(|input| compression::decode(input, 0x2000).ok())
            .ok_or_else(|| super::invalid(CHARACTERS, "window characters"))?
            .data;
        let tile = |index: usize| -> Result<[u8; 64], TextError> {
            characters
                .get(index * 16..index * 16 + 16)
                .map(decode_tile_2bpp)
                .ok_or_else(|| super::invalid(CHARACTERS, "short window characters"))
        };
        let bytes = |at: u32, count: usize| -> Result<&[u8], TextError> {
            let start = usize::try_from(at & 0x3F_FFFF).unwrap_or(usize::MAX);
            image
                .get(start..start + count)
                .ok_or_else(|| super::invalid(at, "window art outside the image"))
        };
        let colour = |at: u32| -> Result<Bgr555, TextError> {
            let word = bytes(at, 2)?;
            Ok(Bgr555::new(u16::from_le_bytes([word[0], word[1]])))
        };
        let mut frame = [[0; 64]; 8];
        for (index, slot) in frame.iter_mut().enumerate() {
            *slot = tile(0x10 + index)?;
        }
        let mut colours = [Bgr555::new(0); 12];
        for (index, slot) in (0_u32..).zip(colours.iter_mut()) {
            *slot = colour(colours_at + index * 2)?;
        }
        let mut shade = [Bgr555::new(0); 32];
        for (line, slot) in (0_u32..).zip(shade.iter_mut()) {
            *slot = colour(shade_at + line * 4 + 2)?;
        }
        let prompt = (0..4)
            .map(|index| Ok(decode_glyph_2bpp(bytes(prompt_at + index * 64, 64)?)))
            .collect::<Result<_, TextError>>()?;
        Ok(Self {
            frame,
            interior: tile(0x20)?,
            colours,
            prompt,
            shade,
        })
    }

    /// Colour 3 on screen line `y`: the window's shade.
    #[must_use]
    pub const fn shade(&self, y: usize) -> Bgr555 {
        self.shade[y % 32]
    }
}
