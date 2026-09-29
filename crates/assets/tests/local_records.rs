//! The Records screen's text and art from both ROMs (`docs/records-screen.md`).
use assets::records::{self, Slot};
use rom::Rom;
use std::path::Path;

fn load(name: &str) -> Option<Rom> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../local")
        .join(name);
    Rom::load(&std::fs::read(path).ok()?).ok()
}

const ROMS: [&str; 2] = ["Tenchi Souzou (Japan).sfc", "Terranigma (E) [!].smc"];

#[test]
fn the_entry_page_places_slots_names_and_the_current_game() {
    for name in ROMS {
        let Some(rom) = load(name) else {
            continue;
        };
        let image = rom.image();
        let ark = [0x21, 0x52, 0x4B];
        let slots = [None, Some(Slot { name: &ark }), None];
        let pages = records::entry_text(image, &slots, &ark).unwrap();
        // The page, three slots, the current game.
        assert_eq!(pages.len(), 5, "{name}");
        let first = |page: usize| pages[page].glyphs()[0].position;
        assert_eq!(pages[0].glyphs()[1].position, [12, 0], "{name}: ` 1`");
        assert_eq!(first(1), [32, 0], "{name}: slot 1's No Data at x 56");
        assert_eq!(first(2), [32, 16], "{name}: slot 2's name");
        assert_eq!(pages[2].glyphs().len(), 3, "{name}");
        assert_eq!(first(4), [32, 80], "{name}: the current game at y 168");
        let saved = records::saved_text(image, 1, &slots).unwrap();
        // The saved page, then the three slots again.
        assert_eq!(saved.len(), 4, "{name}");
        // Digit 2: glyph `$73 + 2` of the font at `$B4:8000` (European
        // `$63 + 2` at `$B6:8000`).
        let (x, two) = if name.starts_with("Tenchi") {
            (24, 0xB4_8000 + 0x75 * 64)
        } else {
            (12, 0xB6_8000 + 0x65 * 64)
        };
        let digit = saved[0]
            .glyphs()
            .iter()
            .find(|glyph| glyph.font_source == two && glyph.position[1] == 80);
        assert_eq!(
            digit.map(|glyph| glyph.position),
            Some([x, 80]),
            "{name}: `2 saved`"
        );
    }
}
