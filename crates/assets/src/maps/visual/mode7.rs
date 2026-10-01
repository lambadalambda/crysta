//! The world map's Mode 7 view ([notes](../../../../docs/world-map-mode7.md)):
//! the per-line tables the setup (`$87:990B`) hands to HDMA. Each line scales
//! the plane by its own factor (M7A = M7D, no rotation); the first lines
//! show the plane mirrored as sky, subtracted from a colour gradient; a fog
//! (a fixed colour subtracted) thins toward the bottom.

use super::VisualMapError;
use crate::graphics::Bgr555;
use crate::layout::per_revision;

/// Lines on screen.
pub const LINES: usize = 224;

/// HDMA tables in bank `$87`: the indirect M7A/M7D list, the colour math
/// (`CGADSUB`, `COLDATA`), the backdrop (`CGADD` twice, `CGDATA` twice),
/// `M7SEL` and the main/sub screens (`TM`, `TS`). European: `$43` lower.
const TABLES: [u32; 5] = [0x87_9992, 0x87_9B6B, 0x87_9B9C, 0x87_999C, 0x87_9C24];
const EUROPE_SHIFT: u32 = 0x43;

/// The view's per-line tables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mode7View {
    /// M7A = M7D per line: the plane's scale, 256 = 1.
    pub scale: [i16; LINES],
    /// The fixed colour's intensity subtracted on each line (`COLDATA`).
    pub fade: [u8; LINES],
    /// The backdrop colour per line (CGRAM 0).
    pub backdrop: [Bgr555; LINES],
    /// Lines of sky: the plane mirrored and 2x2 mosaic on the sub screen,
    /// subtracted from the backdrop (`M7SEL` V-flip).
    pub sky: usize,
    /// The first line sprites show on the main screen; above it, from the
    /// sky down, they only darken the plane (`TM`/`TS`).
    pub sprites_from: usize,
}

fn bytes(image: &[u8], at: u32, count: usize) -> Result<&[u8], VisualMapError> {
    let start = usize::try_from(((at >> 16) & 0x3F) << 16 | (at & 0xFFFF))
        .map_err(|_| VisualMapError::Unsupported("Mode 7 table address"))?;
    image
        .get(start..start + count)
        .ok_or(VisualMapError::Unsupported("Mode 7 table outside the ROM"))
}

/// An HDMA table's data per line, `width` bytes each; `indirect` tables
/// point at their data in the same bank. Lines past the table's end keep
/// the last data, as the registers do.
fn hdma(
    image: &[u8],
    at: u32,
    width: usize,
    indirect: bool,
) -> Result<Vec<Vec<u8>>, VisualMapError> {
    let mut lines: Vec<Vec<u8>> = Vec::with_capacity(LINES);
    let mut at = at;
    while lines.len() < LINES {
        let count = bytes(image, at, 1)?[0];
        if count == 0 {
            break;
        }
        // `$80` is 128 lines of one value, not a repeat of none.
        let (repeat, lines_here) = match count {
            0x80 => (false, 128),
            _ => (count & 0x80 != 0, usize::from(count & 0x7F)),
        };
        let (mut data, step) = if indirect {
            let pointer = bytes(image, at + 1, 2)?;
            (
                at & 0xFF_0000 | u32::from(u16::from_le_bytes([pointer[0], pointer[1]])),
                3,
            )
        } else {
            (at + 1, 1 + if repeat { width * lines_here } else { width })
        };
        for _ in 0..lines_here {
            lines.push(bytes(image, data, width)?.to_vec());
            if repeat {
                data += u32::try_from(width).unwrap_or(0);
            }
        }
        at += u32::try_from(step).unwrap_or(0);
    }
    let last = lines.last().cloned().unwrap_or_else(|| vec![0; width]);
    lines.resize(LINES, last);
    Ok(lines)
}

/// The first line whose data differs from line 0's.
fn split(lines: &[Vec<u8>]) -> usize {
    lines
        .iter()
        .position(|line| *line != lines[0])
        .unwrap_or(LINES)
}

impl Mode7View {
    /// Reads the view's tables.
    ///
    /// # Errors
    /// Refuses a table outside the image.
    pub fn from_rom(image: &[u8]) -> Result<Self, VisualMapError> {
        let shift = per_revision(image, 0, EUROPE_SHIFT);
        let [scale, math, backdrop, select, screens] = TABLES.map(|table| table - shift);
        let scale = hdma(image, scale, 2, true)?;
        let math = hdma(image, math, 2, false)?;
        let backdrop = hdma(image, backdrop, 4, false)?;
        let screens = hdma(image, screens, 2, false)?;
        Ok(Self {
            scale: std::array::from_fn(|k| i16::from_le_bytes([scale[k][0], scale[k][1]])),
            fade: std::array::from_fn(|k| math[k][1] & 0x1F),
            backdrop: std::array::from_fn(|k| {
                Bgr555::new(u16::from_le_bytes([backdrop[k][2], backdrop[k][3]]))
            }),
            sky: split(&hdma(image, select, 1, false)?),
            sprites_from: screens
                .iter()
                .position(|line| line[0] & 0x10 != 0)
                .unwrap_or(LINES),
        })
    }
}
