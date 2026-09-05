//! Optional owned-JP-ROM experiment. Run through replay.sh from repository root.
//! Exactly one Session::new, default empty SRAM, no restores or mutations.
use oracle::{export::digest_hex, Button, Session};
use serde_json::json;
mod bootstrap;
mod native_trace;
use std::io::Write;

const INPUTS: &[(Button, u32, u32)] = &[(Button::Right, 6800, 6820), (Button::Down, 6900, 6920)];
const CHECKPOINTS: &[(u32, &str)] = &[
    (800, "empty-load-menu-new-game-selected"),
    (1100, "kana-grid-default-ark"),
    (1800, "prologue-title"),
    (2500, "bedroom-intro"),
    (3500, "elle-dialogue-1"),
    (4100, "ark-dialogue"),
    (4700, "elle-dialogue-2"),
    (5300, "elle-dialogue-3"),
    (5900, "elle-dialogue-final"),
    (6800, "intro-done-neutral"),
    (6900, "right-released-neutral"),
    (7000, "down-released-neutral"),
    (7100, "control-checkpoint"),
];

fn word(wram: &[u8], address: usize) -> u16 {
    u16::from_le_bytes([wram[address], wram[address + 1]])
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert!(
        args.len() == 3 || args.len() == 4,
        "usage: probe ROM OUT [no-confirm|no-movement|trace-reset|trace-default|trace-map|trace-spawn|trace-release]"
    );
    let mode = args.get(3).map_or("normal", String::as_str);
    let trace_plan = native_trace::plan(mode);
    assert!(["normal", "no-confirm", "no-movement"].contains(&mode) || trace_plan.is_some());
    let out = std::path::Path::new(&args[2]);
    // Do not overwrite prior evidence, or allow accidental non-local asset export.
    assert!(
        out.starts_with("local")
            && !out
                .components()
                .any(|c| c == std::path::Component::ParentDir)
    );
    std::fs::create_dir(out).expect("new output directory beneath local/");
    let rom = rom::Rom::load(&std::fs::read(&args[1]).expect("read owned ROM"))
        .expect("authenticate ROM");
    assert_eq!(rom.revision(), rom::Revision::Japan);
    let mut session = Session::new(&rom).expect("fresh empty-SRAM session");
    let mut csv = std::io::BufWriter::new(std::fs::File::create(out.join("frames.csv")).unwrap());
    writeln!(csv, "frame,map,x,y,flags,flags8,resume,timer,joy,edge,state,intro,gate,input_filter,gate2,aux,player_index,controller_index").unwrap();
    let mut checkpoints = Vec::new();
    for label in 0..trace_plan.map_or(7100, |(end, _)| end) {
        for button in [Button::Start, Button::Down, Button::A, Button::Right] {
            let pressed = bootstrap::INPUTS
                .iter()
                .chain(INPUTS)
                .any(|&(b, start, end)| {
                    b == button
                        && (start..end).contains(&label)
                        && !(mode == "no-confirm" && start == 1200)
                        && !(mode == "no-movement" && start >= 6800)
                });
            session.set_button(button, pressed);
        }
        session.run_frame();
        let frame = label + 1;
        assert_eq!(session.frame_state().frames, frame);
        let w = session.wram_image();
        let u = |a| word(&w, a);
        let resume = u32::from(u(0x100A)) | (u32::from(w[0x100C]) << 16);
        writeln!(
            csv,
            "{frame},{},{},{},{},{},{resume},{},{},{},{},{},{},{},{},{},{},{}",
            u(0x47E),
            u(0x1000),
            u(0x1002),
            u(0x1004),
            u(0x1008),
            u(0x100E),
            u(0x454),
            u(0x456),
            u(0x450),
            w[0x6C4],
            u(0x97C),
            u(0x45E),
            u(0x97E),
            u(0x1301E),
            u(0xDEA),
            u(0xDEE)
        )
        .unwrap();
        if let Some(&(_, name)) = CHECKPOINTS.iter().find(|&&(n, _)| n == frame) {
            let vram: Vec<_> = session
                .vram()
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect();
            checkpoints.push(json!({
                "frame": frame, "name": name, "map": u(0x47E), "player": [u(0x1000), u(0x1002)],
                "program_state": u(0x450), "actor_flags": [u(0x1004), u(0x1008)],
                "actor_resume": resume, "actor_timer": u(0x100E),
                "input_held": u(0x454), "input_edge": u(0x456), "input_filter": u(0x45E),
                "gate_097c": u(0x97C), "gate_097e": u(0x97E), "player_aux_7f301e": u(0x1301E),
                "player_index": u(0xDEA), "controller_index": u(0xDEE),
                "controller_resume": u32::from(u(usize::from(u(0xDEE)) + 10)) | (u32::from(w[usize::from(u(0xDEE)) + 12]) << 16),
                "event_flags_nonzero": w[0x6C0..0x700].iter().enumerate().filter(|(_, b)| **b != 0).collect::<Vec<_>>(),
                "wram_sha256": digest_hex(&w), "vram_le_sha256": digest_hex(&vram),
                "framebuffer_xrgb_sha256": digest_hex(session.pixels()),
                "event_flags_sha256": digest_hex(&w[0x6C0..0x700]),
                "player_entity_sha256": digest_hex(&w[0x1000..0x1040]),
            }));
            std::fs::write(out.join(format!("f{frame}.wram")), &w).unwrap();
            std::fs::write(out.join(format!("f{frame}.pixels")), session.pixels()).unwrap();
        }
    }
    csv.flush().unwrap();
    if trace_plan.is_some() {
        native_trace::capture(&mut session, out, mode);
        std::process::exit(0);
    }
    let report = json!({
        "version": 1, "rom_sha256": digest_hex(rom.image()),
        "boot": "Session::new / default zero SRAM / one session per process",
        "frames_sha256": digest_hex(&std::fs::read(out.join("frames.csv")).unwrap()),
        "checkpoints": checkpoints,
    });
    std::fs::write(
        out.join("checkpoints.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    // ares is process-global; do not attempt teardown/reboot through Drop.
    std::process::exit(0);
}
