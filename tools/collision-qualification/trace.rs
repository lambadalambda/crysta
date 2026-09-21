//! Frame-trace probe: the collision probe's route discipline, plus a full
//! instruction trace of any single frame and the actor slots each frame.
//!
//! Same session discipline as every accepted route: one empty-SRAM Session,
//! real button input from boot, no warp, no memory patch, no save and no state
//! restore. It exists for two questions the movement probe cannot answer:
//! which code runs when a step is refused that does not run when it is
//! admitted, and which word in an actor's slot changes when the player talks
//! to them.
//!
//! Route lines are the movement probe's, plus
//! `{"trace": "name", "buttons": [...]}`, which holds the buttons for exactly
//! one frame, records every instruction address executed during it, and
//! writes them in order to `OUT/name.trace`, eight bytes per instruction:
//! the little-endian u32 address, the status byte, the emulation flag, the
//! data bank, and a zero.
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

fn word(w: &[u8], p: usize) -> u16 {
    u16::from_le_bytes([w[p], w[p + 1]])
}

/// Per-frame sample: the player's words and every live actor slot's first
/// sixteen words, raw. Slot words are `$7E:1040 + $40 * n`; a slot is live
/// when its script pointer at +10 is nonzero, as the movement probe reads it.
fn sample(w: &[u8], label: &str, held: &[&str], frames: u32) -> Value {
    let actors: Vec<Value> = (0x1040..0x2000)
        .step_by(0x40)
        .filter(|&p| word(w, p + 10) != 0)
        .map(|p| {
            let words: Vec<u16> = (0..16).map(|i| word(w, p + 2 * i)).collect();
            json!({"slot": p, "words": words})
        })
        .collect();
    json!({
        "kind": "frame", "label": label, "frame": frames, "held": held,
        "map": word(w, 0x47e),
        "position": [word(w, 0x1000), word(w, 0x1002)],
        "facing": word(w, 0x1014),
        "control": word(w, 0x980),
        // Words the actor services compare against: `$0454`, `$0956`,
        // `$0966`, `$0968` and `$0999`, in that order.
        "service_words": [word(w, 0x454), word(w, 0x956), word(w, 0x966), word(w, 0x968), word(w, 0x999)],
        "player_slot": word(w, 0xdea),
        // The player slot's `$7F` mirror, `$7F:0000 + slot`, thirty-two words.
        "player_extra": (0..32)
            .map(|i| word(w, 0x10000 + usize::from(word(w, 0xdea)) + 2 * i))
            .collect::<Vec<u16>>(),
        "actors": actors,
    })
}

fn emit(value: &Value) {
    println!("{value}");
    std::io::stdout().flush().unwrap();
}

fn hold(s: &mut Session, buttons: &[Value]) {
    for (b, name) in BUTTONS {
        s.set_button(b, buttons.iter().any(|v| v == name));
    }
}

fn position(w: &[u8]) -> (u16, u16) {
    (word(w, 0x1000), word(w, 0x1002))
}

/// Walks the player to a pixel position with the pad, one axis at a time,
/// x first. Every frame is sampled as a `frame` row under `label`.
///
/// A held direction is released for twelve neutral frames before another is
/// pressed, because a fresh press of the same direction inside the game's
/// eleven-tick onset window selects a dash (`docs/input-admission.md`). The
/// walk ends when the target is reached, when thirty frames pass without
/// movement, or after six hundred frames.
fn goto(s: &mut Session, target: (u16, u16), label: &str) -> &'static str {
    let mut held: Option<&str> = None;
    let mut still = 0u32;
    for _ in 0..600 {
        let (x, y) = position(&s.wram_image());
        let want = if x != target.0 {
            Some(if target.0 > x { "Right" } else { "Left" })
        } else if y != target.1 {
            Some(if target.1 > y { "Down" } else { "Up" })
        } else {
            None
        };
        let Some(want) = want else {
            hold(s, &[]);
            return "reached";
        };
        if held != Some(want) {
            hold(s, &[]);
            for _ in 0..12 {
                s.run_frame();
                emit(&sample(&s.wram_image(), label, &[], s.frame_state().frames));
            }
            held = Some(want);
            still = 0;
        }
        hold(s, &[json!(want)]);
        s.run_frame();
        emit(&sample(&s.wram_image(), label, &[want], s.frame_state().frames));
        if position(&s.wram_image()) == (x, y) {
            still += 1;
            if still >= 30 {
                hold(s, &[]);
                return "stalled";
            }
        } else {
            still = 0;
        }
    }
    hold(s, &[]);
    "exhausted"
}

fn main() {
    let a: Vec<_> = std::env::args().collect();
    assert_eq!(a.len(), 3, "trace ROM local/OUT < ROUTE.jsonl");
    let out = std::path::Path::new(&a[2]);
    assert!(
        out.starts_with("local")
            && !out
                .components()
                .any(|c| c == std::path::Component::ParentDir),
        "output must stay under local/"
    );
    std::fs::create_dir_all(out).unwrap();
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
    emit(&sample(&s.wram_image(), "boot", &[], s.frame_state().frames));

    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let c: Value = serde_json::from_str(&line).unwrap();
        if c["finish"] == true {
            std::io::stdout().flush().unwrap();
            std::process::exit(0);
        }
        let buttons = c["buttons"].as_array().unwrap();
        assert!(buttons
            .iter()
            .all(|v| BUTTONS.iter().any(|(_, name)| v == name)));
        let held: Vec<&str> = buttons.iter().filter_map(Value::as_str).collect();
        hold(&mut s, buttons);
        if let Some(name) = c["trace"].as_str() {
            assert!(name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'));
            let before = s.frame_state().frames;
            // An address never executed, so the trace runs out the frame.
            let trace = s.trace_until_pc(0xFF_FFFF, 2_000_000, 1).unwrap();
            let after = s.frame_state().frames;
            let mut bytes = Vec::with_capacity(trace.entries.len() * 8);
            for entry in &trace.entries {
                bytes.extend_from_slice(&entry.address.to_le_bytes());
                bytes.extend_from_slice(&[entry.status, u8::from(entry.emulation), entry.data_bank, 0]);
            }
            std::fs::write(out.join(format!("{name}.trace")), bytes).unwrap();
            emit(&json!({
                "kind": "trace", "trace": name, "held": held,
                "instructions": trace.entries.len(), "stop": format!("{:?}", trace.stop),
                "frames": [before, after],
            }));
            emit(&sample(&s.wram_image(), name, &held, after));
            continue;
        }
        if let Some(target) = c["goto"].as_array() {
            let label = c["label"].as_str().unwrap();
            let target = (
                u16::try_from(target[0].as_u64().unwrap()).unwrap(),
                u16::try_from(target[1].as_u64().unwrap()).unwrap(),
            );
            let result = goto(&mut s, target, label);
            emit(&json!({"kind": "goto", "label": label, "target": target, "result": result,
                         "position": position(&s.wram_image())}));
            continue;
        }
        let label = c["label"].as_str().unwrap();
        let frames = c["frames"].as_u64().unwrap();
        assert!(frames > 0 && frames <= 2000);
        for _ in 0..frames {
            s.run_frame();
            emit(&sample(
                &s.wram_image(),
                label,
                &held,
                s.frame_state().frames,
            ));
        }
    }
    panic!("itinerary missing finish");
}
