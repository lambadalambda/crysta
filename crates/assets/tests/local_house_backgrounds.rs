//! Optional authenticated Japanese ROM test; no extracted fixtures are tracked.
use assets::{graphics::IndexedPixel, maps::visual::StaticBackground};
use rom::{Revision, Rom};

fn hash(bytes: &[u8]) -> String {
    use std::fmt::Write;
    rom::digests(bytes)
        .sha256
        .iter()
        .fold(String::new(), |mut hex, byte| {
            write!(hex, "{byte:02x}").unwrap();
            hex
        })
}

#[test]
fn all_six_house_profiles_share_the_unchanged_complete_natural_background() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../local/Tenchi Souzou (Japan).sfc");
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("skipping optional local Japanese ROM qualification");
            return;
        }
        Err(e) => panic!("reading ROM: {e}"),
    };
    let rom = Rom::load(&bytes).unwrap();
    assert_eq!(rom.revision(), Revision::Japan);
    let baseline = StaticBackground::from_rom(rom.image(), 0xf).unwrap();
    for id in [0xb, 0xc, 0xd, 0xf, 0x10, 0x11] {
        let scene = StaticBackground::from_rom(rom.image(), id).unwrap();
        assert_eq!((scene.layer().width(), scene.layer().height()), (32, 64));
        assert_eq!(
            scene.layer().source_range(),
            baseline.layer().source_range()
        );
        assert_eq!(scene.layer().cells(), baseline.layer().cells());
        assert_eq!(scene.palette(), baseline.palette());
        assert_eq!(scene.resources().len(), baseline.resources().len());
        for (got, want) in scene.resources().iter().zip(baseline.resources()) {
            assert_eq!(got.source_range(), want.source_range());
            assert_eq!(got.source_bytes(), want.source_bytes());
            assert_eq!(got.decoded(), want.decoded());
        }
        let mut indices = Vec::new();
        let mut priorities = Vec::new();
        for y in 0..1024 {
            for x in 0..512 {
                let (index, priority) = match scene.pixel(x, y).unwrap() {
                    IndexedPixel::Transparent => (0, false),
                    IndexedPixel::Opaque {
                        palette_index,
                        priority,
                    } => (palette_index, priority),
                };
                indices.push(index);
                priorities.push(u8::from(priority));
            }
        }
        assert_eq!(
            hash(&indices),
            "4adec38bf192483ec43e62feaeb9219cc1e0d02b4860bc3b682dd5cc11c8886d"
        );
        assert_eq!(
            hash(&priorities),
            "689d88232a9fc9a10d4550ffc8ff31a55ef0826b28046e8b14933cc902968174"
        );
        let palette: Vec<_> = scene
            .palette()
            .iter()
            .flat_map(|c| c.raw().to_le_bytes())
            .collect();
        assert_eq!(
            hash(&palette),
            "f390bfcaf76ff322de892ca50514b387bce1c2c29b949376ea6e6d409dec2010"
        );
    }
    for id in [0xe, 0x20, 0x21] {
        assert!(StaticBackground::from_rom(rom.image(), id).is_err());
    }
}
