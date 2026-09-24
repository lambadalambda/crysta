//! ROM addresses per revision ([ADR 0004](../../../docs/adr/0004-european-executable.md)).
//!
//! Each address the decoders read is an [`Address`]: its Japanese value, the
//! reference, and its European one once the correspondence is recorded
//! (`tools/eu-map`, `docs/european-text.md`). An image names its revision by
//! its header ([`rom::Revision::of_image`]); an image that names none, such
//! as a synthetic test image, reads the Japanese layout. An address with no
//! European value refuses the European image rather than read the wrong
//! bytes.

use rom::Revision;

mod europe;

/// A ROM address in each revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Address {
    japan: u32,
    europe: Option<u32>,
}

impl Address {
    /// An address known in both revisions.
    #[must_use]
    pub const fn both(japan: u32, europe: u32) -> Self {
        Self {
            japan,
            europe: Some(europe),
        }
    }

    /// An address whose European value is not recorded yet.
    #[must_use]
    pub const fn japan(japan: u32) -> Self {
        Self {
            japan,
            europe: None,
        }
    }

    /// The address in `image`'s revision, or `None` when unknown there.
    #[must_use]
    pub fn of(self, image: &[u8]) -> Option<u32> {
        match revision(image) {
            Revision::Japan => Some(self.japan),
            Revision::EuropeEnglish => self.europe,
        }
    }
}

/// The address in `image`'s revision of what sits at Japanese address
/// `japan`, from the recorded correspondence (`tools/eu-map`); `None` for
/// a European image when none is recorded.
#[must_use]
pub fn at(image: &[u8], japan: u32) -> Option<u32> {
    match revision(image) {
        Revision::Japan => Some(japan),
        Revision::EuropeEnglish => europe::EUROPE
            .binary_search_by_key(&japan, |&(from, _)| from)
            .ok()
            .map(|index| europe::EUROPE[index].1),
    }
}

/// The image offset in `image`'s revision of what sits at normalized
/// Japanese image offset `japan` ([`at`] for either `HiROM` window that
/// maps it); `None` for a European image when none is recorded.
#[must_use]
pub fn offset(image: &[u8], japan: usize) -> Option<usize> {
    let japan = u32::try_from(japan)
        .ok()
        .filter(|&offset| offset < 0x40_0000)?;
    match revision(image) {
        Revision::Japan => Some(japan),
        Revision::EuropeEnglish => {
            at(image, 0xC0_0000 | japan).or_else(|| at(image, 0x80_0000 | japan))
        }
    }
    .map(|address| (address & 0x3F_FFFF) as usize)
}

/// The revision whose layout `image` reads: its header's, else Japanese.
#[must_use]
pub fn revision(image: &[u8]) -> Revision {
    Revision::of_image(image).unwrap_or(Revision::Japan)
}

/// A value that differs by revision, not an address: a window's size, a
/// stride.
#[must_use]
pub fn per_revision<T>(image: &[u8], japan: T, europe: T) -> T {
    match revision(image) {
        Revision::Japan => japan,
        Revision::EuropeEnglish => europe,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn titled(title: &[u8]) -> Vec<u8> {
        let mut image = vec![0; 0x1_0000];
        image[0xFFC0..0xFFC0 + 21].fill(b' ');
        image[0xFFC0..0xFFC0 + title.len()].copy_from_slice(title);
        image
    }

    #[test]
    fn an_address_follows_the_images_revision_and_refuses_an_unknown_one() {
        let font = Address::both(0xB4_8000, 0xB6_8000);
        let table = Address::japan(0x92_C259);
        let europe = titled(b"TERRANIGMA P");
        assert_eq!(font.of(&europe), Some(0xB6_8000));
        assert_eq!(table.of(&europe), None);
        let japan = titled(b"TENCHI-JPN");
        assert_eq!(font.of(&japan), Some(0xB4_8000));
        // A synthetic image without a header reads the Japanese layout.
        assert_eq!(table.of(&[0; 16]), Some(0x92_C259));
        assert_eq!(per_revision(&europe, 48, 64), 64);
    }

    #[test]
    fn the_recorded_table_relocates_a_european_image_and_is_sorted() {
        let europe = titled(b"TERRANIGMA P");
        assert!(europe::EUROPE.windows(2).all(|pair| pair[0].0 < pair[1].0));
        // The COP table stays; the font moves two banks up.
        assert_eq!(at(&europe, 0x80_83B2), Some(0x80_83B2));
        assert_eq!(at(&europe, 0xB4_8000), Some(0xB6_8000));
        assert_eq!(at(&europe, 0x80_0001), None, "not recorded");
        assert_eq!(at(&[0; 16], 0x80_0001), Some(0x80_0001));
        // Offsets resolve through either window that maps them.
        assert_eq!(offset(&europe, 0x34_8000), Some(0x36_8000));
        assert_eq!(offset(&europe, 0x0C_2B2C), Some(0x0E_2B2C));
        assert_eq!(offset(&europe, 0x00_0001), None);
        assert_eq!(offset(&[0; 16], 0x00_0001), Some(1));
    }
}
