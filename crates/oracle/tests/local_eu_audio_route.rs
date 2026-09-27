//! Empty-SRAM EU tour: CPU-to-APU effect parity and bounded track-port patterns,
//! not PCM/DSP fidelity.
//! ares is a process singleton, so all checkpoints share one fresh child.
use oracle::{ApuPortWrite, Button, Session, MAX_APU_PORT_WRITES};
use std::{path::Path, process::Command};

const TEST: &str = "european_route_apu_commands";
const TOUR: &str = include_str!("fixtures/eu-pandora-tour.inputs");
const WORLD: &str = include_str!("fixtures/eu-world-map.inputs");
const BUTTONS: [(Button, &str); 6] = [
    (Button::Start, "Start"),
    (Button::A, "A"),
    (Button::Up, "Up"),
    (Button::Down, "Down"),
    (Button::Left, "Left"),
    (Button::Right, "Right"),
];

#[test]
fn european_route_apu_commands() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/Terranigma (E) [!].smc");
    if !path.exists() {
        eprintln!("skipping: owned European ROM absent");
        return;
    }
    if std::env::var_os("EU_APU_CHILD").is_some() {
        run_child(&path);
    }
    let out = Command::new(std::env::current_exe().expect("test executable"))
        .args(["--exact", TEST, "--nocapture"])
        .env("EU_APU_CHILD", "1")
        .output()
        .expect("fresh oracle child");
    assert!(
        out.status.success(),
        "native audio witness failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stderr)
        .contains("EU route APU bounded port-pattern witness checked"));
}

// A write pair must actually be consecutive within a frame, not merely two
// bytes occurring somewhere in a busy interval (track uploads use both ports).
fn pair_index(writes: &[ApuPortWrite], low: u8, high: u8) -> Option<usize> {
    writes.windows(2).position(|w| {
        w[0].port == 2
            && w[0].value == low
            && w[1].port == 3
            && w[1].value == high
            && w[0].frame == w[1].frame
    })
}

// Keep the bus-order position in its fixture leg as well as the frame. The
// second door throw plays $12, then $001A, then $13; it is not just a set.
fn ordered_effect(
    writes: &[ApuPortWrite],
    row: usize,
    word: u16,
    previous: &mut Option<(usize, usize, u32)>,
) {
    let [low, high] = word.to_le_bytes();
    let index = pair_index(writes, low, high)
        .unwrap_or_else(|| panic!("missing exact port2/port3 effect {word:#06x} in leg {row}"));
    let frame = writes[index + 1].frame;
    if let Some((prior_row, prior_index, prior_frame)) = *previous {
        assert!(
            (prior_row == row && prior_index < index) || (prior_row < row && prior_frame < frame),
            "effect {word:#06x} out of order in leg {row}"
        );
    }
    *previous = Some((row, index, frame));
}

// Locate the host control write before the upload starts. Port 0 also
// carries arbitrary sample bytes *during* the upload, including F0/FF/F4.
// Pairing must be adjacent within the SAME emulated frame.
fn control(writes: &[ApuPortWrite], command: u8, parameter: u8) -> Option<usize> {
    writes
        .windows(2)
        .position(|w| {
            w[0].port == 1
                && w[0].value == parameter
                && w[1].port == 0
                && w[1].value == command
                && w[0].frame == w[1].frame
        })
        .map(|index| index + 1)
}

// Bounded port-pattern witness for a requested track change, not an SPC
// completion/handshake proof or an identification of uploaded track bytes.
fn track_port_pattern(writes: &[ApuPortWrite], parameter: u8) {
    let stop =
        control(writes, 0xf0, parameter).expect("same-frame F0 paired with source stop parameter");
    assert!(
        stop >= 2 && writes[stop - 2].port == 0 && writes[stop - 2].value == 0,
        "host clears port 0 before F0 parameter/command"
    );
    let upload = writes[stop + 1..]
        .iter()
        .position(|w| w.port == 0 && w.value == 0xff)
        .map(|offset| stop + 1 + offset)
        .expect("FF-shaped write begins $1048 upload pattern after F0");
    assert_eq!(
        pair_index(&writes[upload + 1..upload + 3], 0x48, 0x10),
        Some(0),
        "FF is followed immediately by the receiver's $1048 destination"
    );
    // The final F4-shaped port-0 write is isolated in a later frame than
    // all preceding port-0/1 writes in this leg. Without an SPC acknowledgment
    // this witnesses the host-side pattern, not completed upload or playback.
    let play = writes
        .iter()
        .rposition(|w| w.port == 0)
        .expect("track script writes port 0");
    let last_upload_write = writes[..play]
        .iter()
        .rfind(|w| w.port <= 1)
        .expect("track uploads through ports 0/1");
    assert!(
        play > upload
            && writes[play].value == 0xf4
            && last_upload_write.frame < writes[play].frame
            && writes
                .iter()
                .filter(|w| w.frame == writes[play].frame && w.port <= 1)
                .count()
                == 1
            && writes[play + 1..].iter().all(|w| w.port > 1),
        "isolated final F4-shaped write follows FF/upload pattern; no later port-0/1 writes"
    );
}

fn advance(session: &mut Session, frames: usize, held: &[&str], keep: bool) -> Vec<ApuPortWrite> {
    for (button, name) in BUTTONS {
        session.set_button(button, held.contains(&name));
    }
    assert!(held
        .iter()
        .all(|name| BUTTONS.iter().any(|(_, known)| name == known)));
    let mut writes = Vec::new();
    for _ in 0..frames {
        session.run_frame();
        let capture = session.take_apu_port_writes();
        assert!(
            !capture.overflow && capture.entries.len() <= MAX_APU_PORT_WRITES,
            "APU port queue overflow"
        );
        if keep {
            writes.extend(capture.entries);
        }
    }
    writes
}

fn run_child(path: &Path) -> ! {
    let image = std::fs::read(path).expect("owned ROM");
    let rom = rom::Rom::load(&image).expect("valid ROM");
    assert_eq!(rom.revision(), rom::Revision::EuropeEnglish);
    // European $99:F9EA table, normalized ROM offset: byte 3's low nibble
    // feeds F0/F1. In particular the spear's three tracks share parameter 5.
    for (track_index, parameter) in [(1, 1), (4, 7), (0x34, 5), (0x1c, 5), (6, 5)] {
        assert_eq!(rom.image()[0x19_f9ea + track_index * 4 + 3] & 15, parameter);
    }
    let mut session = Session::new(&rom).expect("empty-SRAM oracle");
    advance(&mut session, 1800, &[], false);
    advance(&mut session, 10, &["Start"], false);
    advance(&mut session, 150, &[], false);
    let mut fade = Vec::new();
    let mut previous_effect = None;
    // Rows below are identified by fixture *actions* and their route state,
    // not absolute emulator frames. Never seed SRAM, flags or WRAM.
    for (index, line) in TOUR.lines().chain(WORLD.lines().take(58)).enumerate() {
        let row = index + 1;
        let mut parts = line.split_whitespace();
        let frames = parts
            .next()
            .expect("fixture frames")
            .parse()
            .expect("frame count");
        let held: Vec<_> = parts.collect();
        let keep = matches!(
            row,
            187 | 188 | 195 | 196 | 212 | 226 | 238 | 248 | 249 | 251 | 252 | 257 | 501 | 504 | 511
        );
        let writes = advance(&mut session, frames, &held, keep);
        match row {
            // Each lift is a one-frame A edge followed by a neutral settling
            // leg; the missed throw, first door hit and second hit follow Up/A.
            187 | 195 => assert_eq!(line, "1 A"),
            188 | 212 | 238 => {
                assert_eq!(TOUR.lines().nth(row - 2), Some("1 A"), "pot lift edge");
                assert_eq!(session.wram(0x47e), 0x0c, "pot in C");
                assert_eq!(line, "120");
                ordered_effect(&writes, row, 0x1100, &mut previous_effect);
            }
            196 | 226 => {
                assert_eq!(TOUR.lines().nth(row - 2), Some("1 A"), "pot throw edge");
                assert_eq!(session.wram(0x47e), 0x0c, "throw in C");
                assert_eq!(line, "180");
                ordered_effect(&writes, row, 0x1200, &mut previous_effect);
                ordered_effect(&writes, row, 0x1300, &mut previous_effect);
            }
            248 => {
                assert_eq!(TOUR.lines().nth(row - 2), Some("1 A"), "second door throw");
                assert_eq!(session.wram(0x47e), 0x0c, "door in C");
                assert_eq!(line, "240");
                ordered_effect(&writes, row, 0x1200, &mut previous_effect);
                // Native $1200 -> $001A -> $1300; portable $001A precedes
                // $1200 because the hit callback currently runs four frames
                // early. These effect orders are explicitly NOT parity.
                ordered_effect(&writes, row, 0x001a, &mut previous_effect);
                ordered_effect(&writes, row, 0x1300, &mut previous_effect);
                let flags = session.wram(0x6c0 + 0x292 / 8);
                assert_ne!(flags & (1 << (0x292 % 8)), 0, "blue door opened");
            }
            249 => {
                assert_eq!(line, "240");
                assert!(
                    pair_index(&writes, 0x1a, 0).is_none(),
                    "no duplicate door-open effect"
                );
                assert!(
                    !writes.iter().any(|w| w.port == 0 && w.value == 0xf0),
                    "quiet continuation in C does not restart the track"
                );
            }
            251 => {
                assert_eq!(line, "240");
                fade = writes;
                assert!(
                    control(&fade, 0xf1, 1).is_some(),
                    "reaction fades with F1, parameter 1"
                );
            }
            252 => {
                assert_eq!(line, "240");
                track_port_pattern(&writes, 1);
                assert!(
                    fade.last().unwrap().frame < writes[control(&writes, 0xf0, 1).unwrap()].frame
                );
            }
            257 => {
                assert_eq!(line, "240");
                track_port_pattern(&writes, 7);
                assert_eq!(session.wram(0x47e), 0x0c, "C music returns after reaction");
            }
            // Continuation: the second spear conversation grants $242 in
            // map $42; later the route walks back through $21. These are
            // route/source-inferred *windows*, not native track-ID proofs:
            // $34 (fanfare), $1C (return) and $06 (map) all have the same
            // stop parameter 5; no upload fingerprint is compared here.
            // Only the bounded change port patterns and route state are witnessed.
            501 | 504 | 511 => {
                assert!(matches!(line, "180" | "240"));
                assert_ne!(
                    session.wram(0x6c0 + 0x242 / 8) & (1 << (0x242 % 8)),
                    0,
                    "spear granted on foot"
                );
                assert_eq!(session.wram(0x47e), if row == 511 { 0x21 } else { 0x42 });
                track_port_pattern(&writes, 5);
            }
            _ => {}
        }
    }
    eprintln!("EU route APU bounded port-pattern witness checked (effect values but NOT door order parity; not PCM/DSP; spear return 405 native vs 420 portable; door hit four frames earlier portable)");
    std::process::exit(0);
}

#[cfg(test)]
mod boundary_tests {
    use super::*;

    fn write(frame: u32, port: u8, value: u8) -> ApuPortWrite {
        ApuPortWrite {
            frame,
            scanline: 0,
            cycle: 0,
            port,
            value,
        }
    }

    #[test]
    fn control_parameter_must_be_in_the_same_frame() {
        assert_eq!(control(&[write(1, 1, 5), write(2, 0, 0xf0)], 0xf0, 5), None);
    }

    #[test]
    fn final_f4_pattern_rejects_later_upload_writes() {
        let writes = [
            write(1, 0, 0),
            write(1, 1, 5),
            write(1, 0, 0xf0),
            write(2, 0, 0xff),
            write(2, 2, 0x48),
            write(2, 3, 0x10),
            write(3, 0, 0xf4),
            write(3, 1, 3),
        ];
        track_port_pattern(&writes[..7], 5); // Same prefix has an isolated F4 pattern.
        assert!(std::panic::catch_unwind(|| track_port_pattern(&writes, 5)).is_err());
    }

    #[test]
    fn effect_positions_reject_a_reversed_hit_sequence() {
        let writes = [
            write(42, 2, 0),
            write(42, 3, 0x13),
            write(43, 2, 0),
            write(43, 3, 0x12),
        ];
        let mut previous = None;
        ordered_effect(&writes, 248, 0x1200, &mut previous);
        assert!(std::panic::catch_unwind(|| {
            let mut after = previous;
            ordered_effect(&writes, 248, 0x1300, &mut after);
        })
        .is_err());
    }
}
