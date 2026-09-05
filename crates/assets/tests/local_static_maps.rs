//! Optional static decoding qualification against the authenticated Japanese ROM.
use assets::maps::StaticLayer;
use rom::{Revision, Rom};
use std::path::Path;

#[test]
fn cavern_static_packet_matches_loader_qualified_extent() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/Tenchi Souzou (Japan).sfc");
    let input = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("skipping: local Japanese ROM not present");
            return;
        }
        Err(error) => panic!("reading {}: {error}", path.display()),
    };
    let rom = Rom::load(&input).unwrap();
    assert_eq!(rom.revision(), Revision::Japan);
    let layer = StaticLayer::from_rom(rom.image(), 0x09_0000).unwrap();
    assert_eq!((layer.width(), layer.height()), (80, 32));
    assert_eq!(layer.source_range(), 0x09_0000..0x09_05F8);
    assert_eq!(layer.source_bytes(), &rom.image()[layer.source_range()]);
    assert_eq!(
        digest(&layer.source_bytes()[2..]),
        "0c17b274896c4994ed883d1601540ad3f26f8d151a0f2f442e952ca4a9895b4d"
    );
    assert_eq!(
        digest(&layer.layer_bytes()),
        "6a7495daacd32b54f0b6caf22bde1b873fa444455c5a7c39c854adfa230015fb"
    );
    assert!(layer.cells().iter().all(|cell| cell.raw() <= 0x1FF));
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
