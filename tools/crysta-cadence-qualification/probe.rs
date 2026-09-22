//! Input-only census: shared fresh bootstrap, then a recorded real-button itinerary.
use oracle::{Button, Session};
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
fn capture(s: &Session, out: &std::path::Path, label: &str) {
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
    let w = s.wram_image();
    let u = |p| u16::from_le_bytes([w[p], w[p + 1]]);
    println!(
        "{}",
        json!({"label":label,"frame":s.frame_state().frames,"map":u(0x47e),"position":[u(0x1000),u(0x1002)],"camera":[u(0x81e),u(0x822)]})
    );
}
fn main() {
    let a: Vec<_> = std::env::args().collect();
    assert_eq!(a.len(), 4, "probe ROM local/OUT ROUTE.jsonl");
    let out = std::path::Path::new(&a[2]);
    assert!(
        out.starts_with("local")
            && !out
                .components()
                .any(|c| c == std::path::Component::ParentDir)
    );
    std::fs::create_dir(out).unwrap();
    let rom = rom::Rom::load(&std::fs::read(&a[1]).unwrap()).unwrap();
    assert_eq!(rom.revision(), rom::Revision::Japan);
    let mut s = Session::new(&rom).unwrap();
    for label in 0..6800 {
        for (b, _) in BUTTONS {
            s.set_button(
                b,
                bootstrap::INPUTS
                    .iter()
                    .any(|&(v, start, end)| v == b && (start..end).contains(&label)),
            );
        }
        s.run_frame();
    }
    capture(&s, out, "boot");
    let script = std::fs::read_to_string(&a[3]).unwrap();
    let commands: Vec<Value> = script
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert!(
        commands.last().is_some_and(|c| c["finish"] == true),
        "itinerary must finish"
    );
    for (index, command) in commands.iter().enumerate() {
        if command["finish"] == true {
            assert_eq!(index + 1, commands.len());
            for (button, _) in BUTTONS {
                s.set_button(button, false);
            }
            for sample in 0..=400 {
                let w = s.wram_image();
                let u = |p| u16::from_le_bytes([w[p], w[p + 1]]);
                let e = 0x1040;
                println!(
                    "{}",
                    json!({"sample":sample,"frame":s.frame_state().frames,"map":u(0x47e),
                    "e":(0..0x40).step_by(2).map(|i|u(e+i)).collect::<Vec<_>>(),
                    "aux":(0..0x40).step_by(2).map(|i|u(0x10000+e+i)).collect::<Vec<_>>(),
                    "class":u(0x12018+e)})
                );
                if sample < 400 {
                    s.run_frame();
                }
            }
            std::process::exit(0);
        }
        let label = command["label"].as_str().unwrap();
        assert!(label
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-'));
        let frames = command["frames"].as_u64().unwrap();
        assert!(frames > 0 && frames <= 2000);
        let buttons = command["buttons"].as_array().unwrap();
        assert!(buttons
            .iter()
            .all(|v| BUTTONS.iter().any(|(_, name)| v == name)));
        for (b, name) in BUTTONS {
            s.set_button(b, buttons.iter().any(|v| v == name));
        }
        for _ in 0..frames {
            s.run_frame();
        }
        capture(&s, out, label);
    }
}
