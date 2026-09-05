//! Read-only native BGMODE writer witness; no PPU API or state restoration.
use oracle::{Button, CpuTraceStop, Session};
use serde_json::json;
#[path = "../new-game-qualification/bootstrap.rs"]
mod bootstrap;
fn hash(b: &[u8]) -> String {
    rom::digests(b)
        .sha256
        .iter()
        .map(|v| format!("{v:02x}"))
        .collect()
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 4, "scene-order-probe ROM local/OUT START");
    let out = std::path::Path::new(&args[2]);
    assert!(
        out.starts_with("local")
            && !out
                .components()
                .any(|c| c == std::path::Component::ParentDir)
    );
    std::fs::create_dir(out).unwrap();
    let start: u32 = args[3].parse().unwrap();
    assert!([2340, 6967].contains(&start));
    let rom = rom::Rom::load(&std::fs::read(&args[1]).unwrap()).unwrap();
    assert_eq!(rom.revision(), rom::Revision::Japan);
    let scene = assets::maps::visual::StaticBackground::from_rom(
        rom.image(),
        if start == 2340 { 15 } else { 16 },
    )
    .unwrap();
    std::fs::write(out.join("definitions.bin"), scene.resources()[2].decoded()).unwrap();
    let mut session = Session::new(&rom).unwrap();
    for label in 0..start {
        for b in [Button::Start, Button::A, Button::Down, Button::Right] {
            session.set_button(
                b,
                bootstrap::INPUTS
                    .iter()
                    .chain(&[(Button::Right, 6800, 6862), (Button::Down, 6900, 6967)])
                    .any(|&(button, from, to)| button == b && (from..to).contains(&label)),
            );
        }
        session.run_frame();
    }
    for b in [Button::Start, Button::A, Button::Down, Button::Right] {
        session.set_button(b, false);
    }
    let mut stops = Vec::new();
    for target in [0x868d6b, 0x868d6f, 0x868d72] {
        let trace = session.trace_until_pc(target, 2_000_000, 150).unwrap();
        let regs = session.cpu_registers();
        eprintln!(
            "target {target:06x}, stop {:?}, frames {}, regs {regs:?}",
            trace.stop,
            session.frame_state().frames
        );
        assert_eq!(trace.stop, CpuTraceStop::TargetReached);
        let w = session.wram_image();
        let name = format!("{target:06x}");
        std::fs::write(out.join(format!("{name}.wram")), &w).unwrap();
        let entries: Vec<_> = trace
            .entries
            .iter()
            .flat_map(|e| e.address.to_le_bytes())
            .collect();
        std::fs::write(out.join(format!("{name}.trace-pcs")), &entries).unwrap();
        stops.push(json!({"pc":target,"frame":session.frame_state().frames,"a":regs.accumulator,"x":regs.x,"y":regs.y,"p":regs.status,"db":regs.data_bank,"dp":regs.direct_page,"s":regs.stack,
            "table_mode":rom.image()[0x16bb6a+usize::from(regs.x)],
            "map":u16::from_le_bytes([w[0x47e],w[0x47f]]),"wram_sha256":hash(&w),"trace_pcs_sha256":hash(&entries)}));
    }
    std::fs::write(
        out.join("stops.json"),
        serde_json::to_vec_pretty(&json!({"start":start,"stops":stops})).unwrap(),
    )
    .unwrap();
    std::process::exit(0);
}
