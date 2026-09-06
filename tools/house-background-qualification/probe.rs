//! Read-only hardware writer stops on the existing empty-SRAM, finite input route.
use oracle::{Button, CpuTraceStop, Session};
use serde_json::{json, Value};
#[path = "../new-game-qualification/bootstrap.rs"]
mod bootstrap;
const BUTTONS: [(Button, &str); 7] = [
    (Button::Start, "Start"),
    (Button::A, "A"),
    (Button::B, "B"),
    (Button::Up, "Up"),
    (Button::Down, "Down"),
    (Button::Left, "Left"),
    (Button::Right, "Right"),
];
fn hash(bytes: &[u8]) -> String {
    rom::digests(bytes)
        .sha256
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn main() {
    let a: Vec<_> = std::env::args().collect();
    assert_eq!(a.len(), 5, "probe ROM local/OUT ROUTE.jsonl SEGMENT");
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
    let segment = a[4].as_str();
    assert!([
        "boot",
        "room10",
        "settledC",
        "north-door-push",
        "settledD",
        "settled11"
    ]
    .contains(&segment));
    let boot_end = if segment == "boot" { 2340 } else { 6800 };
    for frame in 0..boot_end {
        for (button, _) in BUTTONS {
            s.set_button(
                button,
                bootstrap::INPUTS
                    .iter()
                    .any(|&(b, start, end)| b == button && (start..end).contains(&frame)),
            );
        }
        s.run_frame();
    }
    let mut budget = 100;
    if segment != "boot" {
        let script = std::fs::read_to_string(&a[3]).unwrap();
        let commands: Vec<Value> = script
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(commands.last().unwrap()["finish"], true);
        let mut found = false;
        for c in &commands[..commands.len() - 1] {
            let frames = c["frames"].as_u64().unwrap();
            assert!(frames > 0 && frames <= 2000);
            let buttons = c["buttons"].as_array().unwrap();
            assert!(buttons
                .iter()
                .all(|v| BUTTONS.iter().any(|(_, name)| v == name)));
            for (button, name) in BUTTONS {
                s.set_button(button, buttons.iter().any(|v| v == name));
            }
            if c["label"] == segment {
                budget = frames as u32;
                found = true;
                break;
            }
            s.run_frames(frames as usize);
        }
        assert!(found);
    }
    let start = s.frame_state().frames;
    let mut stops = Vec::new();
    for pc in [0x868d6b, 0x868d6f, 0x868d72] {
        let trace = s.trace_until_pc(pc, 2_000_000, budget).unwrap();
        assert_eq!(trace.stop, CpuTraceStop::TargetReached);
        assert!(
            s.frame_state().frames < start + budget,
            "trace crossed next input edge"
        );
        let regs = s.cpu_registers();
        let w = s.wram_image();
        let pcs: Vec<_> = trace
            .entries
            .iter()
            .flat_map(|e| e.address.to_le_bytes())
            .collect();
        let word = |p| u16::from_le_bytes([w[p], w[p + 1]]);
        std::fs::write(out.join(format!("{pc:06x}.wram")), &w).unwrap();
        std::fs::write(out.join(format!("{pc:06x}.pcs")), &pcs).unwrap();
        stops.push(json!({"pc":pc,"frame":s.frame_state().frames,"map":word(0x47e),"a":regs.accumulator,"x":regs.x,"p":regs.status,"db":regs.data_bank,"dp":regs.direct_page,"wram_sha256":hash(&w),"pcs_sha256":hash(&pcs)}));
    }
    std::fs::write(
        out.join("stops.json"),
        serde_json::to_vec_pretty(&json!({"segment":segment,"start":start,"stops":stops})).unwrap(),
    )
    .unwrap();
    std::process::exit(0);
}
