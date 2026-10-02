//! Bounded Japanese house dialogue; no CPU, Unicode transcription or window engine.
//!
//! Pages retain native two-bit pixels (house background 3; Pandora may use 0). The caller
//! owns presentation and acknowledgement/progression. See `docs/house-dialogue.md`.
use crate::layout::{per_revision, Address};
use std::borrow::Cow;
use std::fmt;

pub mod pandora;
#[cfg(test)]
mod tests;
pub mod window;

/// The entry greeting, not the progression conversation.
pub const ENTRY_TEXT: u32 = 0x88_8fda;
/// The first interaction's prompt, before the event-level choice.
pub const FIRST_TEXT: u32 = 0x88_8ff0;
/// First interaction's second-option/cancel follow-up (not the initial prompt).
pub const RESIDENT_TEXT: u32 = 0x88_905a;
/// Entry, first prompt, its two follow-ups, repeat prompt and its two follow-ups.
pub const TEXT_SOURCES: [u32; 7] = [
    ENTRY_TEXT,
    FIRST_TEXT,
    RESIDENT_TEXT,
    0x88_90d9,
    0x88_9156,
    0x88_918c,
    0x88_91d6,
];

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

/// A text request: its source, and how it reads WRAM.
pub type Request<'a> = (u32, &'a dyn Fn(u16) -> Option<u8>);

/// Text-page boundary semantics; choice dispatch remains with the event caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Acknowledgement {
    /// Native `$D5`: clear this page and continue this same text invocation.
    Next,
    /// Native `$D3`: close the dialogue and return to the event caller.
    End,
    /// Native `$D7` after a page: once typed out, its pauses included, the
    /// window closes without a press (`$85:9D7F`), as "Four Hiballs
    /// appeared!" does (`$90:9367`).
    Closes,
    /// A window opened again after a page (`$C0`, `$C1`, `$C2`, `$DA`):
    /// once typed out, its pauses included, the next page follows without
    /// a press, as the guardian of tower 3's top does (`$90:9DAF`).
    Advances,
    /// Native top-level `$D4`: return immediately, retaining the visible page.
    /// Do NOT invent a Continue acknowledgement. The required prompts next enter
    /// an event-level choice on this same page; see `HouseDialogue::choice`.
    None,
}

/// Where the native engine opens the window. Content stays page-relative;
/// a host decides what to do with the anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// `$C0`/`$C1`: the standard window at tilemap base `$0504`, the bottom
    /// of the screen.
    Bottom,
    /// `$DF` (`$85:965F`): the standard window at tilemap base `$0104`, the
    /// top of the screen (Elle's farewell, `docs/underworld-end.md`).
    Top,
    /// `$DA` (`$85964D`): the standard window at the bottom, or at the top
    /// when the player stands in the lower half of the screen.
    AwayFromPlayer,
    /// `$C2` (`$85982D`): a window at an explicit tile column and row.
    Tile {
        /// Tile column of the window's left edge.
        column: u8,
        /// Tile row of the window's top edge.
        row: u8,
    },
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
    /// Frames after the page opens that the glyph appears: one per glyph at
    /// the message speed (`$C8`, `$0DBE`, 1 by default: `$06A4`), plus the
    /// pauses (`$C5`).
    pub tick: u16,
    /// The blip it sounds on port 3 (`$0DC6`, `$28` by default, set by
    /// `$C7`), or `None` while `$C7 FF` mutes it (`$85:9930`, `$85:9EE8`).
    pub sound: Option<u8>,
    /// Its text palette (`$0DBA`): 0 by default, 1 after `$C6 04` (a
    /// speaker's name), 2 after `$C6 08`; `$DC` resets it.
    pub palette: u8,
}

/// Immutable, precomposed page. Host bitmap presentation requires no original CPU.
#[derive(Debug, Clone)]
pub struct DialoguePage {
    pixels: Vec<u8>,
    background_index: u8,
    dimensions: [u16; 2],
    glyphs: Vec<DialogueGlyph>,
    boundary_source: u32,
    acknowledgement: Acknowledgement,
    placement: Placement,
    /// Frames the page takes to type out, pauses after its last glyph
    /// included.
    duration: u16,
    /// Colour 5, palette 1's first: the speaker's colour.
    speaker: u16,
    /// Where the text stood as the page ended: the prompt's cell.
    end: [u16; 2],
}
impl DialoguePage {
    /// Where the text stood as the page ended, relative to the content:
    /// a `$D5` page's prompt shows there (`$85:9D94`).
    #[must_use]
    pub const fn end(&self) -> [u16; 2] {
        self.end
    }
    /// The speaker's colour (colour 5): `$1B9F` as a window opens, or what
    /// `$CA 05` wrote (`$7F:060A`).
    #[must_use]
    pub const fn speaker(&self) -> crate::graphics::Bgr555 {
        crate::graphics::Bgr555::new(self.speaker)
    }
    /// Content width, without native frame/window effects.
    #[must_use]
    pub const fn width(&self) -> u16 {
        self.dimensions[0]
    }
    /// Content height, without native frame/window effects.
    #[must_use]
    pub const fn height(&self) -> u16 {
        self.dimensions[1]
    }
    /// Row-major native 2bpp color indices: 0..3; see `background_index()`.
    /// No color/font bytes or transcribed text are embedded in the library.
    #[must_use]
    pub fn indexed(&self) -> &[u8] {
        &self.pixels
    }
    /// Clear/background color index: 3 for house/raw font, 0 for Pandora's
    /// source-qualified transparent font mode. No palette/RGBA is implied.
    #[must_use]
    pub const fn background_index(&self) -> u8 {
        self.background_index
    }
    /// Per-glyph source metadata, also useful for independent qualification.
    #[must_use]
    pub fn glyphs(&self) -> &[DialogueGlyph] {
        &self.glyphs
    }
    /// Address of the actual `$D5`/`$D3`/top-level `$D4` ending this page.
    #[must_use]
    pub const fn boundary_source(&self) -> u32 {
        self.boundary_source
    }
    /// Whether/how this text page requires an acknowledgement before returning.
    #[must_use]
    pub const fn acknowledgement(&self) -> Acknowledgement {
        self.acknowledgement
    }
    /// Frames the page takes to type out, pauses after its last glyph
    /// included.
    #[must_use]
    pub const fn duration(&self) -> u16 {
        self.duration
    }
    /// The page with only its first `glyphs` glyphs drawn, as the
    /// typewriter shows it; `image` supplies the font. A page typed out is
    /// borrowed, not copied.
    #[must_use]
    pub fn typed(&self, image: &[u8], glyphs: usize) -> Cow<'_, [u8]> {
        if glyphs >= self.glyphs.len() {
            return Cow::Borrowed(&self.pixels);
        }
        let width = usize::from(self.dimensions[0]);
        let transparent = self.background_index == 0;
        let mut pixels = vec![self.background_index; self.pixels.len()];
        for glyph in &self.glyphs[..glyphs] {
            // Decoding drew each glyph once already, so none fails here.
            let _ = blit(&mut pixels, width, image, glyph, transparent);
        }
        Cow::Owned(pixels)
    }
    /// Where the native engine opened the window this page is shown in.
    #[must_use]
    pub const fn placement(&self) -> Placement {
        self.placement
    }
}

/// Source-derived option cursor and native selection/navigation results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DialogueOption {
    /// Source address of the ten-byte catalog entry.
    pub source: u32,
    /// Native result (1 or 2); cancellation returns 0 instead.
    pub result: u8,
    /// Cursor position relative to the retained page. The real option text is
    /// already drawn on this row; a host can use that row's bitmap as its button.
    pub position: [u16; 2],
    /// Native Up/Down/Left/Right destinations, expressed as option result IDs.
    pub neighbors: [Option<u8>; 4],
}

/// One of the admitted choice catalogs (0, 1 and the shop's `$0A`). Native initial option is result 1;
/// confirm is A/L, cancel is B/result 0. No option labels are fabricated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogueChoice {
    /// Native catalog ID (0 first interaction, 1 repeat interaction).
    pub catalog: u8,
    /// The two options, in native result order.
    pub options: [DialogueOption; 2],
}

/// Only the authenticated Japanese default-name room-B dialogue resources.
#[derive(Debug)]
pub struct HouseDialogue {
    dialogues: Vec<(u32, Vec<DialoguePage>)>,
    choices: [DialogueChoice; 2],
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
            dialogues: TEXT_SOURCES
                .into_iter()
                .map(|source| Ok((source, decode(image, source)?)))
                .collect::<Result<_, TextError>>()?,
            choices: [decode_choice(image, 0)?, decode_choice(image, 1)?],
        })
    }
    /// Decodes the pages at one source address discovered by the caller.
    ///
    /// `from_rom` compiles a fixed list of qualified sources. This decodes an
    /// arbitrary one, for a caller that found it some other way — an actor
    /// spawn record's script pointer, for instance. It validates the same
    /// commands, and also follows the button (`$E3`) and item-label (`$E4`)
    /// calls the Pandora texts use, still refusing labels outside their
    /// qualified set. An address that is not dialogue is rejected rather than
    /// returning noise, but a successful decode is not by itself evidence that
    /// the address *is* a text source.
    ///
    /// # Errors
    /// Rejects unsupported addresses, truncation and unsupported commands.
    pub fn decode_at(image: &[u8], source: u32) -> Result<Vec<DialoguePage>, TextError> {
        // With the Pandora profile's button and item-label calls, whose
        // tables refuse what is not qualified.
        decode_profile(image, source, true)
    }

    /// As [`Self::decode_at`], resolving WRAM through `read`, the engine
    /// byte at an address: the shop texts pick their parts (`$CE`) by the
    /// shop type (`$0DE8`) and the item (`$0DD0`).
    ///
    /// # Errors
    /// As [`Self::decode_at`], and an indexed call whose byte `read` does
    /// not know.
    pub fn decode_reading(
        image: &[u8],
        source: u32,
        read: impl Fn(u16) -> Option<u8>,
    ) -> Result<Vec<DialoguePage>, TextError> {
        decode_reading(image, source, true, &read)
    }

    /// Requests typed one after another into the window the first opens,
    /// each reading WRAM through its own `read` (names, `$CD` numbers);
    /// each request ends its own pages. The Records screen builds its page
    /// so. Not the Pandora profile: these scripts call no Pandora labels.
    ///
    /// # Errors
    /// As [`Self::decode_reading`].
    pub fn decode_requests(
        image: &[u8],
        requests: &[Request<'_>],
    ) -> Result<Vec<DialoguePage>, TextError> {
        let Some(&(first, read)) = requests.first() else {
            return Ok(Vec::new());
        };
        let mut decoder = decoder(image, first, false, read);
        let mut pages = Vec::new();
        for &(source, read) in requests {
            (decoder.pc, decoder.read) = (source, read);
            decoder.stack.clear();
            pages.extend(decoder.run()?);
        }
        Ok(pages)
    }

    /// Source-ID lookup. Page index is a stable zero-based key within the text ID;
    /// hosts can assign sequential numeric page keys scoped to the ROM identity.
    #[must_use]
    pub fn pages(&self, text_source: u32) -> Option<&[DialoguePage]> {
        self.dialogues
            .iter()
            .find(|(source, _)| *source == text_source)
            .map(|(_, pages)| pages.as_slice())
    }
    /// Decodes one admitted choice catalog directly from the ROM, for a
    /// script's `COP 1A`.
    ///
    /// # Errors
    /// Rejects catalogs other than 0, 1 and `$0A`, and malformed records.
    pub fn choice_at(image: &[u8], catalog: u8) -> Result<DialogueChoice, TextError> {
        decode_choice(image, catalog)
    }
    /// Native event-level choice catalog; no event flags or branches are executed.
    #[must_use]
    pub fn choice(&self, catalog: u8) -> Option<&DialogueChoice> {
        self.choices.get(usize::from(catalog))
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
fn decode_choice(image: &[u8], catalog: u8) -> Result<DialogueChoice, TextError> {
    // The Japanese house's catalogs 0 and 1, and the shop's confirm `$0A`;
    // the European scripts number theirs apart (the Elder asks 2), so there
    // every catalog whose records check out is taken.
    if per_revision(image, ![0, 1, 0x0A].contains(&catalog), false) {
        return Err(invalid(0, "unsupported choice catalog"));
    }
    let table = CHOICES
        .of(image)
        .ok_or_else(|| invalid(0, "no choices in this revision"))?;
    let pointer = bytes(image, table + u32::from(catalog) * 2, 2)?;
    let base = u16::from_le_bytes([pointer[0], pointer[1]]);
    let mut options = Vec::new();
    for index in 0..2_u8 {
        let source = 0x92_0000 + u32::from(base) + u32::from(index) * 10;
        let record = bytes(image, source, 10)?;
        let mut tile_offset = u16::from(record[0] & 0x7f) * 64 + u16::from(record[1]);
        if record[0] & 0x80 == 0 {
            tile_offset = tile_offset
                .checked_sub(per_revision(image, 0x0504, 0x04C4))
                .ok_or_else(|| invalid(source, "choice outside standard window"))?;
        }
        let position = [tile_offset % 64 * 4, tile_offset / 64 * 8];
        if tile_offset % 2 != 0 || position[0] >= 224 || position[1] >= standard(image)[1] {
            return Err(invalid(source, "choice outside page"));
        }
        let mut neighbors = [None; 4];
        for (direction, pair) in record[2..].chunks_exact(2).enumerate() {
            let target = u16::from_le_bytes([pair[0], pair[1]]);
            neighbors[direction] = if target == 0 {
                None
            } else if target == base {
                Some(1)
            } else if u32::from(target) == u32::from(base) + 10 {
                Some(2)
            } else {
                return Err(invalid(source, "unsupported choice neighbor"));
            };
        }
        options.push(DialogueOption {
            source,
            result: index + 1,
            position,
            neighbors,
        });
    }
    Ok(DialogueChoice {
        catalog,
        options: [options[0], options[1]],
    })
}

fn glyph_pixels(source: &[u8]) -> Result<[u8; 256], TextError> {
    if source.len() != 64 {
        return Err(invalid(0, "glyph must contain 64 bytes"));
    }
    Ok(crate::graphics::decode_glyph_2bpp(source))
}

struct Decoder<'a> {
    image: &'a [u8],
    pandora: bool,
    /// WRAM the text reads: the indexed calls' bytes, `$CD` numbers and
    /// names (`$0610` falls back to the default name's initialization).
    read: &'a dyn Fn(u16) -> Option<u8>,
    transparent: bool,
    dimensions: [u16; 2],
    pc: u32,
    stack: Vec<u32>,
    kana: bool,
    position: [u16; 2],
    placement: Placement,
    page: DialoguePage,
    pages: Vec<DialoguePage>,
    /// Frames into the page, frames per glyph, the blip and whether it is
    /// muted.
    tick: u16,
    speed: u16,
    /// The blip, `None` while muted, and the one unmuting restores.
    blip: (Option<u8>, u8),
    /// The glyphs' text palette, and colour 5.
    palette: u8,
    speaker: u16,
}
impl Decoder<'_> {
    fn next(&mut self) -> Result<u8, TextError> {
        let value = if let Some(value) = u16::try_from(self.pc)
            .ok()
            .filter(|&at| at < 0x8000)
            .and_then(self.read)
        {
            // Text in WRAM the engine reads: a slot's name (`$061C`).
            value
        } else if (0x610..0x616).contains(&self.pc) {
            // $878C97..8CB6 initializes the default name with six LDA #byte / STA
            // absolute pairs. Read the immediates, not a copied string or capture.
            let source = self.address(DEFAULT_NAME)? + (self.pc - 0x610) * 5;
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
    /// Timing and sound: `$C5` a pause, `$C7` the blip (0 unmutes, `FF`
    /// mutes, else picks one), `$C8` the message speed (`FF` the player's
    /// setting, 1).
    fn timing(&mut self, command: u8) -> Result<(), TextError> {
        let operand = self.next()?;
        match (command, operand) {
            (0xc5, frames) => self.tick = self.tick.saturating_add(u16::from(frames)),
            (0xc7, 0) => self.blip.0 = Some(self.blip.1),
            (0xc7, 0xff) => self.blip.0 = None,
            (0xc7, sound) => self.blip = (self.blip.0.map(|_| sound), sound),
            (_, 0xff) => self.speed = 1,
            (_, speed) => self.speed = u16::from(speed),
        }
        Ok(())
    }
    fn clear(&mut self) {
        self.page = blank_page(self.dimensions, self.transparent, self.placement);
        self.position = [0, 0];
        self.kana = false;
        self.tick = 0;
    }
    fn glyph(&mut self, text_source: u32, font_source: u32) -> Result<(), TextError> {
        let [x, y] = self.position.map(usize::from);
        let [width, height] = self.dimensions.map(usize::from);
        // Glyphs advance by 12 pixels although their cells are 16 wide.
        // The final glyph of a narrow English window, or of a full line
        // ("A SWARM OF HIBALLS!", European `$90:9E97`), can extend four
        // pixels beyond its content; clip that overlap, not the page.
        if x + 8 > width || y + 16 > height {
            return Err(invalid(text_source, "text exceeds qualified page geometry"));
        }
        let glyph = DialogueGlyph {
            text_source,
            font_source,
            position: self.position,
            tick: self.tick,
            sound: self.blip.0,
            palette: self.palette,
        };
        blit(
            &mut self.page.pixels,
            width,
            self.image,
            &glyph,
            self.transparent,
        )?;
        self.page.glyphs.push(glyph);
        self.tick = self.tick.saturating_add(self.speed);
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
        self.page.speaker = self.speaker;
        self.page.end = self.position;
        // A glyph at speed 0 shows on the tick it shares with the one before.
        let last = self.page.glyphs.last().map_or(0, |glyph| glyph.tick + 1);
        self.page.duration = self.tick.max(last);
        self.pages.push(std::mem::replace(
            &mut self.page,
            blank_page(self.dimensions, self.transparent, self.placement),
        ));
        self.position = [0, 0];
        self.kana = false;
        self.tick = 0;
        Ok(())
    }
    /// `$C0`/`$C1` open the standard window; `$DA` (`$85964D`) opens it at
    /// whichever of top and bottom the player is not standing in, `$DF` at
    /// the top.
    fn standard_window(&mut self, at: u32, command: u8) -> Result<(), TextError> {
        self.advance(at)?;
        if command != 0xc0 {
            self.dimensions = standard(self.image);
        }
        // `$C0`, `$C1` and `$DA` reset the speaker's colour.
        self.speaker = SPEAKER;
        self.placement = match command {
            0xda => Placement::AwayFromPlayer,
            0xdf => Placement::Top,
            _ => Placement::Bottom,
        };
        self.clear();
        Ok(())
    }
    /// A window opened over a page ends it: the next follows by itself.
    fn advance(&mut self, at: u32) -> Result<(), TextError> {
        if self.page.glyphs.is_empty() {
            return Ok(());
        }
        self.boundary(at, Acknowledgement::Advances)
    }
    fn custom_window(&mut self, at: u32) -> Result<(), TextError> {
        let [column, row, width, height] = [self.next()?, self.next()?, self.next()?, self.next()?];
        // $85982D: tile column, row, width and height of a window on the
        // 32x28-tile screen. Content is `width` tiles by `height / 2` glyph
        // rows; a window with no glyph row or off the screen is refused.
        let [right, bottom] = [(column, width), (row, height)]
            .map(|(origin, extent)| u16::from(origin) + u16::from(extent));
        if width == 0 || height < 2 || right > 32 || bottom > 28 {
            return Err(invalid(at, "unsupported window layout"));
        }
        self.advance(at)?;
        self.dimensions = [u16::from(width) * 8, u16::from(height / 2) * 16];
        self.placement = Placement::Tile { column, row };
        self.clear();
        Ok(())
    }
    /// `$C6 nn` selects text palette `nn / 4` (0, 1 a speaker's name, 2 the
    /// shop's names); `$DC` returns to palette 0. Both flush a pending
    /// half-tile. Pages keep indices; each glyph keeps its palette.
    fn text_palette(&mut self, at: u32, command: u8) -> Result<(), TextError> {
        self.palette = if command == 0xdc {
            0
        } else {
            let palette = self.next()?;
            if ![0, 4, 8].contains(&palette) {
                return Err(invalid(at, "unsupported text palette"));
            }
            palette >> 2
        };
        self.position[0] = self.position[0].next_multiple_of(8);
        Ok(())
    }
    /// `$85:9B93`: calls entry `[address]` of a table in the text's bank,
    /// returning after the four operand bytes.
    fn indexed_call(&mut self, at: u32) -> Result<(), TextError> {
        let address = self.word()?;
        let table = self.word()?;
        let index =
            (self.read)(address).ok_or_else(|| invalid(at, "unresolved indexed text call"))?;
        let bank = at & 0xff_0000;
        let entry = bytes(
            self.image,
            bank | (u32::from(table) + u32::from(index) * 2),
            2,
        )?;
        self.enter(bank | u32::from(u16::from_le_bytes([entry[0], entry[1]])))
    }
    /// `$CD addr`: the word at `addr` in decimal, without leading zeros;
    /// digit `d` is glyph `$73 + d` (European `$63 + d`).
    fn number(&mut self, at: u32) -> Result<(), TextError> {
        let address = self.word()?;
        let byte =
            |address: u16| (self.read)(address).ok_or_else(|| invalid(at, "unresolved number"));
        let value = u16::from_le_bytes([byte(address)?, byte(address.wrapping_add(1))?]);
        let font = self.address(FONT)?;
        let zero = per_revision(self.image, 0x73, 0x63);
        for digit in value.to_string().bytes() {
            self.glyph(at, font + (zero + u32::from(digit - b'0')) * 64)?;
        }
        Ok(())
    }
    /// `$D8 addr`: the BCD word at `addr & $7FFF` without leading zeros,
    /// as the chests print gems (`$D8 $89CB`, `docs/chests.md`). Bit 15's
    /// meaning is not traced.
    fn bcd_number(&mut self, at: u32) -> Result<(), TextError> {
        let address = self.word()? & 0x7FFF;
        let byte =
            |address: u16| (self.read)(address).ok_or_else(|| invalid(at, "unresolved number"));
        let value = u16::from_le_bytes([byte(address)?, byte(address.wrapping_add(1))?]);
        let font = self.address(FONT)?;
        let zero = per_revision(self.image, 0x73, 0x63);
        for digit in format!("{value:X}").bytes() {
            let digit = u32::from(digit.wrapping_sub(b'0')).min(9);
            self.glyph(at, font + (zero + digit) * 64)?;
        }
        Ok(())
    }
    /// Whether the image is the European one: its dictionary calls, and no
    /// katakana.
    fn europe(&self) -> bool {
        per_revision(self.image, false, true)
    }
    /// This revision's value of `address`.
    fn address(&self, address: Address) -> Result<u32, TextError> {
        address
            .of(self.image)
            .ok_or_else(|| invalid(self.pc, "no address in this revision"))
    }
    /// The European dictionary calls: `$E4`/`$E5 nn` word `nn` of
    /// `$92:C793`, `$E6 nn` of `$92:D2BB`, returning at `$D4` (`$85:9F62`).
    fn dictionary(&mut self, command: u8) -> Result<(), TextError> {
        let index = self.next()?;
        let table = EUROPEAN_WORDS[usize::from(command == 0xe6)];
        let pointer = bytes(self.image, table + u32::from(index) * 2, 2)?;
        self.enter(0x92_0000 | u32::from(u16::from_le_bytes([pointer[0], pointer[1]])))
    }
    fn label_call(&mut self) -> Result<(), TextError> {
        let index = self.next()?;
        let pointer = bytes(self.image, 0x92_c5e7 + u32::from(index) * 2, 2)?;
        self.enter(0x92_0000 | u32::from(u16::from_le_bytes([pointer[0], pointer[1]])))
    }
    fn enter(&mut self, destination: u32) -> Result<(), TextError> {
        if self.stack.len() >= 8 {
            return Err(invalid(
                self.pc - 1,
                "unsupported or recursive text subroutine",
            ));
        }
        if destination >= 0x8000 {
            bytes(self.image, destination, 1)?;
        }
        self.stack.push(self.pc);
        self.pc = destination;
        Ok(())
    }
    fn call(&mut self, index: u8) -> Result<(), TextError> {
        // $859C7A: the `$92C447` word table. Index 0 is the name in WRAM
        // (`read`, else its initialization), `$0D` a slot's; the rest are ROM speaker-prefix
        // subroutines, whose commands are validated like any other text. The
        // table holds 25 entries, ending where its first subroutine begins.
        if index >= 25 {
            return Err(invalid(self.pc - 1, "text subroutine outside table"));
        }
        let pointer = bytes(self.image, self.address(NAMES)? + u32::from(index) * 2, 2)?;
        let address = u32::from(u16::from_le_bytes([pointer[0], pointer[1]]));
        let destination = if (0x600..0x800).contains(&address) {
            // A name in WRAM: the current one (`$0610`), a slot's (`$061C`).
            address
        } else if address >= 0x8000 {
            0x92_0000 | address
        } else {
            return Err(invalid(self.pc - 1, "text subroutine outside ROM"));
        };
        self.enter(destination)
    }
}
/// Colour 5 as a standard window opens: the default speaker yellow.
const SPEAKER: u16 = 0x1B9F;
/// The font: single-byte glyphs from here, katakana `+$2000`
/// (`docs/european-text.md`).
const FONT: Address = Address::both(0xB4_8000, 0xB6_8000);
/// `$D2`'s table of names and speaker prefixes (`$85:9C7A`).
const NAMES: Address = Address::both(0x92_C447, 0x92_C5CD);
/// The default name's initialization, five bytes an `LDA #`/`STA` pair.
const DEFAULT_NAME: Address = Address::both(0x87_8C99, 0x87_8C8E);
/// The choice catalogs' records.
const CHOICES: Address = Address::both(0x92_C259, 0x92_C407);
/// The European dictionaries: `$E4`/`$E5` and `$E6` call word `nn`.
const EUROPEAN_WORDS: [u32; 2] = [0x92_C793, 0x92_D2BB];

fn empty_page(image: &[u8]) -> DialoguePage {
    blank_page(standard(image), false, Placement::Bottom)
}
/// The standard window's content: three lines of text, four in European.
fn standard(image: &[u8]) -> [u16; 2] {
    [224, per_revision(image, 48, 64)]
}
fn blank_page(dimensions: [u16; 2], transparent: bool, placement: Placement) -> DialoguePage {
    let background_index = if transparent { 0 } else { 3 };
    DialoguePage {
        pixels: vec![background_index; usize::from(dimensions[0]) * usize::from(dimensions[1])],
        background_index,
        dimensions,
        glyphs: Vec::new(),
        boundary_source: 0,
        acknowledgement: Acknowledgement::End,
        placement,
        duration: 0,
        speaker: SPEAKER,
        end: [0, 0],
    }
}
/// Draws a glyph into a page's pixels, `transparent` clearing its colour 3
/// (`$85947F`: a' = a XOR (a AND b), b' = b XOR (a AND b)).
fn blit(
    pixels: &mut [u8],
    width: usize,
    image: &[u8],
    glyph: &DialogueGlyph,
    transparent: bool,
) -> Result<(), TextError> {
    let mut shape = glyph_pixels(bytes(image, glyph.font_source, 64)?)?;
    if transparent {
        for pixel in &mut shape {
            if *pixel == 3 {
                *pixel = 0;
            }
        }
    }
    let [x, y] = glyph.position.map(usize::from);
    let visible = 16.min(width - x);
    for row in 0..16 {
        pixels[(y + row) * width + x..(y + row) * width + x + visible]
            .copy_from_slice(&shape[row * 16..row * 16 + visible]);
    }
    Ok(())
}
fn decode(image: &[u8], source: u32) -> Result<Vec<DialoguePage>, TextError> {
    decode_profile(image, source, false)
}
fn decode_profile(
    image: &[u8],
    source: u32,
    pandora: bool,
) -> Result<Vec<DialoguePage>, TextError> {
    decode_reading(image, source, pandora, &|_| None)
}
fn decode_reading(
    image: &[u8],
    source: u32,
    pandora: bool,
    read: &dyn Fn(u16) -> Option<u8>,
) -> Result<Vec<DialoguePage>, TextError> {
    decoder(image, source, pandora, read).run()
}
fn decoder<'a>(
    image: &'a [u8],
    source: u32,
    pandora: bool,
    read: &'a dyn Fn(u16) -> Option<u8>,
) -> Decoder<'a> {
    Decoder {
        image,
        pandora,
        read,
        transparent: false,
        dimensions: standard(image),
        pc: source,
        stack: Vec::new(),
        kana: false,
        position: [0, 0],
        placement: Placement::Bottom,
        page: empty_page(image),
        pages: Vec::new(),
        tick: 0,
        speed: 1,
        blip: (Some(0x28), 0x28),
        palette: 0,
        speaker: SPEAKER,
    }
}
impl Decoder<'_> {
    /// Runs one request from `pc` to its end.
    fn run(&mut self) -> Result<Vec<DialoguePage>, TextError> {
        for _ in 0..4096 {
            let at = self.pc;
            match self.next()? {
                c @ 0..=0x7f => {
                    let font = self.address(FONT)?;
                    self.glyph(
                        at,
                        font + u32::from(c) * 64 + if self.kana { 0x2000 } else { 0 },
                    )?;
                }
                // The European font has no katakana or two-byte glyphs.
                0x80..=0xbf | 0xd0 if self.europe() => {
                    return Err(invalid(at, "no such glyph in the European font"))
                }
                c @ 0x80..=0xbf => {
                    let code = (u32::from(c & 0x3f) << 8) | u32::from(self.next()?);
                    self.glyph(
                        at,
                        ((0xb5 + (code >> 9)) << 16) | (0x8000 + (code & 511) * 64),
                    )?;
                }
                c @ (0xc0 | 0xc1 | 0xda | 0xdf) => self.standard_window(at, c)?,
                // `$85:9A09`: the text goes on at a word in its bank.
                0xcb => {
                    let target = self.word()?;
                    self.pc = (at & 0xff_0000) | u32::from(target);
                }
                0xc2 => self.custom_window(at)?,
                // `$C4 0` draws the page without its window: the friends' line as
                // the door breaks, and the Pandora guide's.
                0xc4 => match self.next()? {
                    1 => self.transparent = false,
                    0 => self.transparent = true,
                    _ => return Err(invalid(at, "unsupported font transformation")),
                },
                command @ (0xc5 | 0xc7 | 0xc8) => self.timing(command)?,
                command @ (0xc6 | 0xdc) => self.text_palette(at, command)?,
                0xca => {
                    if self.next()? != 5 {
                        return Err(invalid(at, "unsupported text memory write"));
                    }
                    // The speaker's colour at `$7F:060A`, not event RAM.
                    self.speaker = self.word()?;
                }
                // $859A13/$859ECA save the banked return after a three-byte pointer.
                0xcc => {
                    let address = u32::from(self.word()?);
                    let destination = address | (u32::from(self.next()?) << 16);
                    self.enter(destination)?;
                }
                0xce => self.indexed_call(at)?,
                // `$C3 xx yy`: to line `yy`, `xx` map bytes (4 pixels each) in.
                0xc3 => {
                    let [column, line] = [self.next()?, self.next()?];
                    self.position = [u16::from(column) * 4, u16::from(line) * 16];
                }
                0xcd => self.number(at)?,
                0xd8 => self.bcd_number(at)?,
                0xcf => {
                    if self.position[1] + 16 >= self.dimensions[1] {
                        return Err(invalid(at, "unsupported text scrolling"));
                    }
                    self.position = [0, self.position[1] + 16];
                    self.kana = false;
                }
                0xd0 => self.kana = true,
                0xd1 => self.kana = false,
                0xd2 => {
                    let index = self.next()?;
                    self.call(index)?;
                }
                0xd3 => {
                    self.boundary(at, Acknowledgement::End)?;
                    return Ok(std::mem::take(&mut self.pages));
                }
                0xd4 => {
                    if let Some(caller) = self.stack.pop() {
                        self.pc = caller;
                    } else if self.page.glyphs.is_empty() && !self.pages.is_empty() {
                        // `$D5 $D4`: the last page was acknowledged; return to
                        // the script with the window open for its next request.
                        return Ok(std::mem::take(&mut self.pages));
                    } else {
                        self.boundary(at, Acknowledgement::None)?;
                        return Ok(std::mem::take(&mut self.pages));
                    }
                }
                0xd5 => self.boundary(at, Acknowledgement::Next)?,
                // `$85:9D7F` clears the window and closes it without a press (a
                // two-pass latch on `$0DA4` bit 7): at once on an empty page,
                // as the spear's presentation uses it, else once the page is
                // typed out.
                0xd7 if self.stack.is_empty() => {
                    if !self.page.glyphs.is_empty() {
                        self.boundary(at, Acknowledgement::Closes)?;
                    }
                    return Ok(std::mem::take(&mut self.pages));
                }
                0xe3 if self.pandora => {
                    let mask = self.word()?;
                    let destination = pandora::default_button_source(self.image, mask)?;
                    self.enter(destination)?;
                }
                command @ 0xe4..=0xe6 if self.europe() => self.dictionary(command)?,
                // $859725 calls the bank-$92 item-label pointer table, returning via D4.
                0xe4 if self.pandora => self.label_call()?,
                _ => return Err(invalid(at, "unsupported text command (including choices)")),
            }
        }
        Err(invalid(self.pc, "text instruction budget exceeded"))
    }
}
