//! Source-backed room-only New Game; intro presentation is semantically omitted.
//! No SRAM, captures, original CPU, or general actor interpreter is involved.

use crate::{invalid, Result};
use serde_json::Value;
use std::collections::BTreeMap;

const SOURCES: &str = include_str!("../../../tools/new-game-qualification/sources.json");

pub(super) struct Startup {
    pub(super) position: (u16, u16),
    pub(super) events: [u8; 64],
}

/// Compile only the qualified Japanese room projection, including semantic
/// completion of the intro. Inventory, NPCs and presentation remain outside it.
pub(super) fn compile(rom: &rom::Rom) -> Result<Startup> {
    let sources = authenticate(rom.image(), SOURCES)?;
    let source = |name: &str| {
        sources
            .get(name)
            .copied()
            .ok_or_else(|| invalid(&format!("missing New Game source: {name}")))
    };
    let queued = queue(source("prologue-house-request")?)?;
    ensure(
        word(source("map-f-actor-bank82")?, 0)? == 0
            && word(source("map-f-actor-bank83")?, 0)? == 0x8D1E,
        "New Game actor table selection changed",
    )?;
    let record = source("map-f-first-actor")?;
    // $83:8D1E's two-byte prefix precedes this FD record. Its three-byte
    // pointer identifies the five-byte player header at $84:A129.
    ensure(
        record.len() == 7 && record[0] == 0xFD && record[4..] == [0x29, 0xA1, 0x84],
        "New Game player FD record changed",
    )?;
    let header = source("player-header")?;
    ensure(
        header.len() == 5 && header[0] == 0 && word(header, 1)? & 0x0400 != 0,
        "New Game player header/selector changed",
    )?;
    let position = spawn((record[1], record[2]), queued);

    // $87:CCA7 clears $0600..07FF; $86:B93F's default table does not
    // reseed $06C0..0700. Both code and data are authenticated above.
    let mut events = reset_events(source("reset-default-table")?)?;
    for name in ["new-game-default-event", "intro-release-event"] {
        let record = source(name)?;
        ensure(
            record.len() == 4 && record[..2] == [2, 7],
            "expected New Game COP 07",
        )?;
        assign_event(&mut events, word(record, 2)?)?;
    }
    Ok(Startup { position, events })
}

fn ensure(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(invalid(message).into())
    }
}

fn text(value: &Value) -> Result<&str> {
    value
        .as_str()
        .ok_or_else(|| invalid("invalid New Game source metadata string").into())
}

fn digest(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    rom::digests(bytes)
        .sha256
        .iter()
        .fold(String::with_capacity(64), |mut text, byte| {
            write!(text, "{byte:02x}").expect("writing to String");
            text
        })
}

fn authenticate<'a>(image: &'a [u8], metadata: &str) -> Result<BTreeMap<String, &'a [u8]>> {
    let metadata: Value = serde_json::from_str(metadata)?;
    ensure(
        digest(image) == text(&metadata["rom_sha256"])?,
        "New Game requires the authenticated Japanese ROM",
    )?;
    let ranges = metadata["ranges"]
        .as_array()
        .ok_or_else(|| invalid("missing New Game source ranges"))?;
    let mut sources = BTreeMap::new();
    for item in ranges {
        let name = text(&item["name"])?;
        let cpu = u32::from_str_radix(text(&item["cpu_start"])?, 16)?;
        let cpu_end = u32::from_str_radix(text(&item["cpu_end_exclusive"])?, 16)?;
        let start = usize::try_from(
            item["normalized_start"]
                .as_u64()
                .ok_or_else(|| invalid("invalid New Game source offset"))?,
        )?;
        ensure(
            start == usize::try_from(cpu & 0x3F_FFFF)? && cpu_end > cpu,
            "invalid New Game source extent",
        )?;
        let end = start
            .checked_add(usize::try_from(cpu_end - cpu)?)
            .ok_or_else(|| invalid("New Game source extent overflow"))?;
        let data = image
            .get(start..end)
            .ok_or_else(|| invalid("truncated New Game source"))?;
        ensure(
            digest(data) == text(&item["sha256"])?,
            &format!("New Game source hash mismatch: {name}"),
        )?;
        ensure(
            sources.insert(name.to_owned(), data).is_none(),
            "duplicate New Game source name",
        )?;
    }
    Ok(sources)
}

fn word(data: &[u8], offset: usize) -> Result<u16> {
    let bytes = data
        .get(offset..offset + 2)
        .ok_or_else(|| invalid("truncated New Game operand"))?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

/// $80:8A23..8A57 consumes COP 14's map, mode, selector and queued coordinates.
fn queue(request: &[u8]) -> Result<(u16, u16)> {
    ensure(
        request.len() == 10 && request[..6] == [2, 0x14, 15, 0, 1, 0],
        "unqualified New Game map/mode/selector",
    )?;
    Ok((word(request, 6)?, word(request, 8)?))
}

/// FD default, then $80:F7F3's (+8,+16) override. The local queue is consumed;
/// no pending coordinates are retained in the compiled startup state.
fn spawn(tile: (u8, u8), queue: (u16, u16)) -> (u16, u16) {
    if queue == (0, 0) {
        (u16::from(tile.0) * 16 + 8, u16::from(tile.1) * 16)
    } else {
        (queue.0.wrapping_add(8), queue.1.wrapping_add(16))
    }
}

fn reset_events(defaults: &[u8]) -> Result<[u8; 64]> {
    ensure(
        defaults.len() % 4 == 2 && defaults.ends_with(&[0xFF, 0xFF]),
        "invalid New Game default table",
    )?;
    for pair in defaults[..defaults.len() - 2].chunks_exact(4) {
        let address = word(pair, 0)?;
        ensure(
            address < 0x8000 && !(0x06BF..0x0700).contains(&address),
            "New Game default table unexpectedly writes events or terminates early",
        )?;
    }
    Ok([0; 64])
}

/// COP 07 -> $80:BB77: low12 selects the event, bit15 sets (otherwise clears).
fn assign_event(events: &mut [u8; 64], operand: u16) -> Result<()> {
    let index = operand & 0x0FFF;
    let byte = events
        .get_mut(usize::from(index >> 3))
        .ok_or_else(|| invalid("New Game event outside room projection"))?;
    let mask = 1 << (index & 7);
    if operand & 0x8000 != 0 {
        *byte |= mask;
    } else {
        *byte &= !mask;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_set_clear_preserves_other_bits_and_checks_bounds() {
        let mut events = [0; 64];
        assign_event(&mut events, 0x80FB).unwrap();
        assign_event(&mut events, 0x8020).unwrap();
        assert_eq!(events[31], 8);
        assert_eq!(events[4], 1);
        let before = events;
        assign_event(&mut events, 0x80FB).unwrap();
        assert_eq!(events, before, "setting a set bit is idempotent");
        assert!(assign_event(&mut events, 0x8200).is_err());
        assert_eq!(events, before, "invalid event must not mutate state");
        assign_event(&mut events, 0x0020).unwrap();
        assert_eq!(events[4], 0);
        assert_eq!(events[31], 8);
        assert_eq!(events.iter().filter(|&&b| b != 0).count(), 1);
    }

    #[test]
    fn queue_overrides_default_even_when_only_one_axis_is_nonzero() {
        assert_eq!(spawn((19, 7), (296, 96)), (304, 112));
        assert_eq!(spawn((19, 7), (0, 0)), (312, 112));
        assert_eq!(spawn((1, 2), (0, 96)), (8, 112));
        assert_eq!(spawn((1, 2), (u16::MAX, u16::MAX)), (7, 15));
    }

    #[test]
    fn reset_defaults_must_not_reseed_projected_events() {
        assert_eq!(
            reset_events(&[0x00, 0x06, 1, 0, 0xFF, 0xFF]).unwrap(),
            [0; 64]
        );
        for bad in [
            vec![],
            vec![0xFF],
            vec![0, 0],
            vec![0xBF, 0x06, 1, 0, 0xFF, 0xFF], // word straddles $06C0
            vec![0xFF, 0x06, 1, 0, 0xFF, 0xFF],
        ] {
            assert!(reset_events(&bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn request_admits_only_the_qualified_map_mode_and_selector() {
        let request = [2, 0x14, 15, 0, 1, 0, 40, 1, 96, 0];
        assert_eq!(queue(&request).unwrap(), (296, 96));
        assert!(queue(&request[..9]).is_err());
        for index in 0..6 {
            let mut bad = request;
            bad[index] ^= 1;
            assert!(queue(&bad).is_err(), "field {index}");
        }
    }

    #[test]
    fn authentication_rejects_image_range_hash_and_extent_changes() {
        let hash = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        let metadata = serde_json::json!({"rom_sha256": hash, "ranges": [{
            "name": "synthetic", "cpu_start": "800000", "cpu_end_exclusive": "800003",
            "normalized_start": 0, "sha256": hash
        }]});
        assert_eq!(
            authenticate(b"abc", &metadata.to_string()).unwrap()["synthetic"],
            b"abc"
        );
        assert!(authenticate(b"abd", &metadata.to_string()).is_err());
        for (field, value) in [
            ("sha256", serde_json::json!("wrong")),
            ("normalized_start", serde_json::json!(1)),
            ("cpu_end_exclusive", serde_json::json!("800004")),
            ("cpu_end_exclusive", serde_json::json!("7fffff")),
            ("name", Value::Null),
        ] {
            let mut changed = metadata.clone();
            changed["ranges"][0][field] = value;
            assert!(
                authenticate(b"abc", &changed.to_string()).is_err(),
                "{field}"
            );
        }
        let mut duplicate = metadata.clone();
        duplicate["ranges"]
            .as_array_mut()
            .unwrap()
            .push(metadata["ranges"][0].clone());
        assert!(authenticate(b"abc", &duplicate.to_string()).is_err());
    }

    #[test]
    fn owned_japanese_rom_compiles_without_sram_or_captures() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../local/Tenchi Souzou (Japan).sfc");
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                eprintln!(
                    "skipping optional owned-ROM compiler test: {}",
                    path.display()
                );
                return;
            }
            Err(error) => panic!("{}: {error}", path.display()),
        };
        let rom = rom::Rom::load(&bytes).unwrap();
        let startup = compile(&rom).unwrap();
        assert_eq!(startup.position, (304, 112));
        let expected = {
            let mut events = [0; 64];
            events[4] = 1;
            events[31] = 8;
            events
        };
        assert_eq!(startup.events, expected);
        assert_eq!(
            digest(&startup.events),
            "6c9f094ecf92d1e5c904aa8d4c2e301ba7c0b195adb86158722a62d530a6797b"
        );
    }
}
