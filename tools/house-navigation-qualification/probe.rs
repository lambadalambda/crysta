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
fn sha(bytes: &[u8]) -> String {
    rom::digests(bytes)
        .sha256
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn source_door(rom: &rom::Rom) {
    // These common-sheet resources are already qualified under F. This does not
    // admit C's complete loading recipe or its occupancy/background profile.
    let bg = assets::maps::visual::StaticBackground::from_rom(rom.image(), 15).unwrap();
    assert_eq!(bg.layer().cells()[19 * 32 + 8].raw(), 0xf2);
    assert_eq!(bg.layer().cells()[20 * 32 + 8].raw(), 0xf3);
    let resources: Vec<_> = [2, 3]
        .into_iter()
        .map(|i| {
            let r = &bg.resources()[i];
            let range = r.source_range();
            json!({"extent":[range.start,range.end],"source_sha256":sha(r.source_bytes()),
            "decoded_sha256":sha(r.decoded())})
        })
        .collect();
    println!("{}", json!({"kind":"source-door","resources":resources}));
}
fn navigation(s: &Session, label: &str) {
    let w = s.wram_image();
    let u = |p| u16::from_le_bytes([w[p], w[p + 1]]);
    println!(
        "{}",
        json!({"kind":"frame","label":label,"frame":s.frame_state().frames,
        "map":u(0x47e),"position":[u(0x1000),u(0x1002)],
        "script":u(0x100a) as u32 + ((w[0x100c] as u32)<<16),
        "flags":u(0x1004),"facing":u(0x1014),"control":u(0x980),
        "door":[u(0xa4d0),u(0xa510)]})
    );
}
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
    source_door(&rom);
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
    navigation(&s, "boot");
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
            navigation(&s, label);
        }
        capture(&s, out, label);
    }
}
