//! Single empty-SRAM session; stdin is a retained, real-button JSONL itinerary.
//! Checkpoint saves synchronize the oracle, then every observation is passive.
use oracle::{Button, Session};
use serde_json::{json, Value};
use std::io::{BufRead, Write};
#[path = "../new-game-qualification/bootstrap.rs"]
mod bootstrap;
const BUTTONS: [(Button, &str); 12] = [
    (Button::Start, "Start"),
    (Button::Select, "Select"),
    (Button::A, "A"),
    (Button::B, "B"),
    (Button::X, "X"),
    (Button::Y, "Y"),
    (Button::L, "L"),
    (Button::R, "R"),
    (Button::Up, "Up"),
    (Button::Down, "Down"),
    (Button::Left, "Left"),
    (Button::Right, "Right"),
];
fn observation(s: &Session, label: &str, kind: &str) {
    let w = s.wram_image();
    let u = |p| u16::from_le_bytes([w[p], w[p + 1]]);
    let actors: Vec<_> = (0x1040..0x2000).step_by(0x40).filter(|&p| u(p+10)!=0).map(|p| json!({"slot":p,"position":[u(p),u(p+2)],"flags":u(p+4),"script":u(p+10) as u32+((w[p+12] as u32)<<16)})).collect();
    println!(
        "{}",
        json!({"kind":kind,"label":label,"frame":s.frame_state().frames,
        "map":u(0x47e),"position":[u(0x1000),u(0x1002)],"camera":[u(0x81e),u(0x822)],
        "script":u(0x100a) as u32+((w[0x100c] as u32)<<16),"facing":u(0x1014),"flags":u(0x1004),"control":u(0x980),
        "events":(0..512).filter(|i| w[0x6c0+i/8]>>(i%8)&1!=0).collect::<Vec<_>>(),"actors":actors})
    );
}
fn capture(s: &Session, out: &std::path::Path, label: &str) {
    std::fs::write(out.join(format!("{label}.state")), oracle::save_state(s)).unwrap();
    std::fs::write(out.join(format!("{label}.wram")), s.wram_image()).unwrap();
    for (name, words) in [("vram", s.vram()), ("cgram", s.cgram())] {
        std::fs::write(
            out.join(format!("{label}.{name}")),
            words
                .iter()
                .flat_map(|w| w.to_le_bytes())
                .collect::<Vec<_>>(),
        )
        .unwrap();
    }
    std::fs::write(out.join(format!("{label}.pixels")), s.pixels()).unwrap();
    let hw = s.sprite_state();
    std::fs::write(out.join(format!("{label}.oam")), hw.oam).unwrap();
    std::fs::write(
        out.join(format!("{label}.obj")),
        [hw.obsel, hw.first_sprite],
    )
    .unwrap();
    observation(s, label, "checkpoint");
    std::io::stdout().flush().unwrap();
}
fn main() {
    let a: Vec<_> = std::env::args().collect();
    assert_eq!(a.len(), 3, "probe ROM local/OUT < ROUTE.jsonl");
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
    for frame in 0..6800 {
        for (b, _) in BUTTONS {
            s.set_button(
                b,
                bootstrap::INPUTS
                    .iter()
                    .any(|&(v, start, end)| v == b && (start..end).contains(&frame)),
            );
        }
        s.run_frame();
    }
    capture(&s, out, "boot");
    let mut route = std::fs::File::create(out.join("route.jsonl")).unwrap();
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let c: Value = serde_json::from_str(&line).unwrap();
        writeln!(route, "{line}").unwrap();
        route.flush().unwrap();
        if c["finish"] == true {
            std::io::stdout().flush().unwrap();
            std::process::exit(0);
        }
        let label = c["label"].as_str().unwrap();
        assert!(
            !label.is_empty()
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        );
        assert!(
            !out.join(format!("{label}.state")).exists(),
            "duplicate checkpoint"
        );
        let frames = c["frames"].as_u64().unwrap();
        assert!(frames > 0 && frames <= 2000);
        let buttons = c["buttons"].as_array().unwrap();
        assert!(buttons
            .iter()
            .all(|v| BUTTONS.iter().any(|(_, name)| v == name)));
        for (b, name) in BUTTONS {
            s.set_button(b, buttons.iter().any(|v| v == name));
        }
        for _ in 0..frames {
            s.run_frame();
            observation(&s, label, "frame");
        }
        capture(&s, out, label);
    }
    panic!("itinerary missing finish");
}
