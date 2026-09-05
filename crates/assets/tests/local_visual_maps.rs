//! Optional authenticated ROM-only visual resource fixtures.
use assets::{
    graphics::IndexedPixel,
    maps::visual::{CavernBackground, StaticBackground},
};
use rom::{Revision, Rom};
use std::path::Path;

#[test]
fn cavern_visual_resources_and_full_indexed_layer_are_stable() {
    let Some(rom) = local_rom() else {
        return;
    };
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

fn local_rom() -> Option<Rom> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/Tenchi Souzou (Japan).sfc");
    match std::fs::read(&path) {
        Ok(bytes) => {
            let rom = Rom::load(&bytes).unwrap();
            assert_eq!(rom.revision(), Revision::Japan);
            Some(rom)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("skipping: local Japanese ROM not present");
            None
        }
        Err(e) => panic!("reading {}: {e}", path.display()),
    }
}

#[test]
fn room_visual_resources_and_complete_first_background_are_stable() {
    let Some(rom) = local_rom() else {
        return;
    };
    for id in [0xf, 0x10] {
        let scene = StaticBackground::from_rom(rom.image(), id).unwrap();
        assert_eq!((scene.layer().width(), scene.layer().height()), (32, 64));
        assert_eq!(scene.tiles().len(), 768);
        assert_eq!(scene.layer().source_range(), 0x2f_cbb3..0x2f_cfe1);
        assert_eq!(
            scene.layer().source_bytes(),
            &rom.image()[scene.layer().source_range()]
        );
        let fixtures = [
            (
                0x23_b11d..0x23_e222,
                "0615ed6038f710e54707c8a9614eca0b9ae9716367f3b6cc2b679e81dc98f443",
            ),
            (
                0x31_e945..0x31_ea05,
                "38b1af79146a7a1360decaa84aa5d49c65d378c3cf0b05787814a998a79d5c2e",
            ),
            (
                0x2a_ba43..0x2a_c506,
                "ad57ffc7e313e0c5436c8372542845da994bf965b88195d86b068ca2f0c3f4da",
            ),
            (
                0x30_ff53..0x30_fffc,
                "f4a836fc55450d600becc83b3a8b5836de0983ef2fa63f30e8deeea262d98ede",
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
            digest(&indices),
            "4adec38bf192483ec43e62feaeb9219cc1e0d02b4860bc3b682dd5cc11c8886d"
        );
        assert_eq!(
            digest(&priorities),
            "689d88232a9fc9a10d4550ffc8ff31a55ef0826b28046e8b14933cc902968174"
        );
    }
}

/// Optional ignored captures from the documented cold-boot input replay.
/// The test authenticates each capture rather than trusting arbitrary filenames.
#[test]
fn room_resources_match_qualified_runtime_except_documented_effects() {
    let Some(rom) = local_rom() else {
        return;
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/room-runtime");
    if !root.try_exists().unwrap() {
        eprintln!("skipping: ignored room-runtime replay captures not present");
        return;
    }
    for (id, frame, hashes) in [
        (
            0xf,
            1601,
            [
                "655ea43073c4a4f7f0f1b8d736125c1a7b62ddbe07af912ee958c3eb162f0dd6",
                "410ae58358a216ac7cbea928a4f0b6201f76b67e601944c035d50ee8af3d387d",
                "1b1902ad7bddd1d46e87ce86e3bc53bed3407283caa02cf4f35a89d41e75296d",
            ],
        ),
        (
            0x10,
            1801,
            [
                "22e775d46572211cca57fe9b763499173e16c6aab18f3e5506d0338313098d90",
                "1f5f99fe06f93d737ca24ec8aef89a6dd0a60525c3526124581213222032766a",
                "85c0e45763fe4837e9bb792864754af1306439e88f30285286b2d37e9168ce56",
            ],
        ),
    ] {
        let memories: Vec<_> = ["wram", "vram", "cgram"]
            .into_iter()
            .zip(hashes)
            .map(|(name, hash)| {
                let bytes = std::fs::read(root.join(format!("f{frame}.{name}"))).unwrap();
                assert_eq!(digest(&bytes), hash);
                bytes
            })
            .collect();
        let (wram, vram, cgram) = (&memories[0], &memories[1], &memories[2]);
        let scene = StaticBackground::from_rom(rom.image(), id).unwrap();
        let r = scene.resources();
        assert_eq!(r[2].decoded(), &wram[0x2000..0x3000]);
        assert_eq!(r[1].decoded(), &cgram[64..256]);
        assert_eq!(&r[4].decoded()[2..], &cgram[2..64]); // backdrop color 0 differs
        for (i, (&raw, &runtime)) in r[0].decoded().iter().zip(vram).enumerate() {
            if !(37 * 32..41 * 32).contains(&i) {
                assert_eq!(raw, runtime, "tile {}", i / 32);
            }
        }
        assert_ne!(r[0].decoded(), &vram[..0x6000]); // qualified fire animation difference
        assert_eq!(&r[3].decoded()[1..], &wram[0x10001..0x10200]);
        assert_eq!(wram[0x10000], if id == 0xf { 22 } else { 0 });
        for (cell, bytes) in scene
            .layer()
            .cells()
            .iter()
            .zip(wram[0xa000..0xb000].chunks_exact(2))
        {
            assert_eq!(cell.raw(), u16::from_le_bytes([bytes[0], bytes[1]]) & 511);
        }
    }
}
