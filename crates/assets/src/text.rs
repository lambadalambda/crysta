//! Bounded Japanese house dialogue; no CPU, Unicode transcription or window engine.
//!
//! Pages retain the native font's two-bit pixels (3 = background). The caller
//! owns presentation and acknowledgement/progression. See `docs/house-dialogue.md`.
use std::fmt;

#[cfg(test)]
mod tests;

/// The entry greeting, not the progression conversation.
pub const ENTRY_TEXT: u32 = 0x88_8fda;
/// The resident's first progression conversation.
pub const RESIDENT_TEXT: u32 = 0x88_905a;
const WIDTH: usize = 224;
const HEIGHT: usize = 48;

/// Invalid or explicitly unsupported dialogue source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextError {
    /// Source address at which decoding failed (zero for image authentication).
    pub source: u32,
    /// Bounded failure description; unsupported controls are never skipped.
    pub reason: &'static str,
}
impl fmt::Display for TextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "text ${:06X}: {}", self.source, self.reason)
    }
}
impl std::error::Error for TextError {}
fn invalid(source: u32, reason: &'static str) -> TextError {
    TextError { source, reason }
}

/// Action after one acknowledgement of a fully presented page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Acknowledgement {
    /// Native `$D5`: clear this page and continue this same text invocation.
    Next,
    /// Native `$D3`: close the dialogue and return to the event caller.
    End,
}

/// Provenance and placement of one native font record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DialogueGlyph {
    /// Encoded character's source address; `$0610..0615` denotes the default name.
    pub text_source: u32,
    /// Runtime ROM address of the 64-byte, four-tile, 2bpp glyph.
    pub font_source: u32,
    /// Pixel coordinates relative to this page's content area.
    pub position: [u16; 2],
}

/// Immutable, precomposed page. Host bitmap presentation requires no original CPU.
#[derive(Debug, Clone)]
pub struct DialoguePage {
    pixels: Vec<u8>,
    glyphs: Vec<DialogueGlyph>,
    boundary_source: u32,
    acknowledgement: Acknowledgement,
}
impl DialoguePage {
    /// Content width, without native frame/window effects.
    #[must_use]
    pub const fn width(&self) -> u16 {
        224
    }
    /// Three native 16-pixel text rows.
    #[must_use]
    pub const fn height(&self) -> u16 {
        48
    }
    /// Row-major native 2bpp color indices: 0..3, with 3 the background.
    /// No color/font bytes or transcribed text are embedded in the library.
    #[must_use]
    pub fn indexed(&self) -> &[u8] {
        &self.pixels
    }
    /// Per-glyph source metadata, also useful for independent qualification.
    #[must_use]
    pub fn glyphs(&self) -> &[DialogueGlyph] {
        &self.glyphs
    }
    /// Address of the actual `$D5`/`$D3` control ending this page.
    #[must_use]
    pub const fn boundary_source(&self) -> u32 {
        self.boundary_source
    }
    /// The semantic action after one explicit acknowledgement.
    #[must_use]
    pub const fn acknowledgement(&self) -> Acknowledgement {
        self.acknowledgement
    }
}

/// Only the authenticated Japanese default-name room-B dialogue resources.
#[derive(Debug)]
pub struct HouseDialogue {
    entry: Vec<DialoguePage>,
    resident: Vec<DialoguePage>,
}
impl HouseDialogue {
    /// Compile the required text and font directly from a normalized Japanese ROM.
    /// This does not load/execute event scripts or award progression flags.
    ///
    /// # Errors
    /// Rejects any other ROM/hash, missing resources and unsupported text commands.
    pub fn from_rom(image: &[u8]) -> Result<Self, TextError> {
        if image.len() != 0x40_0000 || rom::digests(image).sha256 != rom::Revision::Japan.sha256() {
            return Err(invalid(0, "expected authenticated normalized Japanese ROM"));
        }
        Ok(Self {
            entry: decode(image, ENTRY_TEXT)?,
            resident: decode(image, RESIDENT_TEXT)?,
        })
    }
    /// Source-ID lookup. Page index is a stable zero-based key within the text ID;
    /// hosts can assign sequential numeric page keys scoped to the ROM identity.
    #[must_use]
    pub fn pages(&self, text_source: u32) -> Option<&[DialoguePage]> {
        match text_source {
            ENTRY_TEXT => Some(&self.entry),
            RESIDENT_TEXT => Some(&self.resident),
            _ => None,
        }
    }
}

fn bytes(image: &[u8], source: u32, count: usize) -> Result<&[u8], TextError> {
    if !((0x80..=0xbf).contains(&(source >> 16)) && source & 0xffff >= 0x8000) {
        return Err(invalid(source, "unsupported ROM address"));
    }
    let start = (source & 0x3f_ffff) as usize;
    image
        .get(start..start + count)
        .ok_or_else(|| invalid(source, "truncated source"))
}
fn glyph_pixels(source: &[u8]) -> Result<[u8; 256], TextError> {
    if source.len() != 64 {
        return Err(invalid(0, "glyph must contain 64 bytes"));
    }
    Ok(std::array::from_fn(|i| {
        let (x, y) = (i % 16, i / 16);
        let at = (y / 8 * 2 + x / 8) * 16 + y % 8 * 2;
        ((source[at] >> (7 - x % 8)) & 1) | (((source[at + 1] >> (7 - x % 8)) & 1) << 1)
    }))
}

struct Decoder<'a> {
    image: &'a [u8],
    pc: u32,
    stack: Vec<u32>,
    kana: bool,
    position: [u16; 2],
    page: DialoguePage,
    pages: Vec<DialoguePage>,
}
impl Decoder<'_> {
    fn next(&mut self) -> Result<u8, TextError> {
        let value = if (0x610..0x616).contains(&self.pc) {
            // $878C97..8CB6 initializes the default name with six LDA #byte / STA
            // absolute pairs. Read the immediates, not a copied string or capture.
            let source = 0x87_8c99 + (self.pc - 0x610) * 5;
            let instruction = bytes(self.image, source, 5)?;
            let target = u16::from_le_bytes([instruction[3], instruction[4]]);
            if instruction[0] != 0xa9 || instruction[2] != 0x8d || u32::from(target) != self.pc {
                return Err(invalid(source, "changed default-name initialization"));
            }
            instruction[1]
        } else {
            *bytes(self.image, self.pc, 1)?.first().unwrap()
        };
        if self.pc & 0xffff == 0xffff {
            return Err(invalid(self.pc, "text bank boundary"));
        }
        self.pc += 1;
        Ok(value)
    }
    fn word(&mut self) -> Result<u16, TextError> {
        Ok(u16::from_le_bytes([self.next()?, self.next()?]))
    }
    fn clear(&mut self) {
        self.page = empty_page();
        self.position = [0, 0];
        self.kana = false;
    }
    fn glyph(&mut self, text_source: u32, font_source: u32) -> Result<(), TextError> {
        let [x, y] = self.position.map(usize::from);
        if x + 16 > WIDTH || y + 16 > HEIGHT {
            return Err(invalid(text_source, "text exceeds qualified page geometry"));
        }
        let pixels = glyph_pixels(bytes(self.image, font_source, 64)?)?;
        for row in 0..16 {
            self.page.pixels[(y + row) * WIDTH + x..(y + row) * WIDTH + x + 16]
                .copy_from_slice(&pixels[row * 16..row * 16 + 16]);
        }
        self.page.glyphs.push(DialogueGlyph {
            text_source,
            font_source,
            position: self.position,
        });
        self.position[0] += 12;
        Ok(())
    }
    fn boundary(&mut self, source: u32, action: Acknowledgement) -> Result<(), TextError> {
        if self.page.glyphs.is_empty() || self.pages.len() >= 16 || !self.stack.is_empty() {
            return Err(invalid(
                source,
                "empty, excessive or nested acknowledgement page",
            ));
        }
        self.page.boundary_source = source;
        self.page.acknowledgement = action;
        self.pages
            .push(std::mem::replace(&mut self.page, empty_page()));
        self.position = [0, 0];
        self.kana = false;
        Ok(())
    }
    fn call(&mut self, index: u8) -> Result<(), TextError> {
        if ![0, 1, 7].contains(&index) || self.stack.len() >= 8 {
            return Err(invalid(
                self.pc - 1,
                "unsupported or recursive text subroutine",
            ));
        }
        let pointer = bytes(self.image, 0x92_c447 + u32::from(index) * 2, 2)?;
        let address = u32::from(u16::from_le_bytes([pointer[0], pointer[1]]));
        let destination = if index == 0 && address == 0x610 {
            address
        } else {
            0x92_0000 | address
        };
        self.stack.push(self.pc);
        self.pc = destination;
        Ok(())
    }
}
fn empty_page() -> DialoguePage {
    DialoguePage {
        pixels: vec![3; WIDTH * HEIGHT],
        glyphs: Vec::new(),
        boundary_source: 0,
        acknowledgement: Acknowledgement::End,
    }
}
fn decode(image: &[u8], source: u32) -> Result<Vec<DialoguePage>, TextError> {
    let mut d = Decoder {
        image,
        pc: source,
        stack: Vec::new(),
        kana: false,
        position: [0, 0],
        page: empty_page(),
        pages: Vec::new(),
    };
    for _ in 0..4096 {
        let at = d.pc;
        match d.next()? {
            c @ 0..=0x7f => d.glyph(
                at,
                0xb4_8000 + u32::from(c) * 64 + if d.kana { 0x2000 } else { 0 },
            )?,
            c @ 0x80..=0xbf => {
                let code = (u32::from(c & 0x3f) << 8) | u32::from(d.next()?);
                d.glyph(
                    at,
                    ((0xb5 + (code >> 9)) << 16) | (0x8000 + (code & 511) * 64),
                )?;
            }
            0xc0 | 0xc1 => {
                if !d.page.glyphs.is_empty() {
                    return Err(invalid(at, "unacknowledged page clear"));
                }
                d.clear();
            }
            0xc4 => {
                if d.next()? != 1 {
                    return Err(invalid(at, "unsupported font transformation"));
                }
            }
            // Timing/sound controls do not change content or acknowledgements.
            0xc5 | 0xc7 | 0xc8 => {
                d.next()?;
            }
            0xc6 => {
                if ![0, 4].contains(&d.next()?) {
                    return Err(invalid(at, "unsupported text palette"));
                }
                d.position[0] = d.position[0].next_multiple_of(8);
            }
            0xca => {
                if d.next()? != 5 {
                    return Err(invalid(at, "unsupported text memory write"));
                }
                d.word()?; // qualified speaker color at $7F060A, not event/progression RAM
            }
            0xcf => {
                if d.position[1] >= 32 {
                    return Err(invalid(at, "unsupported text scrolling"));
                }
                d.position = [0, d.position[1] + 16];
                d.kana = false;
            }
            0xd0 => d.kana = true,
            0xd1 => d.kana = false,
            0xd2 => {
                let index = d.next()?;
                d.call(index)?;
            }
            0xd3 => {
                d.boundary(at, Acknowledgement::End)?;
                return Ok(d.pages);
            }
            0xd4 => {
                d.pc = d
                    .stack
                    .pop()
                    .ok_or_else(|| invalid(at, "unacknowledged top-level end"))?;
            }
            0xd5 => d.boundary(at, Acknowledgement::Next)?,
            // Palette changes flush a pending half-tile even without color effects.
            0xdc => d.position[0] = d.position[0].next_multiple_of(8),
            _ => return Err(invalid(at, "unsupported text command (including choices)")),
        }
    }
    Err(invalid(d.pc, "text instruction budget exceeded"))
}
