//! D's initialized source pose before its first AI instruction. Not an OAM latch.
use oracle::{Button, CpuTraceStop, Session};
use serde_json::{Value, json};
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
    let a: Vec<_> = std::env::args().collect();
    assert_eq!(a.len(), 3, "setup-probe ROM local/OUT");
    let out = std::path::Path::new(&a[2]);
    assert!(
        out.starts_with("local")
            && !out
                .components()
                .any(|c| c == std::path::Component::ParentDir)
    );
    std::fs::create_dir(out).unwrap();
    let r = rom::Rom::load(&std::fs::read(&a[1]).unwrap()).unwrap();
    assert_eq!(r.revision(), rom::Revision::Japan);
    let mut s = Session::new(&r).unwrap();
    let buttons = [
        (Button::Start, "Start"),
        (Button::A, "A"),
        (Button::B, "B"),
        (Button::Up, "Up"),
        (Button::Down, "Down"),
        (Button::Left, "Left"),
        (Button::Right, "Right"),
    ];
    for label in 0..6800 {
        for (b, _) in buttons {
            s.set_button(
                b,
                bootstrap::INPUTS
                    .iter()
                    .any(|&(v, from, to)| v == b && (from..to).contains(&label)),
            );
        }
        s.run_frame();
    }
    let itinerary = std::fs::read_to_string("tools/house-scene-qualification/route.jsonl").unwrap();
    for line in itinerary.lines() {
        let c: Value = serde_json::from_str(line).unwrap();
        for (b, name) in buttons {
            s.set_button(
                b,
                c["buttons"].as_array().unwrap().iter().any(|v| v == name),
            );
        }
        for _ in 0..c["frames"].as_u64().unwrap() {
            s.run_frame();
        }
        if c["label"] == "southC" {
            break;
        }
    }
    assert_eq!(s.frame_state().frames, 8075);
    for (b, _) in buttons {
        s.set_button(b, false);
    }
    let mut selected = None;
    let mut initial_trace = None;
    let mut skipped = Vec::new();
    let mut reports = Vec::new();
    for _ in 0..24 {
        let trace = s.trace_until_pc(0x80f5d3, 2_000_000, 150).unwrap();
        assert_eq!(trace.stop, CpuTraceStop::TargetReached);
        let w = s.wram_image();
        let u = |p| u16::from_le_bytes([w[p], w[p + 1]]);
        let e = usize::from(s.cpu_registers().x);
        let cursor = u32::from_le_bytes([w[0x6e], w[0x6f], w[0x70], 0]);
        if u(0x47e) == 13 && cursor == 0x838cbe && u(e + 10) == 0xa83c && w[e + 12] == 0x88 {
            selected = Some(e);
            initial_trace = Some(trace);
            break;
        }
        skipped.push(json!({"entity":e,"map":u(0x47e),"cursor":cursor}));
        assert_eq!(
            s.trace_until_pc(0x80f5d7, 200_000, 10).unwrap().stop,
            CpuTraceStop::TargetReached
        );
    }
    let entity = selected.expect("D source record was not initialized");
    for target in [0x80f5d3, 0x80ed99, 0x80ee11, 0x80f5e4, 0x88a83c] {
        let trace = if target == 0x80f5d3 {
            initial_trace.take().unwrap()
        } else {
            s.trace_until_pc(target, 500_000, 50).unwrap()
        };
        assert_eq!(trace.stop, CpuTraceStop::TargetReached);
        let regs = s.cpu_registers();
        let w = s.wram_image();
        let pcs: Vec<_> = trace
            .entries
            .iter()
            .flat_map(|e| e.address.to_le_bytes())
            .collect();
        let name = format!("{target:06x}");
        std::fs::write(out.join(format!("{name}.wram")), &w).unwrap();
        std::fs::write(out.join(format!("{name}.pcs")), &pcs).unwrap();
        reports.push(json!({"pc":target,"frame":s.frame_state().frames,"a":regs.accumulator,"x":regs.x,"y":regs.y,"db":regs.data_bank,"dp":regs.direct_page,"p":regs.status,"wram_sha256":hash(&w),"pcs_sha256":hash(&pcs)}));
    }
    std::fs::write(
        out.join("setup.json"),
        serde_json::to_vec_pretty(&json!({"entity":entity,"skipped":skipped,"stops":reports}))
            .unwrap(),
    )
    .unwrap();
    std::process::exit(0);
}
