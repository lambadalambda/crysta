//! Small safe shared native/Wasm probe; no browser or host services in this module.

use room_core::{Direction, FrameInput, Room, WalkingState};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

fn sha256(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    rom::digests(bytes)
        .sha256
        .iter()
        .fold(String::new(), |mut s, b| {
            write!(s, "{b:02x}").expect("writing to String");
            s
        })
}

/// Runs the six-frame fixture from room-core's
/// `single_frame_tap_never_moves_and_snapshot_replay_is_repeatable` test.
/// Hashes canonical snapshot bytes, not Rust memory layout or JSON.
///
/// # Panics
/// Only if the fixed synthetic fixture becomes invalid under room-core semantics.
#[must_use]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn replay() -> String {
    let room = Room::new(32, 64, vec![0; 2048]).expect("synthetic floor");
    let mut state = WalkingState::new(104, 112);
    let mut snapshots = Vec::new();
    for direction in [
        Some(Direction::Left),
        None,
        None,
        Some(Direction::Down),
        Some(Direction::Down),
        Some(Direction::Down),
    ] {
        state
            .step(&room, FrameInput { direction })
            .expect("synthetic input");
        let snapshot = state.encode_snapshot();
        snapshots.extend_from_slice(&snapshot);
        state = WalkingState::decode_snapshot(&room, &snapshot).expect("snapshot round trip");
    }
    format!("{{\"fixture\":\"walking-six-v1\",\"frames\":6,\"state_sha256\":\"{}\",\"trace_sha256\":\"{}\"}}",
        sha256(&state.encode_snapshot()), sha256(&snapshots))
}

/// Validates JP input and decodes the previously qualified intro Earth packet.
/// Returns JSON metadata only; selected and decoded bytes never cross back to JS.
///
/// # Errors
/// Rejects malformed/unrecognized images, non-Japanese revisions, or bad packets.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn probe(input: &[u8]) -> Result<String, String> {
    let rom = rom::Rom::load(input).map_err(|e| e.to_string())?;
    if rom.revision() != rom::Revision::Japan {
        return Err("this bounded spike requires the Japanese reference".into());
    }
    let packet = assets::compression::decode(&rom.image()[0x2d_0000..], 0x8000)
        .map_err(|e| e.to_string())?;
    Ok(format!("{{\"revision\":\"japan\",\"packet_offset\":2949120,\"consumed\":{},\"decoded_bytes\":{},\"decoded_sha256\":\"{}\",\"replay\":{}}}",
        packet.consumed, packet.data.len(), sha256(&packet.data), replay()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_pins_synthetic_final_state_and_trace() {
        assert_eq!(replay(), concat!(
            "{\"fixture\":\"walking-six-v1\",\"frames\":6,",
            "\"state_sha256\":\"5794285e77e705ad7f20335be388a6efd1b81c9c40a5cc89854042fd74db82e5\",",
            "\"trace_sha256\":\"c4512040a2288ee874b1d9815005bd5be4ea0fd12ba11810b2ce233374911e5a\"}"
        ));
    }

    #[test]
    fn invalid_image_is_not_treated_as_a_packet() {
        assert!(probe(&[]).is_err());
        assert!(probe(&vec![0; rom::Rom::IMAGE_SIZE]).is_err());
    }

    #[test]
    fn local_jp_matches_independently_qualified_packet() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../local/Tenchi Souzou (Japan).sfc");
        let input = match std::fs::read(path) {
            Ok(input) => input,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                eprintln!("skipping local JP probe: owned dump absent");
                return;
            }
            Err(error) => panic!("{error}"),
        };
        let report = probe(&input).unwrap();
        assert!(report.contains("\"consumed\":18112"));
        assert!(report.contains("\"decoded_bytes\":32768"));
        assert!(report.contains("e61b2cb1d7f0e37b9610073a2004513317547e78ade2f978a8a6e9a43f98e4ce"));
        let mut headered = vec![0xa5; rom::Rom::HEADER_SIZE];
        headered.extend_from_slice(&input);
        assert_eq!(probe(&headered).unwrap(), report);
    }
}
