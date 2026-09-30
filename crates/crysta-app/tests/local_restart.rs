//! The Restart file select drawn from the ROM against native frames
//! (`docs/restart-screen.md`): `local/restart/*.rgb`.
#[path = "support/native.rs"]
mod native;
use crysta_app::frame::{Canvas, CLASSIC_WIDTH};
use crysta_app::restart::{self, RestartArtCache};
use crysta_runtime::restart::{Page, View};
use crysta_runtime::sram::Sram;
use native::{best, local};
use rom::Rom;

/// The page fully typed, with the cursor on `cursor` and Copy's second
/// cursor on `second`.
fn page(cursor: Option<u8>, second: Option<u8>) -> View {
    View::Screen(Page {
        brightness: 15,
        cursor,
        second,
        lines: 6,
        slots: 3,
    })
}

#[test]
fn the_screen_matches_native_frames_on_both_roms() {
    let (Some(saves), Some(blank), Some(copy)) = (
        local("saves/Terranigma.srm"),
        local("restart/blank.srm"),
        local("restart/copy.srm"),
    ) else {
        return;
    };
    let sram = |bytes: &[u8]| Sram::from_bytes(bytes).unwrap().repaired();
    let mut copied = sram(&copy);
    assert!(copied.copy_slot(0, 2));
    let mut erased = sram(&saves);
    erased.erase(1);
    erased.repair();
    let scenes = [
        ("blank", sram(&blank), page(Some(0), None)),
        ("2008", sram(&saves), page(Some(2), None)),
        ("copy-source", sram(&copy), page(Some(0), None)),
        ("copy-dest", sram(&copy), page(Some(0), Some(2))),
        ("copy-done", copied.repaired(), page(Some(0), None)),
        ("erase-confirm", sram(&saves), page(Some(1), None)),
        ("erase-done", erased, page(Some(2), None)),
    ];
    for (rom, prefix) in [
        ("Tenchi Souzou (Japan).sfc", "jp"),
        ("Terranigma (E) [!].smc", "eu"),
    ] {
        let Some(bytes) = local(rom) else {
            continue;
        };
        let rom = Rom::load(&bytes).unwrap();
        let image = rom.image();
        let cache = RestartArtCache::default();
        let art = cache.get(image).unwrap();
        for (scene, sram, view) in &scenes {
            let native = local(&format!("restart/{prefix}-{scene}.rgb"))
                .expect("with the ROM, its captures in `local/restart`");
            let mut canvas = Canvas::new(CLASSIC_WIDTH);
            restart::draw(&mut canvas, image, art, *view, sram);
            let (unlike, offset) = best(&canvas, &native);
            assert_eq!(unlike, 0, "{prefix}-{scene} at {offset:?}");
        }
    }
}
