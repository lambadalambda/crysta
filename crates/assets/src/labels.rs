//! Labels the room-label engine (`$85:869B`) draws as 16×16 sprites in the
//! dialogue font: the area titles shown on entering a map, and the shop's
//! item names.
//!
//! A title follows the map's spawn list: `$80:F4AC` keeps the byte after
//! the resolved `$FF` at `$7F:0806`, and the controller `$85:8008` types it
//! unless flag `$14` is set or it is empty (`D4`). Its letters then fly in
//! from the upper right, hold, and fly away to the upper left: effect 3 of
//! `$B0:DE49` (`DE 03 04`, text `$85:8000`), each letter 4 frames after the
//! one before. The European engine is the same with its own font, palette,
//! effect scripts and dictionary calls (`docs/european-text.md`).

use crate::graphics::Bgr555;
use crate::layout::{self, per_revision};
use crate::maps::actors::SpawnList;
use crate::maps::scripts::EventFlags;
use crate::shops::{read, ShopError};

/// The dialogue font (`$B4:8000`, katakana `+$2000`), European `$B6:8000`.
const FONT: u32 = 0xB4_8000;
/// The two-byte codes' first font bank: `$B5`, European `$B6` (`ADC #$00B6`
/// at `$85:86E9`).
const WIDE_FONT_BANKS: (u32, u32) = (0xB5, 0xB6);
/// `E4 nn` calls label `nn` of this table in bank `$92`, returning at `D4`.
/// The European engine calls its dictionaries instead: `E4`/`E5` word `nn`
/// of `$92:C793`, `E6` of `$92:D2BB` (`$85:8D01`, `$85:8D25`, `$85:8D49`),
/// the text engine's.
const LABELS: (u32, u32) = (0x92_C5E7, 0x92_C793);
const EUROPEAN_WORDS: u32 = 0x92_D2BB;
/// Flag `$14` hides the titles (`COP 08 $8014`).
const NO_TITLES: u16 = 0x14;
/// The effect scripts (`$B0:DE49`, European `$B2:E26C`), and the titles'
/// effect.
const EFFECTS: u32 = 0xB0_DE49;
const TITLE_EFFECT: u32 = 3;
/// Frames between one letter's start and the next's.
const DELAY: i64 = 4;
/// The titles' resting row and their letters' pitch.
const ROW: i32 = 48;
const PITCH: i32 = 12;

/// A 16×16 glyph: 0 clear, 1 and 2 drawn with OBJ palette 2's colours.
pub type Glyph = [u8; 256];

/// `$B2:8B58`: OBJ palette 2, the labels' colours (`$86:C3B8` copies it);
/// European `$B4:90BB`.
const PALETTE: u32 = 0xB2_8B58;

/// The address in `image`'s revision of what sits at Japanese `japan`
/// ([`layout::at`]).
pub(crate) fn located(image: &[u8], japan: u32) -> Result<u32, ShopError> {
    layout::at(image, japan).ok_or(ShopError::Invalid(japan, "unrecorded in this revision"))
}

/// The labels' 16 colours (OBJ palette 2).
///
/// # Errors
/// Refuses a read outside the image.
pub fn label_palette(image: &[u8]) -> Result<[Bgr555; 16], ShopError> {
    let bytes = read(image, located(image, PALETTE)?, 32)?;
    Ok(std::array::from_fn(|i| {
        Bgr555::new(u16::from_le_bytes([bytes[i * 2], bytes[i * 2 + 1]]))
    }))
}

/// A label's glyphs, from `at` to its `D4`: glyph codes, the katakana
/// switches `D0`/`D1`, and `E4 nn` calls (European `E4`-`E6`).
///
/// # Errors
/// Refuses a label that leaves the image, does not end, or holds another
/// code.
pub fn label_glyphs(image: &[u8], at: u32) -> Result<Vec<Glyph>, ShopError> {
    let font = located(image, FONT)?;
    let europe = per_revision(image, false, true);
    let labels = per_revision(image, LABELS.0, LABELS.1);
    let wide_bank = per_revision(image, WIDE_FONT_BANKS.0, WIDE_FONT_BANKS.1);
    let mut glyphs = Vec::new();
    let (mut at, mut kana, mut returns) = (at, false, Vec::new());
    for _ in 0..64 {
        let code = read(image, at, 1)?[0];
        at += 1;
        let source = match code {
            0xD4 => match returns.pop() {
                Some(back) => {
                    at = back;
                    continue;
                }
                None => return Ok(glyphs),
            },
            0xD0 | 0xD1 => {
                kana = code == 0xD0;
                continue;
            }
            0xE4..=0xE6 if returns.len() < 4 && (europe || code == 0xE4) => {
                let index = read(image, at, 1)?[0];
                returns.push(at + 1);
                let table = if code == 0xE6 { EUROPEAN_WORDS } else { labels };
                let pointer = read(image, table + u32::from(index) * 2, 2)?;
                at = 0x92_0000 | u32::from(u16::from_le_bytes([pointer[0], pointer[1]]));
                continue;
            }
            0..=0x7F => font + u32::from(code) * 64 + if kana { 0x2000 } else { 0 },
            0x80..=0xBF => {
                let code = u32::from(code & 0x3F) << 8 | u32::from(read(image, at, 1)?[0]);
                at += 1;
                ((wide_bank + (code >> 9)) << 16) | (0x8000 + (code & 511) * 64)
            }
            _ => return Err(ShopError::Invalid(at - 1, "unsupported label code")),
        };
        glyphs.push(glyph(read(image, source, 64)?));
    }
    Err(ShopError::Invalid(at, "unending label"))
}

/// A map's area title under `events`, if it has one.
///
/// # Errors
/// Refuses a spawn list that does not resolve and a title that does not
/// decode.
pub fn area_title(
    image: &[u8],
    map: u16,
    events: EventFlags<'_>,
) -> Result<Option<Vec<Glyph>>, ShopError> {
    if events.get(NO_TITLES) == Some(true) {
        return Ok(None);
    }
    let end = SpawnList::resolved_end(image, map, events)
        .map_err(|_| ShopError::Invalid(u32::from(map), "unresolved spawn list"))?;
    let at = u32::try_from(end).map_err(|_| ShopError::Truncated(0))? | 0x80_0000;
    let glyphs = label_glyphs(image, at)?;
    Ok((!glyphs.is_empty()).then_some(glyphs))
}

/// A 16×16 glyph with colour 3 cleared.
fn glyph(source: &[u8]) -> Glyph {
    crate::graphics::decode_glyph_2bpp(source).map(|index| if index == 3 { 0 } else { index })
}

/// The titles' letter motion: two scripts of (frames, velocity) per axis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TitleMotion {
    x: Vec<(u16, i16)>,
    y: Vec<(u16, i16)>,
}

impl TitleMotion {
    /// Decodes effect 3's scripts: a header word, then `(frames - 1,
    /// velocity)` word pairs up to a count of `$FFFF`.
    ///
    /// # Errors
    /// Refuses scripts outside the image or unending.
    pub fn from_rom(image: &[u8]) -> Result<Self, ShopError> {
        let effects = located(image, EFFECTS)?;
        let script = |axis: u32| -> Result<Vec<(u16, i16)>, ShopError> {
            let offset = read(image, effects + (TITLE_EFFECT * 2 + axis) * 2, 2)?;
            let start = effects + u32::from(u16::from_le_bytes([offset[0], offset[1]]));
            let mut steps = Vec::new();
            for index in 0..64 {
                let pair = read(image, start + 2 + index * 4, 4)?;
                let count = u16::from_le_bytes([pair[0], pair[1]]);
                if count == 0xFFFF {
                    return Ok(steps);
                }
                steps.push((count + 1, i16::from_le_bytes([pair[2], pair[3]])));
            }
            Err(ShopError::Invalid(start, "unending effect script"))
        };
        Ok(Self {
            x: script(0)?,
            y: script(1)?,
        })
    }

    /// Frames one letter's script runs.
    #[must_use]
    pub fn length(&self) -> u32 {
        self.x.iter().map(|&(frames, _)| u32::from(frames)).sum()
    }

    /// The visible letters of an `n`-letter title `t` frames after the
    /// effect began: index and screen position. Letter `i` starts `4(i +
    /// 1)` frames in, from its place plus 256 pixels right; a letter is
    /// shown only while `0 <= x < $110` and `-16 < y < 240` (`$80:ECEA`).
    #[must_use]
    pub fn positions(&self, n: usize, t: i64) -> Vec<(usize, i32, i32)> {
        let (Ok(count), length) = (i32::try_from(n), i64::from(self.length())) else {
            return Vec::new();
        };
        (0..n)
            .zip(0..count)
            .filter_map(|(index, i)| {
                let steps = t - DELAY * (i64::from(i) + 1) + 1;
                if steps > length {
                    return None;
                }
                let x = 128 - 6 * count + PITCH * i + 256 + moved(&self.x, steps);
                let y = ROW + moved(&self.y, steps);
                ((0..0x110).contains(&x) && y > -16 && y < 240).then_some((index, x, y))
            })
            .collect()
    }
}

/// Where a script has moved after `steps` frames.
fn moved(script: &[(u16, i16)], steps: i64) -> i32 {
    let mut left = steps.max(0);
    let mut total = 0;
    for &(frames, velocity) in script {
        let taken = left.min(i64::from(frames));
        total += taken * i64::from(velocity);
        left -= taken;
    }
    i32::try_from(total).unwrap_or(0)
}
