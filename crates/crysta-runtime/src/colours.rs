//! The OBJ colours scripts load (`COP 5A`, `$80:9AEB`): colours from the ROM
//! into the palette buffer (`$7F:0600`), from there into CGRAM. Only the
//! OBJ half (`$80..$FF`) is kept here, over each art's own palette; a map
//! load starts with none.

use assets::graphics::Bgr555;

/// The first OBJ colour's CGRAM index.
const OBJ: usize = 0x80;

/// The OBJ colours scripts loaded since the map loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjColours([Option<Bgr555>; 128]);

impl Default for ObjColours {
    fn default() -> Self {
        Self([None; 128])
    }
}

impl ObjColours {
    /// `COP 5A bank word index count` with its operands at `operands`:
    /// `count` colours from `bank:word` to CGRAM `index`. Returns whether
    /// the operands and the source are in the image. The source is read
    /// as ROM: a WRAM one (`$7E`, `$7F`) is not modelled.
    pub fn load(&mut self, image: &[u8], operands: usize) -> bool {
        let Some(&[bank, low, high, index, count]) = image.get(operands..operands + 5) else {
            return false;
        };
        let source = usize::from(bank & 0x3F) << 16 | usize::from(u16::from_le_bytes([low, high]));
        let (index, count) = (usize::from(index), usize::from(count));
        let Some(bytes) = image.get(source..source + 2 * count) else {
            return false;
        };
        for (at, colour) in (index..index + count).zip(bytes.chunks_exact(2)) {
            if let Some(slot) = at.checked_sub(OBJ).and_then(|at| self.0.get_mut(at)) {
                *slot = Some(Bgr555::new(u16::from_le_bytes([colour[0], colour[1]])));
            }
        }
        true
    }

    /// Sets CGRAM `index` when it is an OBJ colour; a background one is
    /// not kept.
    pub fn set(&mut self, index: u8, colour: Bgr555) {
        if let Some(slot) = usize::from(index)
            .checked_sub(OBJ)
            .and_then(|at| self.0.get_mut(at))
        {
            *slot = Some(colour);
        }
    }

    /// The colour a script loaded at CGRAM `index`, `$80` to `$FF`.
    #[must_use]
    pub fn get(&self, index: u8) -> Option<Bgr555> {
        usize::from(index)
            .checked_sub(OBJ)
            .and_then(|at| self.0.get(at).copied().flatten())
    }
}

/// `COP EC`'s blend (`$80:BA35`): each channel goes from `from` toward
/// `to` by the difference times `step / total`, the share rounded down.
#[must_use]
pub fn blend(from: Bgr555, to: Bgr555, step: u16, total: u16) -> Bgr555 {
    let channel = |shift: u16| {
        let (a, b) = ((from.raw() >> shift) & 0x1F, (to.raw() >> shift) & 0x1F);
        let share = (a.abs_diff(b) * step).checked_div(total).unwrap_or(0);
        let value = if b >= a { a + share } else { a - share };
        (value & 0x1F) << shift
    };
    Bgr555::new(channel(0) | channel(5) | channel(10))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blend_steps_each_channel_by_its_share_rounded_down() {
        let rgb = |r: u16, g: u16, b: u16| Bgr555::new(b << 10 | g << 5 | r);
        // Red 0 to 31, green 31 to 0, blue 10 to 13, at steps of 60.
        let (from, to) = (rgb(0, 31, 10), rgb(31, 0, 13));
        assert_eq!(blend(from, to, 0, 60), from);
        assert_eq!(blend(from, to, 30, 60), rgb(15, 16, 11));
        assert_eq!(blend(from, to, 59, 60), rgb(30, 1, 12));
        assert_eq!(blend(from, to, 60, 60), to);
    }

    #[test]
    fn a_load_keeps_the_obj_colours_and_drops_the_background_ones() {
        // `COP 5A CC 30 4D 7F 03`: three colours from `$CC:4D30` to `$7F`.
        let mut image = vec![0; 0x0C_4D40];
        image[0x0C_4D30..0x0C_4D36].copy_from_slice(&[0x11, 0x01, 0x22, 0x02, 0x33, 0x03]);
        image[..5].copy_from_slice(&[0xCC, 0x30, 0x4D, 0x7F, 3]);
        let mut colours = ObjColours::default();
        assert!(colours.load(&image, 0));
        assert_eq!(colours.get(0x7F), None);
        assert_eq!(colours.get(0x80), Some(Bgr555::new(0x0222)));
        assert_eq!(colours.get(0x81), Some(Bgr555::new(0x0333)));
        assert_eq!(colours.get(0x82), None);
        assert!(!colours.load(&image, image.len() - 2));
    }
}
