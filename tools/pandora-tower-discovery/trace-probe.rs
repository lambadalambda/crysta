//! Discovery trace probe: replay the accepted prefix, then breakpoint scripts.
//!
//! Same session discipline as the accepted route: one empty-SRAM Session, real
//! button input from the menu onward, no warp, no memory patch, no save and no
//! state restore. It answers questions the walking probe cannot: whether a
//! script address is ever reached, and what CPU state holds when it is.
//!
//! It keeps `save_state` at the same point in every command as the accepted
//! probe, because that call synchronizes the core (`serialize(true)`) and is
//! therefore part of the journey's timing; see `docs/sprite-hardware.md`. The
//! other accessors are passive copies, and this probe does not reproduce the
//! accepted probe's per-frame `wram_image` reads. That makes timing equivalence
//! an empirical question, not a structural one, so the endpoint observation must
//! be compared against the accepted capture before any trace result is trusted.
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

/// The accepted probe's observation, minus artifact writes.
fn synchronize(s: &Session) -> Vec<u8> {
    let _ = oracle::save_state(s);
    let w = s.wram_image();
    let _ = s.vram();
    let _ = s.cgram();
    let _ = s.pixels();
    let _ = s.sprite_state();
    w
}

fn observation(w: &[u8], label: &str, kind: &str, frames: u32) -> Value {
    json!({
        "kind": kind, "label": label, "frame": frames,
        "map": word(w, 0x47e), "position": [word(w, 0x1000), word(w, 0x1002)],
        "facing": word(w, 0x1014), "control": word(w, 0x980),
        "script": word(w, 0x100a) as u32 + ((w[0x100c] as u32) << 16),
        "flags": (0..0x400).filter(|i| w[0x6c0 + i / 8] >> (i % 8) & 1 != 0).collect::<Vec<_>>(),
    })
}

fn emit(value: &Value) {
    println!("{value}");
    std::io::stdout().flush().unwrap();
}

fn main() {
    let a: Vec<_> = std::env::args().collect();
    assert_eq!(a.len(), 3, "trace-probe ROM local/OUT < ROUTE.jsonl");
    let out = std::path::Path::new(&a[2]);
    assert!(
        out.starts_with("local") && !out.components().any(|c| c == std::path::Component::ParentDir)
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
    let w = synchronize(&s);
    emit(&observation(&w, "boot", "checkpoint", s.frame_state().frames));
    // Retain the itinerary, like the accepted probe, so a session's evidence
    // can be reproduced instead of merely described.
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
        if let Some(target) = c["trace"].as_str() {
            // Breakpoint: run until PC reaches `target`, bounded by frames.
            let target = u32::from_str_radix(target, 16).expect("hex trace target");
            let frames = c["frames"].as_u64().unwrap_or(600) as u32;
            let limit = c["instructions"].as_u64().unwrap_or(2_000_000) as usize;
            let before = s.frame_state().frames;
            let trace = s.trace_until_pc(target, limit, frames).unwrap();
            // Both reads must happen before `synchronize`: the trace parks the
            // CPU *before* the target instruction, and `save_state` resumes the
            // cothread to its safe point, completing that instruction. Reading
            // memory afterwards would pair pre-instruction registers with
            // post-instruction memory.
            let reg = s.cpu_registers();
            let stack = s.wram_image();
            let w = synchronize(&s);
            // The 65816 stack is bank $00:$0000..$1FFF, mirrored at the base of
            // the WRAM image. The caller's return address is what identifies
            // which script site invoked a shared predicate.
            let base = reg.stack as usize;
            assert!(base + 8 < 0x2000, "stack pointer outside the stack page");
            let stack_bytes: Vec<u8> = (1..=8).map(|i| stack[base + i]).collect();
            // Deduplicated tail of the execution path into the target.
            let mut path: Vec<String> = Vec::new();
            for entry in trace.entries.iter().rev().take(4000) {
                let text = format!("{:06X}", entry.address);
                if path.last() != Some(&text) {
                    path.push(text);
                }
                if path.len() >= 24 {
                    break;
                }
            }
            emit(&json!({
                "kind": "trace", "target": format!("{target:06X}"),
                "stop": format!("{:?}", trace.stop),
                "instructions": trace.entries.len(),
                "digest": trace.digest_hex(),
                "elapsed_frames": s.frame_state().frames - before,
                "registers": {
                    "address": format!("{:06X}", reg.address), "a": reg.accumulator,
                    "x": reg.x, "y": reg.y, "stack": format!("{:04X}", reg.stack),
                    "direct_page": format!("{:04X}", reg.direct_page), "status": reg.status,
                },
                "stack_bytes": stack_bytes,
                "path": path,
                "state": observation(&w, "trace", "trace-state", s.frame_state().frames),
            }));
            continue;
        }
        if let Some(range) = c["profile"].as_object() {
            // Which code in a range actually executes over a few frames. The
            // target is deliberately unreachable, so the trace runs to its
            // frame limit and the entries are the execution profile.
            let from = u32::from_str_radix(range["from"].as_str().unwrap(), 16).unwrap();
            let to = u32::from_str_radix(range["to"].as_str().unwrap(), 16).unwrap();
            let frames = c["frames"].as_u64().unwrap_or(2) as u32;
            let limit = c["instructions"].as_u64().unwrap_or(2_000_000) as usize;
            let trace = s.trace_until_pc(0xFF_FFFF, limit, frames).unwrap();
            let mut counts: std::collections::BTreeMap<u32, u32> = Default::default();
            for entry in &trace.entries {
                if (from..to).contains(&entry.address) {
                    *counts.entry(entry.address).or_default() += 1;
                }
            }
            let w = synchronize(&s);
            emit(&json!({
                "kind": "profile",
                "range": [format!("{from:06X}"), format!("{to:06X}")],
                "stop": format!("{:?}", trace.stop),
                "instructions": trace.entries.len(),
                "hits": counts.iter().map(|(a, n)| json!([format!("{a:06X}"), n]))
                    .collect::<Vec<_>>(),
                "state": observation(&w, "profile", "profile-state", s.frame_state().frames),
            }));
            continue;
        }
        let label = c["label"].as_str().unwrap();
        assert!(!label.is_empty() && label.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'));
        assert!(
            !out.join(format!("{label}.wram")).exists(),
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
        }
        let w = synchronize(&s);
        std::fs::write(out.join(format!("{label}.wram")), &w).unwrap();
        emit(&observation(&w, label, "checkpoint", s.frame_state().frames));
    }
    panic!("itinerary missing finish");
}
