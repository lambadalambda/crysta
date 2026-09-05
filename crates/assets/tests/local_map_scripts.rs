//! Authenticated static cases; only maps $0004/$0128 also have loader equality tests.
use assets::maps::{
    scripts::{resolve_map, Command, Limits, ResourceKind},
    StaticLayer,
};
use rom::{Revision, Rom};
use std::path::Path;

type Layer = (usize, usize, usize, usize, &'static str);
struct Fixture {
    id: u16,
    entry: u32,
    instructions: usize,
    layers: &'static [Layer],
}
const FIXTURES: &[Fixture] = &[
    Fixture {
        id: 4,
        entry: 0xB3_8002,
        instructions: 16,
        layers: &[(
            0x0D_16C7,
            0x0D_1725,
            16,
            48,
            "3d28667ce54abe8e5d5327adac2cddcf87e5ce49c45b137e60f67f6a82d14ac5",
        )],
    },
    Fixture {
        id: 0x24,
        entry: 0xB3_8037,
        instructions: 16,
        layers: &[
            (
                0x0B_0C9D,
                0x0B_0FAD,
                32,
                64,
                "28ff1f17ee88b2a4fdeaa28ffee8a1e8f7c5a4f5a270f077b0f70fb05fb4866a",
            ),
            (
                0x0A_7E71,
                0x0A_7FDA,
                16,
                16,
                "a0cf3d7ffffa4af964733f323dfa46d4c7d0f3b6e511db0445887c490ee77faa",
            ),
        ],
    },
    Fixture {
        id: 0x25,
        entry: 0x98_82B0,
        instructions: 16,
        layers: &[
            (
                0x29_BAF5,
                0x29_C8A1,
                64,
                80,
                "1ff4401c085ac9a20850bc28335828367c37db25ed561a3a86112d9708caa633",
            ),
            (
                0x32_8A1E,
                0x32_8A69,
                16,
                16,
                "4991dbcf6f38edcaed50c5ec5652451e9d6d9a81730e0637fe556eb5cd81462f",
            ),
        ],
    },
    Fixture {
        id: 0x128,
        entry: 0xB3_89D3,
        instructions: 15,
        layers: &[(
            0x09_0000,
            0x09_05F8,
            80,
            32,
            "6a7495daacd32b54f0b6caf22bde1b873fa444455c5a7c39c854adfa230015fb",
        )],
    },
    Fixture {
        id: 0x263,
        entry: 0xD9_1182,
        instructions: 11,
        layers: &[
            (
                0x2B_3860,
                0x2B_397D,
                16,
                32,
                "b7fa4c6a846c2eadc0b8196e52c344ee3a0a1acc49bc8ec0b45954c5a837cf49",
            ),
            (
                0x2B_67F2,
                0x2B_6816,
                16,
                32,
                "e1a5488b8c046b9354bba4cbf0e0571c27df7d6f65442bac85adbbd3a7727528",
            ),
        ],
    },
];

#[test]
fn resolves_five_map_ids_and_decodes_eight_layer_packets() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/Tenchi Souzou (Japan).sfc");
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("skipping: local Japanese ROM not present");
            return;
        }
        Err(error) => panic!("reading {}: {error}", path.display()),
    };
    let rom = Rom::load(&bytes).unwrap();
    assert_eq!(rom.revision(), Revision::Japan);
    for fixture in FIXTURES {
        let program = resolve_map(rom.image(), fixture.id, Limits::default()).unwrap();
        assert_eq!(program.entry.value(), fixture.entry);
        assert_eq!(program.instructions.len(), fixture.instructions);
        let sources: Vec<_> = program
            .instructions
            .iter()
            .filter_map(|instruction| match instruction.command {
                Command::Resource {
                    kind: ResourceKind::Layer,
                    source,
                } => Some(source.normalized().value() as usize),
                _ => None,
            })
            .collect();
        assert_eq!(sources.len(), fixture.layers.len());
        for (source, &(start, end, width, height, hash)) in sources.iter().zip(fixture.layers) {
            assert_eq!(*source, start);
            let layer = StaticLayer::from_rom(rom.image(), *source).unwrap();
            assert_eq!(layer.source_range(), start..end);
            assert_eq!((layer.width(), layer.height()), (width, height));
            assert_eq!(digest(&layer.layer_bytes()), hash);
        }
    }
}
fn digest(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    rom::digests(bytes)
        .sha256
        .iter()
        .fold(String::new(), |mut text, byte| {
            write!(text, "{byte:02x}").unwrap();
            text
        })
}
