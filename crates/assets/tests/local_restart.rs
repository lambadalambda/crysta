//! The Restart file select's art and text from both ROMs
//! (`docs/restart-screen.md`).
use assets::records::Slot;
use assets::restart::{self, RestartArt};
use rom::Rom;
use std::path::Path;

fn load(name: &str) -> Option<Rom> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../local")
        .join(name);
    Rom::load(&std::fs::read(path).ok()?).ok()
}

#[test]
fn the_scroll_title_and_page_decode_on_both_roms() {
    for (name, title) in [
        ("Tenchi Souzou (Japan).sfc", 4),
        ("Terranigma (E) [!].smc", 7),
    ] {
        let Some(rom) = load(name) else {
            continue;
        };
        let image = rom.image();
        let art = RestartArt::from_rom(image).unwrap();
        assert_eq!(art.bg_map.len(), 32 * 32, "{name}");
        let blank_or_scroll =
            |entry: u16| entry == 0 || (entry & 0x3FF <= 0x71 && matches!(entry >> 10 & 7, 2 | 3));
        assert!(
            art.bg_map.iter().all(|&entry| blank_or_scroll(entry)),
            "{name}: blank, or tiles 0–$71 in palettes 2 and 3"
        );
        assert_eq!(art.title.len(), title, "{name}");
        let ark = [0x21, 0x52, 0x4B];
        let pages = restart::page_text(image, &[None, Some(Slot { name: &ark }), None]).unwrap();
        // The page, then the three slots.
        assert_eq!(pages.len(), 4, "{name}");
        let lines: Vec<u16> = pages[0]
            .glyphs()
            .iter()
            .map(|glyph| glyph.position[1])
            .collect();
        assert_eq!(lines.iter().max(), Some(&80), "{name}: six lines");
        assert_eq!(
            pages[2].glyphs()[0].position,
            [32, 16],
            "{name}: slot 2's name"
        );
    }
}
