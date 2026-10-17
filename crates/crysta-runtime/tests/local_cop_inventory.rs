//! `docs/cop-inventory.md` against a fresh census of the Japanese ROM.

use crysta_runtime::service_sites::{census, markdown};
use crysta_runtime::{TOWER_MAPS, WORLD_MAPS};
use rom::{Revision, Rom};
use std::path::Path;

#[test]
fn the_cop_inventory_is_current() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let Ok(bytes) = std::fs::read(root.join("local/Tenchi Souzou (Japan).sfc")) else {
        return;
    };
    let rom = Rom::load(&bytes).expect("the Japanese ROM loads");
    assert_eq!(rom.revision(), Revision::Japan);
    let maps: Vec<u16> = TOWER_MAPS.iter().chain(&WORLD_MAPS).copied().collect();
    let report = markdown(&census(rom.image(), &maps));
    let doc = root.join("docs/cop-inventory.md");
    if std::env::var_os("CRYSTA_WRITE_INVENTORY").is_some() {
        std::fs::write(&doc, &report).expect("the doc is written");
    }
    let current = std::fs::read_to_string(&doc).unwrap_or_default();
    assert!(
        current == report,
        "docs/cop-inventory.md is not current; run with CRYSTA_WRITE_INVENTORY=1"
    );
}
