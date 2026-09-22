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
        "player_extra": w.get(0x10000 + usize::from(word(w, 0xdea))..)
            .and_then(|tail| tail.get(..64))
            .map(|bytes| (0..32).map(|i| word(bytes, 2 * i)).collect::<Vec<_>>()),
        "actors": actors,
    })
}

/// Dense arrival observations, including transient loader state. A decoded
/// layer here is evidence, not permission to walk during loading or forced motion.
fn arrival_sample(w: &[u8], label: &str, held: &[&str], frames: u32) -> Value {
    let mut value = sample(w, label, held, frames);
    value["kind"] = json!("arrival");
    for (name, at) in [
        ("pending_map", 0x47C),
        ("previous_map", 0x482),
        ("exit_list_map", 0x480),
        ("selector", 0x490),
        ("special", 0x97C),
        ("request_mode", 0x484),
        ("control_slot", 0xDEE),
        ("input_mask", 0x45E),
    ] {
        value[name] = json!(word(w, at));
    }
    value["queued_position"] = json!([word(w, 0x492), word(w, 0x494)]);
    value["exit_origin"] = json!([word(w, 0x95E), word(w, 0x960)]);
    value["exit_cell"] = json!([word(w, 0x962), word(w, 0x964)]);
    value["control_words"] = json!((0..32)
        .map(|i| word(w, usize::from(word(w, 0xDEE)) + 2 * i))
        .collect::<Vec<_>>());
    value["player_words"] = json!((0..32).map(|i| word(w, 0x1000 + 2 * i)).collect::<Vec<_>>());
    value["layer"] = assets::maps::LoadedMap::from_wram(w).map_or(Value::Null, |layer| {
        json!({
            "width": layer.width(), "height": layer.height(),
            "cells": layer.cells().iter().map(|cell| cell.raw()).collect::<Vec<_>>()
        })
    });
    value
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
        emit(&sample(
            &s.wram_image(),
            label,
            &[want],
            s.frame_state().frames,
        ));
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

/// Capture the player's live inputs to $80:D107 before its STA $02 executes.
/// No state is patched: both bounded traces advance the same input-only session.
fn motion_frame(s: &mut Session, label: &str, held: &[&str]) -> Value {
    use oracle::CpuTraceStop::{FrameLimit, TargetReached};
    let frame = s.frame_state().frames;
    let start = s.wram_image();
    // Resident scheduling can put another actor ahead of Ark. Select the
    // player call by slot, never by call order; retain one shared trace budget
    // and refuse any frame without an ordinary player resolver invocation.
    let mut remaining = 100_000;
    loop {
        let entry = s.trace_until_pc(0x80_D107, remaining, 1).unwrap();
        if entry.stop != TargetReached {
            let end = s.wram_image();
            panic!(
                "player motion entry not reached: {:?}, frame {} -> {}, XY {:?} -> {:?}, flags {:04x} -> {:04x}, special {:04x} -> {:04x}, control {:04x} -> {:04x}",
                entry.stop, frame, s.frame_state().frames,
                position(&start), position(&end), word(&start, 0x1004), word(&end, 0x1004),
                word(&start, 0x97C), word(&end, 0x97C), word(&start, 0x980), word(&end, 0x980)
            );
        }
        assert_eq!(
            s.frame_state().frames,
            frame,
            "motion entry crossed a frame"
        );
        remaining -= entry.entries.len();
        if s.cpu_registers().x == 0x1000 {
            break;
        }
        assert!(remaining > 0, "player motion search exhausted");
    }
    let before = s.wram_image();
    assert_eq!(word(&before, 0xDEA), 0x1000);
    assert_eq!(
        position(&start),
        position(&before),
        "controller moved Ark before collision"
    );
    let layer = assets::maps::LoadedMap::from_wram(&before).unwrap();
    let rest = s.trace_until_pc(0xFF_FFFF, 100_000, 1).unwrap();
    assert_eq!(rest.stop, FrameLimit, "incomplete motion frame");
    assert_eq!(s.frame_state().frames, frame + 1);
    let after = s.wram_image();
    assert_eq!(
        word(&before, 0x47E),
        word(&after, 0x47E),
        "motion changed maps"
    );
    json!({
        "kind": "motion", "label": label, "frame": frame + 1, "held": held,
        "map": word(&before, 0x47E),
        "before": position(&before), "after": position(&after),
        "attempt": [word(&before, 0x11018) as i16, word(&before, 0x1101A) as i16],
        "control": [word(&before, 0x980), word(&after, 0x980)],
        "flags": word(&before, 0x1004), "special": word(&before, 0x97C),
        "bounds": [word(&before, 0x11028), word(&before, 0x1102A),
                   word(&before, 0x1102C), word(&before, 0x1102E)],
        "width": layer.width(), "height": layer.height(),
        "cells": layer.cells().iter().map(|cell| cell.raw()).collect::<Vec<_>>(),
        "path": rest.entries.iter().filter_map(|entry|
            (0x80_D100..0x80_E87C).contains(&entry.address).then_some(entry.address)
        ).collect::<Vec<_>>(),
    })
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
    emit(&sample(
        &s.wram_image(),
        "boot",
        &[],
        s.frame_state().frames,
    ));

    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let c: Value = serde_json::from_str(&line).unwrap();
        if c["finish"] == true {
            std::io::stdout().flush().unwrap();
            std::process::exit(0);
        }
        if let Some(target) = c["goto"].as_array() {
            let label = c["label"].as_str().unwrap();
            let target = (
                u16::try_from(target[0].as_u64().unwrap()).unwrap(),
                u16::try_from(target[1].as_u64().unwrap()).unwrap(),
            );
            let result = goto(&mut s, target, label);
            emit(
                &json!({"kind": "goto", "label": label, "target": target, "result": result,
                         "position": position(&s.wram_image())}),
            );
            continue;
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
            assert_eq!(
                trace.stop,
                oracle::CpuTraceStop::FrameLimit,
                "incomplete frame trace"
            );
            assert_eq!(after, before + 1, "trace must advance exactly one frame");
            let mut bytes = Vec::with_capacity(trace.entries.len() * 8);
            for entry in &trace.entries {
                bytes.extend_from_slice(&entry.address.to_le_bytes());
                bytes.extend_from_slice(&[
                    entry.status,
                    u8::from(entry.emulation),
                    entry.data_bank,
                    0,
                ]);
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
        let label = c["label"].as_str().unwrap();
        let frames = c["frames"].as_u64().unwrap();
        assert!(frames > 0 && frames <= 2000);
        for _ in 0..frames {
            if c["motion"] == true {
                emit(&motion_frame(&mut s, label, &held));
                continue;
            }
            s.run_frame();
            let state = s.wram_image();
            emit(&if c["arrival"] == true {
                arrival_sample(&state, label, &held, s.frame_state().frames)
            } else {
                sample(&state, label, &held, s.frame_state().frames)
            });
        }
    }
    panic!("itinerary missing finish");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transient_player_mirror_is_bounded_and_distinct_from_fixed_player_words() {
        let mut w = vec![0; 0x20000];
        w[0x1000] = 42;
        w[0x1FFC0] = 99;
        for slot in [0xFFC0u16, 0xFFC1, 0xFFFF] {
            w[0xDEA..0xDEC].copy_from_slice(&slot.to_le_bytes());
            let row = arrival_sample(&w, "transient", &[], 1);
            assert_eq!(row["player_slot"], slot);
            assert_eq!(row["player_words"][0], 42);
            if slot == 0xFFC0 {
                assert_eq!(row["player_extra"][0], 99);
                assert_eq!(row["player_extra"].as_array().unwrap().len(), 32);
            } else {
                assert!(row["player_extra"].is_null());
            }
        }
    }

    #[test]
    fn arrival_fields_keep_loader_queue_and_controller_state_distinct() {
        let mut w = vec![0; 0x20000];
        for (at, value) in [
            (0x47C, 0x17u16),
            (0x47E, 0x19),
            (0x480, 0x18),
            (0x482, 0x16),
            (0x490, 14),
            (0x492, 448),
            (0x494, 352),
            (0x97C, 0x10),
            (0x980, 0xA0),
            (0x1000, 442),
            (0x1002, 345),
            (0x1004, 0x0415),
            (0xDEA, 0x1000),
            (0x484, 0x80),
            (0xDEE, 0x1040),
            (0x45E, 0xFFFF),
            (0x95E, 440),
            (0x960, 329),
            (0x962, 27),
            (0x964, 20),
            (0x1040, 123),
            (0x107E, 456),
        ] {
            w[at..at + 2].copy_from_slice(&value.to_le_bytes());
        }
        let row = arrival_sample(&w, "return", &[], 123);
        for (name, expected) in [
            ("pending_map", 0x17),
            ("map", 0x19),
            ("exit_list_map", 0x18),
            ("previous_map", 0x16),
            ("selector", 14),
            ("special", 0x10),
            ("control", 0xA0),
        ] {
            assert_eq!(row[name], expected);
        }
        assert_eq!(row["kind"], "arrival");
        assert_eq!(row["queued_position"], json!([448, 352]));
        assert_eq!(row["position"], json!([442, 345]));
        assert_eq!(row["player_words"][2], 0x0415);
        assert_eq!(row["request_mode"], 0x80);
        assert_eq!(row["control_slot"], 0x1040);
        assert_eq!(row["input_mask"], 0xFFFF);
        assert_eq!(row["exit_origin"], json!([440, 329]));
        assert_eq!(row["exit_cell"], json!([27, 20]));
        assert_eq!(row["control_words"][0], 123);
        assert_eq!(row["control_words"][31], 456);
        assert!(
            row["layer"].is_null(),
            "invalid transient dimensions are not a layer"
        );
    }
}
