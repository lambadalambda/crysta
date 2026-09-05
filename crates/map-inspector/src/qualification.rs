//! Fixed cavern-loader experiment. Raw memory and instruction traces stay local.
use crate::{invalid, sha256, Result, SAVE_SHA256};
use assets::{
    compression,
    maps::{LoadedMap, StaticLayer},
};
use oracle::{Button, CpuTraceStop, Session};
use rom::Rom;
use serde_json::{json, Value};

pub(super) fn run(session: &mut Session, rom: &Rom) -> Result<Value> {
    let layer = StaticLayer::from_rom(rom.image(), 0x09_0000)?;
    let packet = compression::decode(&rom.image()[0x2B_439E..0x2C_0000], 512)?;
    let attributes: &[u8; 512] = packet.data.as_slice().try_into()?;
    let initialized: Vec<_> = layer
        .attributed_cells(attributes)
        .iter()
        .flat_map(|cell| cell.raw().to_le_bytes())
        .collect();
    let raw = layer.layer_bytes();
    for label in 0..1112 {
        session.set_button(Button::Start, (400..408).contains(&label));
        session.set_button(Button::A, (1100..1112).contains(&label));
        session.run_frame();
    }
    session.set_button(Button::A, false);
    let mut stops = Vec::new();
    for pc in [
        0x86_86ED, 0x86_902B, 0x86_8A40, 0x86_8A44, 0x86_8A68, 0x86_8ADA, 0x86_8B66, 0x86_8B6A,
        0x86_9332, 0x86_936C,
    ] {
        let trace = session.trace_until_pc(pc, 2_000_000, 80)?;
        if trace.stop != CpuTraceStop::TargetReached {
            return Err(
                invalid(&format!("loader did not reach ${pc:06X}: {:?}", trace.stop)).into(),
            );
        }
        let wram = session.wram_image();
        let map_id = u16::from_le_bytes([wram[0x47E], wram[0x47F]]);
        if map_id != 0x128 {
            return Err(invalid("loader trace left the qualified map $0128").into());
        }
        let pointer =
            |offset| u32::from_le_bytes([wram[offset], wram[offset + 1], wram[offset + 2], 0]);
        // Validate each content-producing stage, not just the final framebuffer.
        let matches = match pc {
            0x86_8A40 => pointer(0x66) == 0xEB_439E && pointer(0xB9) == 0x7E_5000,
            0x86_8A44 => &wram[0x5000..0x5200] == attributes,
            0x86_8A68 | 0x86_9332 => &wram[0x10000..0x10200] == attributes,
            0x86_8ADA => pointer(0x66) == 0xC9_0000,
            0x86_8B66 => pointer(0x66) == 0xC9_0002 && pointer(0xB9) == 0x7E_A000,
            0x86_8B6A => wram[0xA000..0xB400] == raw,
            0x86_936C => wram[0xA000..0xB400] == initialized,
            _ => true,
        };
        if !matches {
            return Err(invalid(&format!(
                "static data disagrees with loader stage ${pc:06X}"
            ))
            .into());
        }
        let entry = trace.entries.last().expect("target entry included");
        stops.push(json!({
            "pc":pc,"frame":session.frame_state().frames,"instructions":trace.entries.len(),
            "trace_sha256":trace.digest_hex(),"direct_page":entry.direct_page,
            "data_bank":entry.data_bank,"status":entry.status,"map_id":map_id,
            "script_pointer":pointer(0x62),"source_pointer":pointer(0x66),
            "destination_pointer":pointer(0xB9),"copy_destination":pointer(0x6A)
        }));
    }
    while session.frame_state().frames < 1601 {
        session.run_frame();
    }
    let map = LoadedMap::from_wram(&session.wram_image())?;
    if map.map_id() != 0x128
        || (map.width(), map.height()) != (80, 32)
        || map.player() != (776, 112)
        || session.frame_state().frames != 1601
    {
        return Err(invalid("loader replay missed the qualified final checkpoint").into());
    }
    let differences: Vec<_> = layer
        .attributed_cells(attributes)
        .iter()
        .zip(map.cells())
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(index, (a, b))| json!({"index":index,"initialized":a.raw(),"runtime":b.raw()}))
        .collect();
    Ok(json!({
        "schema_version":1,"scenario":"qualified-cavern-loader","revision":rom.revision().id(),
        "rom_sha256":sha256(rom.image()),"sram_sha256":SAVE_SHA256,"map_id":map.map_id(),
        "layer_source":[layer.source_range().start,layer.source_range().end],
        "attribute_source":[0x2B_439E,0x2B_439E + packet.consumed],
        "static_sha256":sha256(&raw),"attribute_sha256":sha256(attributes),
        "attributed_sha256":sha256(&initialized),"runtime_sha256":sha256(&map.layer_bytes()),
        "runtime_differences":differences,"stops":stops
    }))
}
