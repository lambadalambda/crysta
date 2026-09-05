//! Optional authenticated ROM checks against independently reconstructed capture evidence.
use assets::sprites::{ArkSprites, SpritePixel};
use std::fmt::Write as _;
#[test]
#[allow(clippy::too_many_lines)] // Audited frame-hash fixture list.
fn selected_rom_frames_match_reference_indexed_compositions() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../local/Tenchi Souzou (Japan).sfc");
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("skipping: local ROM absent");
            return;
        }
        Err(e) => panic!("{e}"),
    };
    let rom = rom::Rom::load(&bytes).unwrap();
    assert_eq!(rom.revision(), rom::Revision::Japan);
    let a = ArkSprites::from_rom(rom.image()).unwrap();
    assert_eq!(a.frames().len(), 21);
    for (id, mirror, expected) in [
        (
            0x9a_d76a,
            false,
            "cab1328186e4b444131579b913c7acba79cca5d1da163ec4611902cbe8345523",
        ),
        (
            0x9a_d7ac,
            false,
            "6acebec5e4db33528e280b67e48aaa3db3bbc62e14b7163ef347d1d71f008780",
        ),
        (
            0x9a_d7e7,
            false,
            "52291ddb5a9062e1d53b13e72711f14e20fc40e63fecad8700b9743e05d4e1cf",
        ),
        (
            0xa4_a5e0,
            false,
            "69ee6e91b11f6627e3e0dfff524a714903276ad6eddbd81ff829191b93acd15e",
        ),
        (
            0x9a_d48a,
            false,
            "7c06f3cd07eab3f5ee028b50f65cc1bdc5c4312e11faabba8e34d03d0916e723",
        ),
        (
            0x9a_d4c5,
            false,
            "59edf5e32284129c1a0f81b1987b0fc2c16edda503f89987dd39cc5ecd3d76d1",
        ),
        (
            0x9a_d507,
            false,
            "4946e6bb347bbbfe85ac3761119581f04058c406aaa2ed377a6cdfb7f3c5cd13",
        ),
        (
            0xa4_a54e,
            false,
            "7545faa75875f7e52b35bece4e647cffb10731eeb9a392c47b5e5a01c1d3c4c5",
        ),
        (
            0x9a_d608,
            false,
            "ac6105149397e7fb82d73a1759e138c4bd7ca5a8b152f36a1e06c2860557f60e",
        ),
        (
            0x9a_d64a,
            false,
            "5c2e1881f11e1e524bdf260f04569c6364a4e989953b6640b39c39c5c5e763d6",
        ),
        (
            0xa4_a597,
            false,
            "0fb9d7580f5151296f40d818f72ee27d3cc9fa5b16e84fc47643d72534fb58b2",
        ),
        (
            0x9a_d76a,
            true,
            "ea1533e142dd2ff0c9036a885b4b9e0c066859083cc0fe68992237ea4d024ff3",
        ),
        (
            0x9a_d7ac,
            true,
            "ba4187c4edf2dcdb8277e85e8ae35c90a4d36e02d6262f5aca65471053821244",
        ),
        (
            0xa4_a5e0,
            true,
            "620884b65f16ee577142a028fd555e37577abeaed882ca3220789082e7b6bbd0",
        ),
    ] {
        let frame = a.frame(id).unwrap();
        let tiles = a.graphics(frame.resource()).unwrap();
        let mut indices = Vec::new();
        for y in -48..16 {
            for x in -32..32 {
                indices.push(
                    match frame
                        .composition()
                        .sample(tiles, mirror, false, x, y)
                        .unwrap()
                    {
                        SpritePixel::Transparent => 0,
                        SpritePixel::Opaque { palette_index, .. } => palette_index,
                    },
                );
            }
        }
        let actual = rom::digests(&indices)
            .sha256
            .iter()
            .fold(String::new(), |mut s, byte| {
                write!(s, "{byte:02x}").unwrap();
                s
            });
        assert_eq!(actual, expected, "{id:x}, mirror {mirror}");
    }
    assert!(
        a.frame(0xa5_f839).is_none(),
        "special idle is not ordinary standing"
    );
}
