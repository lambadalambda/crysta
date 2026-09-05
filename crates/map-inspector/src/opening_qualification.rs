//! Input-only save-slot-1 Crysta qualification; not a general event interpreter.
use crate::{invalid, sha256, Result, SAVE_SHA256};
use assets::maps::{exits::ExitList, LoadedMap};
use oracle::{Button, CpuTraceStop, Session};
use rom::Rom;
use serde_json::{json, Value};

const FLAGS_SHA: &str = "bf7d61f4953777ab96d024578f0a147a6700c9a75085c45454f9aec69e801fcb";
const CHECKPOINTS: [(u32, u16, (u16, u16), &str); 7] = [
    (
        1601,
        15,
        (472, 176),
        "655ea43073c4a4f7f0f1b8d736125c1a7b62ddbe07af912ee958c3eb162f0dd6",
    ),
    (
        1681,
        15,
        (392, 209),
        "33b89ff30dcd40d909881ff6ab69c8d55c179e354c3af5d0ce406b6345a57855",
    ),
    (
        1697,
        15,
        (392, 225),
        "ed6e01d966e546412d2e896aba2ccb4930df918b81b29894ee05e502465c15e4",
    ),
    (
        1698,
        16,
        (392, 226),
        "59f6125bf7fd9e27a0ee75da9fc7a4879d156b4671a54d276bbe757410416243",
    ),
    (
        1703,
        16,
        (392, 336),
        "9f19770610f6ed761aa99bd25874e34f3b6ad214757f5c32df1c5a7ec1af1ac0",
    ),
    (
        1751,
        16,
        (392, 353),
        "d69f9460cdda16f5a37256333a838aa95840716a9608b3776938daad1a3b1615",
    ),
    (
        1801,
        16,
        (392, 353),
        "22e775d46572211cca57fe9b763499173e16c6aab18f3e5506d0338313098d90",
    ),
];

fn inputs(session: &mut Session, label: u32) {
    for (button, held) in [
        (Button::Start, (400..408).contains(&label)),
        (
            Button::Up,
            (900..908).contains(&label) || (950..958).contains(&label),
        ),
        (Button::A, (1100..1112).contains(&label)),
        (Button::Left, (1601..1657).contains(&label)),
        (Button::Down, (1657..1682).contains(&label)),
    ] {
        session.set_button(button, held);
    }
}
fn advance(session: &mut Session, frame: u32) {
    while session.frame_state().frames < frame {
        inputs(session, session.frame_state().frames);
        session.run_frame();
    }
}

pub(super) fn run(session: &mut Session, rom: &Rom, trace: bool) -> Result<Value> {
    let exits = ExitList::from_rom(rom.image(), 15)?;
    let record = exits
        .select(384, 193)
        .ok_or_else(|| invalid("missing qualified doorway"))?;
    if record.direct_destination()? != 16
        || record.transition_mode() != 0
        || record.selector() != 5
        || record.destination_position() != (384, 336)
    {
        return Err(invalid("unsupported Crysta exit profile").into());
    }
    let range = record.source_range();
    let mut result = json!({
        "schema_version":1,"scenario":"qualified-slot-1-crysta-doorway",
        "revision":rom.revision().id(),"rom_sha256":sha256(rom.image()),"sram_sha256":SAVE_SHA256,
        "frame_convention":"input label F before run_frame; checkpoint N after N calls",
        "inputs":[["Start",400,408],["Up",900,908],["Up",950,958],["A",1100,1112],["Left",1601,1657],["Down",1657,1682]],
        "exit":{"record_source":[range.start,range.end],"record_sha256":sha256(record.bytes()),
            "list_sha256":sha256(exits.source_bytes()),"destination":record.direct_destination()?,
            "mode":record.transition_mode(),"selector":record.selector(),"raw_position":record.destination_position()},
        "limits":"One genuine table-triggered doorway; native actor scripts/COP services, not a complete event VM. No state patches or snapshot restores."
    });
    if trace {
        result["stops"] = trace_stages(session, rom, u16::try_from(range.start & 0xFFFF)?)?.into();
    } else {
        let mut checkpoints = Vec::new();
        for (frame, id, player, expected_hash) in CHECKPOINTS {
            advance(session, frame);
            let wram = session.wram_image();
            let map = LoadedMap::from_wram(&wram)?;
            let hash = sha256(&wram);
            if map.map_id() != id
                || map.player() != player
                || (map.width(), map.height()) != (32, 64)
                || hash != expected_hash
                || sha256(&wram[0x6c0..0x700]) != FLAGS_SHA
            {
                return Err(
                    invalid(&format!("Crysta checkpoint {frame} differs from reference")).into(),
                );
            }
            checkpoints.push(json!({"frame":frame,"map_id":id,"player":player,"camera":map.camera(),
                "wram_sha256":hash,"flags_sha256":sha256(&wram[0x6c0..0x700]),
                "layer_sha256":sha256(&map.layer_bytes()),"pixels_sha256":sha256(session.pixels())}));
        }
        result["checkpoints"] = checkpoints.into();
    }
    Ok(result)
}

fn trace_stages(session: &mut Session, rom: &Rom, record_pointer: u16) -> Result<Vec<Value>> {
    advance(session, 1678);
    // Entry lookup only: this room has conditional loading branches.
    let bytes = &rom.image()[0x06_959C + 16 * 3..][..3];
    let entry =
        rom::RuntimeRomAddress::new(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], 0]))?.value();
    let mut stops = Vec::new();
    for (pc, frame) in [
        (0x8d_883e, 1680),
        (0x8d_884c, 1680),
        (0x8d_888d, 1680),
        (0x8d_88df, 1681),
        (0x8d_8720, 1697),
        (0x86_902b, 1698),
        (0x80_f3f1, 1702),
        (0x84_a12e, 1711),
    ] {
        if pc == 0x8d_8720 {
            advance(session, 1682);
            inputs(session, 1682);
        }
        let trace = session.trace_until_pc(pc, 2_000_000, 80)?;
        if trace.stop != CpuTraceStop::TargetReached || session.frame_state().frames != frame {
            return Err(
                invalid(&format!("Crysta stage ${pc:06X} missed: {:?}", trace.stop)).into(),
            );
        }
        let r = session.cpu_registers();
        let w = session.wram_image();
        let word = |i| u16::from_le_bytes([w[i], w[i + 1]]);
        let pointer = |i| u32::from_le_bytes([w[i], w[i + 1], w[i + 2], 0]);
        let queued = (word(0x492), word(0x494));
        let valid = match pc {
            0x8d_883e => {
                r.x == record_pointer
                    && r.data_bank == 0x81
                    && (word(0x95e), word(0x960)) == (384, 193)
            }
            0x8d_884c => r.x == record_pointer && r.y == 16 && word(0x47c) == 0,
            0x8d_888d => r.x == 12 && word(0x47c) == 16 && word(0x490) == 5 && queued == (384, 336),
            0x8d_88df => word(0x47c) == 16 && queued == (384, 336),
            0x8d_8720 => {
                r.y == 16
                    && word(0x47e) == 15
                    && word(0x482) == 15
                    && queued == (384, 320)
                    && pointer(0x100a) == 0x84_b975
            }
            0x86_902b => word(0x47c) == 0 && word(0x47e) == 16 && pointer(0x62) == entry,
            0x80_f3f1 => word(0x47e) == 16 && (word(0x1000), word(0x1002)) == (0, 0),
            0x84_a12e => {
                r.x == 0x1000 && (word(0x1000), word(0x1002)) == (392, 336) && word(0x490) == 0
            }
            _ => false,
        };
        if !valid || sha256(&w[0x6c0..0x700]) != FLAGS_SHA {
            return Err(invalid(&format!(
                "decoded exit disagrees with Crysta stage ${pc:06X}"
            ))
            .into());
        }
        stops.push(json!({"pc":pc,"frame":frame,"trace_sha256":trace.digest_hex(),
            "instructions":trace.entries.len(),"x":r.x,"y":r.y,"data_bank":r.data_bank,
            "map_id":word(0x47e),"pending_map":word(0x47c),"previous_map":word(0x482),
            "queued_position":queued,"player":[word(0x1000),word(0x1002)],"player_resume":pointer(0x100a)}));
    }
    Ok(stops)
}
