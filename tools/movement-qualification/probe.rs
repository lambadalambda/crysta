use oracle::{Button, Session};
use std::io::Write;
fn main() {
    let a: Vec<_> = std::env::args().collect();
    let out = &a[1];
    let end: u32 = a[2].parse().unwrap();
    std::fs::create_dir_all(out).unwrap();
    let rom = rom::Rom::load(&std::fs::read("local/Tenchi Souzou (Japan).sfc").unwrap()).unwrap();
    let mut s = Session::new_with_sram(&rom, &std::fs::read("local/saves/Terranigma.srm").unwrap())
        .unwrap();
    let buttons = [
        ("Start", Button::Start),
        ("A", Button::A),
        ("Up", Button::Up),
        ("Down", Button::Down),
        ("Left", Button::Left),
        ("Right", Button::Right),
    ];
    let mut inputs = vec![
        ("Start", 400..408),
        ("Up", 900..908),
        ("Up", 950..958),
        ("A", 1100..1112),
    ];
    for v in &a[3..] {
        let p: Vec<_> = v.split(':').collect();
        inputs.push((p[0], p[1].parse().unwrap()..p[2].parse().unwrap()));
    }
    let mut csv = std::fs::File::create(format!("{out}/frames.csv")).unwrap();
    writeln!(csv,"frame,input,map,x,y,flags,flags8,resume,timer,dir,ptrx,ptry,cntx,cnty,outx,outy,dx,dy,anim,joy,edge").unwrap();
    for f in 0..end {
        for (n, b) in buttons {
            s.set_button(
                b,
                inputs.iter().any(|(name, r)| *name == n && r.contains(&f)),
            );
        }
        s.run_frame();
        if f + 1 >= 1601 {
            let w = s.wram_image();
            let u = |i| u16::from_le_bytes([w[i], w[i + 1]]);
            let i = |j| u(j) as i16;
            if f + 1 == 1601 && (u(0x47e), u(0x1000), u(0x1002)) != (15, 472, 176) {
                eprintln!("bootstrap mismatch: use the authenticated SRAM, not an empty save");
                std::process::exit(1);
            }
            let active = inputs
                .iter()
                .filter(|(_, r)| r.contains(&f))
                .map(|(n, _)| *n)
                .collect::<Vec<_>>()
                .join("+");
            writeln!(csv,"{},{},{:x},{},{},{:04x},{:04x},{:02x}{:04x},{},{},{:04x},{:04x},{},{},{},{},{},{},{:04x},{:04x},{:04x}",f+1,active,u(0x47e),u(0x1000),u(0x1002),u(0x1004),u(0x1008),w[0x100c],u(0x100a),u(0x100e),u(0x1014),u(0x11010),u(0x11012),i(0x1028),i(0x102a),i(0x1100c),i(0x1100e),i(0x11018),i(0x1101a),u(0x13014),u(0x454),u(0x456)).unwrap();
            if f + 1 == 1601 || f + 1 == end {
                std::fs::write(format!("{out}/f{}.wram", f + 1), &w).unwrap();
            }
        }
    }
    if let Ok(pcs) = std::env::var("PCS") {
        for (k, p) in pcs.split(',').enumerate() {
            let pc = u32::from_str_radix(p, 16).unwrap();
            let tr = s.trace_until_pc(pc, 2_000_000, 4).unwrap();
            let r = s.cpu_registers();
            let w = s.wram_image();
            let u = |i| u16::from_le_bytes([w[i], w[i + 1]]);
            println!("step={k} target={pc:06x} stop={:?} regs={r:?} frame={} pos={},{} dxdy={},{} scratch={:02x?}",tr.stop,s.frame_state().frames,u(0x1000),u(0x1002),u(0x11018) as i16,u(0x1101a) as i16,&w[0x66..0x90]);
            std::fs::write(format!("{out}/step{k}-{pc:06x}.wram"), w).unwrap();
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
            std::fs::write(format!("{out}/step{k}-{pc:06x}.trace"), text).unwrap();
        }
    }
    csv.flush().unwrap();
    std::process::exit(0);
}
