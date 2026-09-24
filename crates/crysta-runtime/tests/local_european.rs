//! The slice on the European English ROM (ADR 0004).
use crysta_runtime::scene::Presses;
use crysta_runtime::world::World;
use rom::{Revision, Rom};
use std::path::Path;

fn european() -> Option<Rom> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/Terranigma (E) [!].smc");
    let rom = Rom::load(&std::fs::read(path).ok()?).ok()?;
    (rom.revision() == Revision::EuropeEnglish).then_some(rom)
}

#[test]
fn the_bedroom_loads_and_runs() {
    let Some(rom) = european() else {
        return;
    };
    let image = rom.image();
    let mut world = World::enter_with_events(
        image,
        0x000F,
        304,
        112,
        crysta_runtime::world::fresh_game_flags(),
    )
    .unwrap();
    for _ in 0..600 {
        world.update(None, Presses::default()).unwrap();
    }
    assert!(
        world.frozen_scripts().is_empty(),
        "{:x?}",
        world.frozen_scripts()
    );
}
