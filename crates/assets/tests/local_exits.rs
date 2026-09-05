//! Owned-ROM qualification: metadata and hashes only, never extracted record fixtures.
use assets::maps::exits::ExitList;
use rom::{Revision, Rom};
use std::path::Path;

#[test]
fn opening_south_doorway_matches_static_source_and_runtime_selection() {
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
    assert_eq!(
        digest(rom.image()),
        "f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548"
    );
    // Hash only the supported lookup prefix; do not claim this is the full table.
    assert_eq!(
        digest(&rom.image()[0x01_8000..0x01_88A0]),
        "49aef0affa07705c3a0134aeabbc1299a7868071d615aee858712ed8061e701e"
    );
    for (map_id, start, end, count, hash) in [
        (
            0x0F,
            0x01_8E3C,
            0x01_8E55,
            2,
            "3892a00a4104b13625fa277fdc71ceb432af79e48693b353afadd2b96275a580",
        ),
        (
            0x10,
            0x01_8E55,
            0x01_8E7A,
            3,
            "a3784e3b0ca4e9fdceb81bb2b8aa8639fbe21f43545c85914175c5d1da4bf2f2",
        ),
    ] {
        let exits = ExitList::from_rom(rom.image(), map_id).unwrap();
        assert_eq!(exits.source_range(), Some(start..end));
        assert_eq!(exits.records().len(), count);
        assert_eq!(exits.source_bytes(), &rom.image()[start..end]);
        assert_eq!(digest(exits.source_bytes()), hash);
    }

    let exits = ExitList::from_rom(rom.image(), 0x0F).unwrap();
    assert_eq!(exits.entry().unwrap().value(), 0x81_8E3C);
    // Parent's successful $8D:883E stage: frame 1680, X=$8E3C, DB=$81.
    // Player (392,209) has bounds-origin offsets (-8,-16).
    let selected = exits.select(392 - 8, 209 - 16).unwrap();
    assert_eq!(selected, &exits.records()[0]);
    assert_eq!(selected.source_range(), 0x01_8E3C..0x01_8E48);
    assert!(exits.select(392, 209).is_none()); // Player position is not the input.
                                               // Raw pending destination fields at $8D:888D, before effect/arrival adjustment.
    assert_eq!(selected.raw_destination(), 0x10);
    assert_eq!(selected.direct_destination(), Ok(0x10));
    assert_eq!(selected.transition_mode(), 0);
    assert_eq!(selected.selector(), 5);
    assert_eq!(selected.destination_position(), (384, 336));

    let reverse = ExitList::from_rom(rom.image(), 0x10).unwrap();
    assert_eq!(reverse.entry().unwrap().value(), 0x81_8E55);
    assert_eq!(reverse.records()[0].direct_destination(), Ok(0x0F));
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
