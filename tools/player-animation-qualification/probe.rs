//! Input-only fresh boot; one Session/process, explicit exit (no restore/patch).
use oracle::{export::digest_hex, Button, CpuTraceStop, Session};
use serde_json::json;
use std::io::Write;
mod bootstrap;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert!(args.len() >= 4, "probe ROM OUT END [Right:6800:6862 ...]");
    let out = std::path::Path::new(&args[2]);
    assert!(
        out.starts_with("local")
            && !out
                .components()
                .any(|c| c == std::path::Component::ParentDir)
    );
    std::fs::create_dir(out).unwrap();
    let end: u32 = args[3].parse().unwrap();
    let buttons = [
        ("Start", Button::Start),
        ("A", Button::A),
        ("Up", Button::Up),
        ("Down", Button::Down),
        ("Left", Button::Left),
        ("Right", Button::Right),
    ];
    let mut inputs = bootstrap::INPUTS.to_vec();
    for arg in &args[4..] {
        let p: Vec<_> = arg.split(':').collect();
        inputs.push((
            buttons.iter().find(|(n, _)| *n == p[0]).unwrap().1,
            p[1].parse().unwrap(),
            p[2].parse().unwrap(),
        ));
    }
    let rom = rom::Rom::load(&std::fs::read(&args[1]).unwrap()).unwrap();
    assert_eq!(rom.revision(), rom::Revision::Japan);
    let mut s = Session::new(&rom).unwrap();
    let mut csv = std::io::BufWriter::new(std::fs::File::create(out.join("frames.csv")).unwrap());
    writeln!(csv,"frame,map,x,y,flags,flip,resume,timer,facing,base,sequence,cursor,composition,stream,outx,outy,joy,input").unwrap();
    for label in 0..end {
        for &(_, b) in &buttons {
            s.set_button(
                b,
                inputs
                    .iter()
                    .any(|&(k, a, z)| b == k && (a..z).contains(&label)),
            );
        }
        s.run_frame();
        if label + 1 < 6800 {
            continue;
        }
        let w = s.wram_image();
        let u = |a| u16::from_le_bytes([w[a], w[a + 1]]);
        if label + 1 == 6800 {
            assert_eq!(
                digest_hex(&w),
                "49ab74b6af5283a4fb8151e56cdafc27339276213eaa7f1e997fe65e55b0ea82"
            );
            std::fs::write(out.join("f6800.wram"), &w).unwrap();
        }
        let submitted: u16 = [
            (Button::Right, 0x100),
            (Button::Left, 0x200),
            (Button::Down, 0x400),
            (Button::Up, 0x800),
        ]
        .iter()
        .filter(|&&(b, _)| {
            inputs
                .iter()
                .any(|&(k, a, z)| b == k && (a..z).contains(&label))
        })
        .map(|&(_, mask)| mask)
        .sum();
        writeln!(
            csv,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            label + 1,
            u(0x47e),
            u(0x1000),
            u(0x1002),
            u(0x1004),
            u(0x1008),
            u32::from(u(0x100a)) | (u32::from(w[0x100c]) << 16),
            u(0x100e),
            u(0x1014),
            u32::from(u(0x1010)) | (u32::from(w[0x1012]) << 16),
            u(0x11008),
            u(0x1020),
            u(0x1100a),
            u(0x13014),
            u(0x1100c) as i16,
            u(0x1100e) as i16,
            u(0x454),
            submitted
        )
        .unwrap();
    }
    csv.flush().unwrap();
    std::fs::write(out.join("final.wram"), s.wram_image()).unwrap();
    let mut stops = Vec::new();
    if let Ok(pcs) = std::env::var("PCS") {
        for (i, pc) in pcs.split(',').enumerate() {
            let pc = u32::from_str_radix(pc, 16).unwrap();
            let tr = s.trace_until_pc(pc, 2_000_000, 2).unwrap();
            assert_eq!(tr.stop, CpuTraceStop::TargetReached);
            assert_eq!(
                s.frame_state().frames,
                end,
                "trace crossed completed-frame boundary"
            );
            let w = s.wram_image();
            let u = |a| u16::from_le_bytes([w[a], w[a + 1]]);
            let r = s.cpu_registers();
            stops.push(json!({"pc":pc,"frame":s.frame_state().frames,"a":r.accumulator,"x":r.x,"y":r.y,
                "db":r.data_bank,"p":r.status,"timer":u(0x100e),"facing":u(0x1014),"sequence":u(0x11008),
                "cursor":u(0x1020),"composition":u(0x1100a),"wram_sha256":digest_hex(&w),
                "trace_sha256":tr.digest_hex(),"instructions":tr.entries.len()}));
            std::fs::write(out.join(format!("stop-{i}.wram")), &w).unwrap();
            let text = tr
                .entries
                .iter()
                .map(|e| {
                    format!(
                        "{:06x} p={:02x} d={:04x} db={:02x}\n",
                        e.address, e.status, e.direct_page, e.data_bank
                    )
                })
                .collect::<String>();
            std::fs::write(out.join(format!("stop-{i}.trace")), text).unwrap();
        }
    }
    std::fs::write(
        out.join("report.json"),
        serde_json::to_vec_pretty(&json!({"rom_sha256":digest_hex(rom.image()),
        "frames_sha256":digest_hex(&std::fs::read(out.join("frames.csv")).unwrap()),"stops":stops}))
        .unwrap(),
    )
    .unwrap();
    std::process::exit(0);
}
