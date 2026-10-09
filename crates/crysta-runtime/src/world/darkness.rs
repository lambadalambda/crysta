//! Tower 5's top dark but for its lit torches (`$8F:8182`,
//! `docs/tower-five.md`): `$123`'s second layer is a mask subtracted from
//! the first; an HDMA table switches it per band of lines between the map's
//! mask (`$3C00`, the band's torch lit) and the intro's fill (`$6C00`), and
//! the darkness's colour (`$7F:06FE`) falls as more torches burn.

use super::World;

/// The bands' heights from 256 above the map's top (`$8F:826B`), and the
/// darkness's colour by the torches lit (`$8F:8279`); the same in both
/// ROMs.
const HEIGHTS: usize = 0x0F_826B;
const COLOURS: usize = 0x0F_8279;
/// Seven bands, and an eighth below them that bit 7 would light: the
/// table's walk reads on into the colours there.
const BANDS: usize = 8;
/// Where the first band starts: 256 above the map.
const TOP: i32 = -0x100;

/// The darkness as the hosts draw it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Darkness {
    /// Each band's first map line and whether its torch burns.
    bands: Vec<(i32, bool)>,
    /// The darkness's colour, a BGR555 word: the mask's colour 127 and the
    /// fill's.
    pub colour: u16,
}

impl Darkness {
    /// Whether map line `y` shows the mask (its band's torch burns); else
    /// the fill darkens the whole line.
    #[must_use]
    pub fn lit(&self, y: i32) -> bool {
        self.bands
            .iter()
            .rev()
            .find(|&&(start, _)| start <= y)
            .is_some_and(|&(_, lit)| lit)
    }
}

/// The darkness for the lit torches `torches` (`$04A4`, a bit a band).
pub(super) fn darkness(image: &[u8], torches: u16) -> Darkness {
    let word = |at: usize| {
        image
            .get(at..at + 2)
            .map_or(0, |word| u16::from_le_bytes([word[0], word[1]]))
    };
    let mut start = TOP;
    let bands = (0..BANDS)
        .map(|band| {
            let band_start = start;
            start += i32::from(word(HEIGHTS + 2 * band));
            (band_start, torches >> band & 1 != 0)
        })
        .collect();
    let lit = usize::try_from((torches & 0x7F).count_ones()).unwrap_or(0);
    Darkness {
        bands,
        colour: word(COLOURS + 2 * lit),
    }
}

impl World<'_> {
    /// The darkness of tower 5's top, once its intro set it up.
    #[must_use]
    pub fn darkness(&self) -> Option<Darkness> {
        self.globals
            .torch_darkness
            .then(|| darkness(self.image, self.lit_torches()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_band_is_lit_by_its_torch_and_the_dark_eases_with_each() {
        let mut image = vec![0; COLOURS + 16];
        for (band, height) in [0xF0u16, 0x70, 0x70, 0x70, 0x70, 0x70, 0x120]
            .into_iter()
            .enumerate()
        {
            image[HEIGHTS + 2 * band..HEIGHTS + 2 * band + 2]
                .copy_from_slice(&height.to_le_bytes());
        }
        for (lit, colour) in [0x5252u16, 0x4E31].into_iter().enumerate() {
            image[COLOURS + 2 * lit..COLOURS + 2 * lit + 2].copy_from_slice(&colour.to_le_bytes());
        }
        // Torch 6 lights the last band, from line `$220`.
        let dark = darkness(&image, 0x40);
        assert!(!dark.lit(0x21F) && dark.lit(0x220) && dark.lit(0x33F));
        assert!(!dark.lit(0x340), "the eighth band, unlit");
        assert_eq!(dark.colour, 0x4E31);
        assert_eq!(darkness(&image, 0).colour, 0x5252);
    }
}
