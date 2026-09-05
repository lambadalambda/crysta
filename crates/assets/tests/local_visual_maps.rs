//! Optional authenticated ROM-only visual resource fixtures.
use assets::{graphics::IndexedPixel, maps::visual::CavernBackground};
use rom::{Revision, Rom};
use std::path::Path;

#[test]
fn cavern_visual_resources_and_full_indexed_layer_are_stable() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/Tenchi Souzou (Japan).sfc");
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("skipping: local Japanese ROM not present");
            return;
        }
        Err(e) => panic!("reading {}: {e}", path.display()),
    };
    let rom = Rom::load(&bytes).unwrap();
    assert_eq!(rom.revision(), Revision::Japan);
    let scene = CavernBackground::from_rom(rom.image()).unwrap();
    assert_eq!((scene.layer().width(), scene.layer().height()), (80, 32));
    let fixtures = [
        (
            0x1c_45fe..0x1c_77b5,
            "06c37bb92a0571049642e701af19aba23739f5fb129f5b36fba5dcc9ef7cfef8",
        ),
        (
            0x2b_5426..0x2b_54e6,
            "0820e01ccbfa952c28f54317033a331db2fffe3e54a2bb07614b32c3b0d675ec",
        ),
        (
            0x20_76cd..0x20_7ffc,
            "81ea75bfce96d0a1dbc7fcff904a56baeaae023f11d38c39710072a4972abe1f",
        ),
        (
            0x2b_439e..0x2b_4462,
            "8adcb94d19af67b8995b84c442005cadbcef176df3811ff79a111dc590cb3914",
        ),
        (
            0x32_8b78..0x32_8bb8,
            "e24e9c2cabf46f25db23372360c8fad1c4c55159dadc6dc9ff70d97c5545066b",
        ),
    ];
    assert_eq!(scene.resources().len(), fixtures.len());
    for (resource, (range, hash)) in scene.resources().iter().zip(fixtures) {
        assert_eq!(resource.source_range(), range);
        assert_eq!(resource.source_bytes(), &rom.image()[range]);
        assert_eq!(digest(resource.decoded()), hash);
    }
    let indices: Vec<_> = (0..512)
        .flat_map(|y| (0..1280).map(move |x| (x, y)))
        .map(|(x, y)| match scene.pixel(x, y).unwrap() {
            IndexedPixel::Transparent => 0,
            IndexedPixel::Opaque { palette_index, .. } => palette_index,
        })
        .collect();
    #[allow(clippy::naive_bytecount)] // No extra dependency for a fixture count.
    let transparent = indices.iter().filter(|&&v| v == 0).count();
    assert_eq!(transparent, 25_281);
    assert_eq!(
        digest(&indices),
        "bb590b388811f2471232fc58e6a081455bd701540d4cf7a962c8b1e71827eb68"
    );
}
fn digest(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    rom::digests(bytes)
        .sha256
        .iter()
        .fold(String::new(), |mut s, byte| {
            write!(s, "{byte:02x}").unwrap();
            s
        })
}
