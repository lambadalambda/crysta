//! Collision-discovery probe: dense movement samples against the runtime layer.
//!
//! Same session discipline as every accepted route: one empty-SRAM Session,
//! real button input from boot, no warp, no memory patch, no save and no state
//! restore. The player is walked to each sample; positions are never assigned.
//!
//! It answers the question `docs/maps.md` leaves open — which cell words admit
//! the player and which refuse — by recording, for every frame, the held
//! direction and the resulting pixel position, beside the map's own runtime
//! layer. Deriving a predicate from those samples is `derive.py`'s job; this
//! probe claims nothing about semantics.
use assets::maps::LoadedMap;
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

/// Per-frame movement sample. Position is the loader's own player word pair,
/// not a sprite or camera derivation.
///
/// Live actor positions are recorded because terrain is not the only thing
/// that stops a player: a resident standing in a doorway produces exactly the
/// same sustained stall as a wall, and reading that as solid terrain would
/// invent a collision the map does not have.
fn sample(w: &[u8], label: &str, held: &[&str], frames: u32) -> Value {
    let actors: Vec<Value> = (0x1040..0x2000)
        .step_by(0x40)
        .filter(|&p| word(w, p + 10) != 0)
        .map(|p| json!([word(w, p), word(w, p + 2)]))
        .collect();
    json!({
        "kind": "frame", "label": label, "frame": frames, "held": held,
        "map": word(w, 0x47e),
        "position": [word(w, 0x1000), word(w, 0x1002)],
        "facing": word(w, 0x1014),
        "control": word(w, 0x980),
        "flags": word(w, 0x1004),
        "actors": actors,
    })
}

/// Writes the decoded runtime layer for the current map, once per map id.
///
/// Raw words are preserved; no collision meaning is imposed here.
///
/// Only dumps once the map has settled, meaning input is admitted. A layer
/// read mid-transition has not had its attribute pass applied yet: map `$000A`
/// captured that way reads as 5,120 cells of attribute zero, which would make
/// every wall look walkable.
fn dump_layer(s: &Session, out: &std::path::Path, seen: &mut Vec<u16>) {
    let w = s.wram_image();
    let id = word(&w, 0x47e);
    if seen.contains(&id) || word(&w, 0x980) != 160 {
        return;
    }
    let Ok(map) = LoadedMap::from_wram(&w) else {
        println!("{}", json!({"kind": "layer-unavailable", "map": id}));
        return;
    };
    seen.push(id);
    let cells: Vec<u16> = map.cells().iter().map(|c| c.raw()).collect();
    let path = out.join(format!("layer-{id:04x}.json"));
    std::fs::write(
        &path,
        serde_json::to_vec(&json!({
            "map": id, "width": map.width(), "height": map.height(), "cells": cells,
        }))
        .unwrap(),
    )
    .unwrap();
    println!(
        "{}",
        json!({"kind": "layer", "map": id, "width": map.width(), "height": map.height(),
               "cells": cells.len()})
    );
}

fn emit(value: &Value) {
    println!("{value}");
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
    let mut seen = Vec::new();
    dump_layer(&s, out, &mut seen);
    emit(&sample(&s.wram_image(), "boot", &[], s.frame_state().frames));

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
        // Diagnostic-only: retain a full WRAM image so offline analysis can look
        // for planes beside the documented first runtime layer.
        if c["wram"] == true {
            let label = c["label"].as_str().unwrap();
            std::fs::write(out.join(format!("{label}.wram")), s.wram_image()).unwrap();
            emit(&json!({"kind": "wram", "label": label}));
            continue;
        }
        let label = c["label"].as_str().unwrap();
        assert!(
            !label.is_empty()
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        );
        let frames = c["frames"].as_u64().unwrap();
        assert!(frames > 0 && frames <= 2000);
        let buttons = c["buttons"].as_array().unwrap();
        assert!(buttons
            .iter()
            .all(|v| BUTTONS.iter().any(|(_, name)| v == name)));
        let held: Vec<&str> = buttons.iter().filter_map(Value::as_str).collect();
        for (b, name) in BUTTONS {
            s.set_button(b, buttons.iter().any(|v| v == name));
        }
        for _ in 0..frames {
            s.run_frame();
            emit(&sample(
                &s.wram_image(),
                label,
                &held,
                s.frame_state().frames,
            ));
            dump_layer(&s, out, &mut seen);
        }
    }
    panic!("itinerary missing finish");
}
