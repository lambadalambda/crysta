//! Checks of the European English ROM's decoders (`docs/european-text.md`).
use assets::text::HouseDialogue;
use rom::{Revision, Rom};
use std::path::Path;

fn european() -> Option<Rom> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/Terranigma (E) [!].smc");
    let rom = Rom::load(&std::fs::read(path).ok()?).ok()?;
    (rom.revision() == Revision::EuropeEnglish).then_some(rom)
}

/// The European glyph codes as text, for checks.
fn transcribe(code: u8) -> char {
    match code {
        0x20 => ' ',
        0x21..=0x3A => char::from(b'A' + code - 0x21),
        0x41..=0x5A => char::from(b'a' + code - 0x41),
        0x60 => '?',
        0x6D => '!',
        0x6E => ',',
        0x6F => ':',
        0x7F => '.',
        _ => '#',
    }
}

#[test]
fn the_first_bedroom_pages_decode_with_their_dictionary_words() {
    let Some(rom) = european() else {
        return;
    };
    let image = rom.image();
    let pages = HouseDialogue::decode_at(image, 0x88_9C15).unwrap();
    let text = |index: usize| -> String {
        pages[index]
            .glyphs()
            .iter()
            .map(|glyph| transcribe(u8::try_from((glyph.font_source - 0xB6_8000) / 64).unwrap()))
            .collect()
    };
    assert_eq!(text(0), "Elle: ...Are you allright?");
    assert_eq!(pages[0].speaker().raw(), 0x5E3F);
    assert_eq!(pages[0].glyphs()[0].palette, 1);
    // Four lines: the window is 224 x 64.
    assert_eq!((pages[0].width(), pages[0].height()), (224, 64));
    // "looked " and "having " come from the dictionaries (`E5`, `E6`).
    assert_eq!(text(1), "Ark.You looked likeyou were having anightmare.");
}

fn japanese() -> Option<Rom> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/Tenchi Souzou (Japan).sfc");
    Rom::load(&std::fs::read(path).ok()?).ok()
}

/// The slice maps: Crysta and its houses, and the box's tour.
const SLICE: [u16; 28] = [
    0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19,
    0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F, 0x20, 0x21, 0x41, 0x42, 0x43, 0x44,
];

#[test]
fn the_slice_backgrounds_are_the_japanese_ones_moved() {
    use assets::maps::visual::first_background;
    let (Some(europe), Some(japan)) = (european(), japanese()) else {
        return;
    };
    for map in SLICE {
        let eu = first_background(europe.image(), map).unwrap_or_else(|e| panic!("{map:x}: {e:?}"));
        let jp = first_background(japan.image(), map).unwrap();
        let (a, b) = (eu.layer(), jp.layer());
        assert_eq!((a.width(), a.height()), (b.width(), b.height()), "{map:x}");
        let changed: Vec<usize> = (0..a.cells().len())
            .filter(|&i| a.cells()[i] != b.cells()[i])
            .collect();
        assert!(
            changed.is_empty(),
            "{map:x}: {} cells differ, first {:?}",
            changed.len(),
            &changed[..changed.len().min(8)]
        );
        assert_eq!(eu.tiles(), jp.tiles(), "{map:x}");
        assert_eq!(eu.metatiles(), jp.metatiles(), "{map:x}");
        assert_eq!(eu.palette(), jp.palette(), "{map:x}");
    }
}
