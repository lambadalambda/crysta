//! The world map's Mode 7 tables from both ROMs (`docs/world-map-mode7.md`).
use assets::maps::visual::mode7::Mode7View;
use rom::Rom;
use std::path::Path;

#[test]
fn the_view_tables_read_as_the_doc_gives_them() {
    for name in ["Tenchi Souzou (Japan).sfc", "Terranigma (E) [!].smc"] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../local")
            .join(name);
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let rom = Rom::load(&bytes).unwrap();
        let view = Mode7View::from_rom(rom.image()).unwrap();
        assert_eq!(
            (view.scale[0], view.scale[52], view.scale[223]),
            (0xD3, 0x23F, 0xB0),
            "{name}"
        );
        assert_eq!(
            (
                view.fade[0],
                view.fade[52],
                view.fade[54],
                view.fade[98],
                view.fade[200]
            ),
            (24, 15, 14, 1, 1),
            "{name}"
        );
        assert_eq!(
            (
                view.backdrop[0].raw(),
                view.backdrop[14].raw(),
                view.backdrop[51].raw(),
                view.backdrop[100].raw()
            ),
            (0x7227, 0x7F0E, 0x2C00, 0x2C00),
            "{name}"
        );
        assert_eq!((view.sky, view.sprites_from), (52, 68), "{name}");
    }
}
