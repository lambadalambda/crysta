//! The Records screen drawn from the ROM against native frames
//! (`docs/records-screen.md`): `local/records/*.rgb`, 256×240 RGB captures
//! from the reference emulator.
#[path = "support/native.rs"]
mod native;
use crysta_app::frame::{Canvas, CLASSIC_WIDTH};
use crysta_app::records::{self, Games, RecordsArtCache};
use crysta_runtime::records::{Page, Text};
use crysta_runtime::save::SaveSlot;
use crysta_runtime::sram::Sram;
use native::{best, local};
use rom::Rom;

#[test]
fn the_screen_matches_native_frames_on_both_roms() {
    for (rom, prefix) in [
        ("Tenchi Souzou (Japan).sfc", "jp"),
        ("Terranigma (E) [!].smc", "eu"),
    ] {
        let Some(bytes) = local(rom) else {
            continue;
        };
        let rom = Rom::load(&bytes).unwrap();
        let image = rom.image();
        // The captured game: a new one, 61 seconds in (0:01).
        let mut current = SaveSlot::new_game(image);
        for _ in 0..61 * 60 {
            current.tick_clock();
        }
        let mut saved = Sram::default();
        saved.write_slot(0, &current);
        let cache = RecordsArtCache::default();
        let art = cache.get(image).unwrap();
        let entry = Page {
            cursor: 0,
            title: true,
            text: Text::Entry,
        };
        let typed = Page {
            text: Text::Saved {
                slot: 0,
                lines: 3,
                message: true,
                slots: 3,
            },
            ..entry
        };
        for (capture, page, sram) in [("menu", entry, Sram::default()), ("saved", typed, saved)] {
            let native = local(&format!("records/{prefix}-{capture}.rgb"))
                .expect("with the ROM, its captures in `local/records`");
            let mut canvas = Canvas::new(CLASSIC_WIDTH);
            let games = Games {
                sram: &sram,
                current: &current,
            };
            records::draw(&mut canvas, image, art, page, &games);
            let (unlike, offset) = best(&canvas, &native);
            assert_eq!(unlike, 0, "{prefix}-{capture} at {offset:?}");
        }
    }
}
