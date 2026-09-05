//! Optional checks against authenticated local JP/EU dumps. Only metadata and
//! independent community-decoder output hashes are committed (see docs/compression.md).

use assets::compression::{decode, encode, MAX_OUTPUT_SIZE};
use rom::{Revision, Rom};
use std::path::Path;

struct Packet {
    name: &'static str,
    offset: usize,
    compressed_size: usize,
    output_size: usize,
    compressed_sha256: &'static str,
    output_sha256: &'static str,
}

fn fixtures(revision: Revision) -> [Packet; 3] {
    let japanese = revision == Revision::Japan;
    [
        Packet {
            name: "bank-zero graphics",
            offset: 0,
            compressed_size: if japanese { 0x1693 } else { 0x1834 },
            output_size: 0x2000,
            compressed_sha256: if japanese {
                "a70c792994b4782fab097ce7282c68aaa418e3d7eba7942ae77386f439e91a7f"
            } else {
                "5cbbcd2ed2ea4a834adaae813574c7d2ace3abb8e954475d05393e6285565e59"
            },
            output_sha256: if japanese {
                "c2649ee61bdd4e89362b5e23d633d6ec8e8caafb44b2ae3fb157506935a6400d"
            } else {
                "0877a4cce3fce812a35398da01dfe586588604306d95bbde58f85070f3f5c621"
            },
        },
        Packet {
            name: "intro Earth graphics",
            offset: if japanese { 0x2D_0000 } else { 0x2F_0000 },
            compressed_size: 0x46C0,
            output_size: 0x8000,
            compressed_sha256: "a1b0c74ea3a8e06b03ae00569e482d746effea8ffbe096d07d25ba318481f202",
            output_sha256: "e61b2cb1d7f0e37b9610073a2004513317547e78ade2f978a8a6e9a43f98e4ce",
        },
        Packet {
            name: "box menu map",
            offset: if japanese { 0x30_B113 } else { 0x32_B215 },
            compressed_size: 0x031C,
            output_size: 0x0800,
            compressed_sha256: "feb5ab2db0489321383c248f9c6e15ecac047a0e640005706f5654234300b3b1",
            output_sha256: "6159da7997834fbd3a348743e716586cce6b39f256b106e675f9c669c369a6e8",
        },
    ]
}

fn sha256(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    rom::digests(bytes)
        .sha256
        .iter()
        .fold(String::new(), |mut text, byte| {
            write!(text, "{byte:02x}").unwrap();
            text
        })
}

fn verify_local_packets(name: &str, revision: Revision) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../local")
        .join(name);
    let input = match std::fs::read(&path) {
        Ok(input) => input,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("skipping: {} not present", path.display());
            return;
        }
        Err(error) => panic!("reading {}: {error}", path.display()),
    };
    let rom = Rom::load(&input).expect("local dump must authenticate");
    assert_eq!(rom.revision(), revision);
    for packet in fixtures(revision) {
        let source = &rom.image()[packet.offset..packet.offset + packet.compressed_size];
        assert_eq!(sha256(source), packet.compressed_sha256, "{}", packet.name);
        let decoded = decode(&rom.image()[packet.offset..], MAX_OUTPUT_SIZE)
            .unwrap_or_else(|error| panic!("{}: {error}", packet.name));
        assert_eq!(decoded.consumed, packet.compressed_size, "{}", packet.name);
        assert_eq!(decoded.data.len(), packet.output_size, "{}", packet.name);
        assert_eq!(
            sha256(&decoded.data),
            packet.output_sha256,
            "{}",
            packet.name
        );
        assert_eq!(decode(source, packet.output_size).unwrap(), decoded);
        let encoded = encode(&decoded.data).expect("qualified output must encode");
        assert_eq!(
            encoded.len(),
            source.len(),
            "{} encoded length",
            packet.name
        );
        assert!(
            encoded == source,
            "{} must re-encode byte-for-byte",
            packet.name
        );
        assert_eq!(
            decode(&encoded, packet.output_size).unwrap().data,
            decoded.data
        );
        eprintln!(
            "{} {}: offset=${:06X}, {} -> {} bytes",
            revision.id(),
            packet.name,
            packet.offset,
            decoded.consumed,
            decoded.data.len()
        );
    }
}

#[test]
fn japanese_graphics_and_maps_match_community_decoder_hashes() {
    verify_local_packets("Tenchi Souzou (Japan).sfc", Revision::Japan);
}

#[test]
fn european_graphics_and_maps_match_community_decoder_hashes() {
    verify_local_packets("Terranigma (E) [!].smc", Revision::EuropeEnglish);
}
