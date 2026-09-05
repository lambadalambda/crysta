//! Bounded stopped-instruction evidence for source-backed NewGame operations.
use oracle::{export::digest_hex, CpuTraceStop, Session};
use serde_json::json;
use std::path::Path;

pub fn plan(mode: &str) -> Option<(u32, &'static [u32])> {
    match mode {
        "trace-reset" => Some((901, &[0x87CCA7, 0x86B93A, 0x87CCCC])),
        "trace-default" => Some((964, &[0x8780EC, 0x80BB77, 0x80BB9F, 0x8780F0])),
        "trace-map" => Some((2264, &[0x9087A6, 0x808A23, 0x808A53, 0x9087B0])),
        "trace-spawn" => Some((
            2422,
            &[0x80F42F, 0x80F5F9, 0x80F7F3, 0x80F80F, 0x80F815, 0x80F8E1],
        )),
        "trace-release" => Some((5902, &[0x889791, 0x808669, 0x80BB77, 0x80BB9F, 0x889795])),
        _ => None,
    }
}

pub fn capture(session: &mut Session, out: &Path, mode: &str) {
    let (start, targets) = plan(mode).unwrap();
    let mut stops = Vec::new();
    for &pc in targets {
        // All chosen stops remain within the initial completed-frame interval;
        // no input release boundary is crossed by a diagnostic trace.
        let trace = session.trace_until_pc(pc, 2_000_000, 2).unwrap();
        assert_eq!(trace.stop, CpuTraceStop::TargetReached, "missed {pc:06x}");
        assert_eq!(session.frame_state().frames, start);
        let w = session.wram_image();
        let u = |a| u16::from_le_bytes([w[a], w[a + 1]]);
        let r = session.cpu_registers();
        stops.push(json!({
            "pc": pc, "completed_frames": start,
            "a": r.accumulator, "x": r.x, "y": r.y, "db": r.data_bank, "p": r.status,
            "pending_map": u(0x47C), "selector": u(0x490), "queue": [u(0x492), u(0x494)],
            "position": [u(0x1000), u(0x1002)], "flags": u(0x1004), "player_index": u(0xDEA),
            "event_nonzero": w[0x6C0..0x700].iter().enumerate().filter(|(_, b)| **b != 0).collect::<Vec<_>>(),
            "wram_sha256": digest_hex(&w), "cpu_trace_sha256": trace.digest_hex(),
            "instructions": trace.entries.len(),
        }));
        std::fs::write(out.join(format!("stop-{pc:06x}.wram")), &w).unwrap();
        let text = trace
            .entries
            .iter()
            .map(|e| {
                format!(
                    "{:06x} p={:02x} d={:04x} db={:02x}\n",
                    e.address, e.status, e.direct_page, e.data_bank
                )
            })
            .collect::<String>();
        std::fs::write(out.join(format!("stop-{pc:06x}.trace")), text).unwrap();
    }
    std::fs::write(
        out.join("native-stops.json"),
        serde_json::to_vec_pretty(&stops).unwrap(),
    )
    .unwrap();
}
