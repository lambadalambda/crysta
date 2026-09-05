//! Fresh F -> 10 -> F + ordinary reactivation witness. No SRAM, patches/restores.
use oracle::{export::digest_hex, Button, Session};
use serde_json::json;
use std::io::Write;
mod bootstrap;

const ROUTE: &[(Button, u32, u32)] = &[
    (Button::Right, 6800, 6862),
    (Button::Down, 6900, 6967),
    (Button::Up, 7100, 7119),
    (Button::Right, 7260, 7280),
    (Button::Left, 7310, 7330),
    (Button::Right, 7360, 7380),
    (Button::Left, 7410, 7430),
];
const END: u32 = 7460;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 3, "usage: route-probe ROM OUT");
    let out = std::path::Path::new(&args[2]);
    assert!(
        out.starts_with("local")
            && !out
                .components()
                .any(|c| c == std::path::Component::ParentDir)
    );
    std::fs::create_dir(out).expect("new local output directory");
    let rom = rom::Rom::load(&std::fs::read(&args[1]).unwrap()).unwrap();
    assert_eq!(rom.revision(), rom::Revision::Japan);
    let bg = assets::maps::visual::StaticBackground::from_rom(rom.image(), 15).unwrap();
    assert_eq!((bg.layer().width(), bg.layer().height()), (32, 64));
    let attributes: &[u8; 512] = bg.resources()[3].decoded().try_into().unwrap();
    let static_cells = bg.layer().attributed_cells(attributes);
    let static_grid: Vec<_> = static_cells
        .iter()
        .flat_map(|c| c.raw().to_le_bytes())
        .collect();
    std::fs::write(out.join("static-bg1-grid.bin"), &static_grid).unwrap();
    let mut session = Session::new(&rom).unwrap();
    let buttons = [
        ("Start", Button::Start),
        ("A", Button::A),
        ("Up", Button::Up),
        ("Down", Button::Down),
        ("Left", Button::Left),
        ("Right", Button::Right),
    ];
    let mut csv = std::io::BufWriter::new(std::fs::File::create(out.join("frames.csv")).unwrap());
    writeln!(csv,"frame,input,map,x,y,flags,flags8,resume,timer,dir,ptrx,ptry,cntx,cnty,outx,outy,dx,dy,anim,joy,edge,window,last").unwrap();
    let mut checkpoints = Vec::new();
    let mut grid_differences = Vec::new();
    for label in 0..END {
        for &(_, button) in &buttons {
            session.set_button(
                button,
                bootstrap::INPUTS
                    .iter()
                    .chain(ROUTE)
                    .any(|&(b, start, end)| b == button && (start..end).contains(&label)),
            );
        }
        session.run_frame();
        let frame = label + 1;
        if frame < 6800 {
            continue;
        }
        assert_eq!(session.frame_state().frames, frame);
        let w = session.wram_image();
        let u = |i| u16::from_le_bytes([w[i], w[i + 1]]);
        let i = |j| u(j) as i16;
        let active = buttons
            .iter()
            .filter(|(_, b)| {
                ROUTE
                    .iter()
                    .any(|&(r, start, end)| r == *b && (start..end).contains(&label))
            })
            .map(|(name, _)| *name)
            .collect::<Vec<_>>()
            .join("+");
        writeln!(csv,"{frame},{active},{:x},{},{},{:04x},{:04x},{:02x}{:04x},{},{},{:04x},{:04x},{},{},{},{},{},{},{:04x},{:04x},{:04x},{},{:04x}",
            u(0x47e),u(0x1000),u(0x1002),u(0x1004),u(0x1008),w[0x100c],u(0x100a),u(0x100e),u(0x1014),
            u(0x11010),u(0x11012),i(0x1028),i(0x102a),i(0x1100c),i(0x1100e),i(0x11018),i(0x1101a),
            u(0x13014),u(0x454),u(0x456),u(0x96a),u(0x96c)).unwrap();
        if frame == 6800 {
            assert_eq!((u(0x47e), u(0x1000), u(0x1002)), (15, 304, 112));
            assert_eq!(
                digest_hex(&w),
                "49ab74b6af5283a4fb8151e56cdafc27339276213eaa7f1e997fe65e55b0ea82"
            );
            for (index, cell) in static_cells.iter().enumerate() {
                let runtime = u(0xa000 + index * 2);
                assert_eq!(runtime & 0x7fff, cell.raw(), "BG1 low15 cell {index}");
                if runtime != cell.raw() {
                    grid_differences.push(json!({"index": index, "x": index % 32, "y": index / 32,
                        "static": cell.raw(), "runtime": runtime}));
                }
            }
            std::fs::write(out.join("runtime-grid.bin"), &w[0xa000..0xb000]).unwrap();
        }
        if matches!(
            frame,
            6800 | 6967 | 6968 | 6984 | 6989 | 7050 | 7114 | 7131 | 7141 | 7250
        ) || frame == END
        {
            checkpoints.push(json!({"frame": frame, "map": u(0x47e), "player": [u(0x1000),u(0x1002)],
                "flags": [u(0x1004),u(0x1008)], "resume": u32::from(u(0x100a)) | (u32::from(w[0x100c]) << 16),
                "wram_sha256": digest_hex(&w), "grid_sha256": digest_hex(&w[0xa000..0xb000]),
                "event_sha256": digest_hex(&w[0x6c0..0x700])}));
            std::fs::write(out.join(format!("f{frame}.wram")), &w).unwrap();
            std::fs::write(out.join(format!("f{frame}.pixels")), session.pixels()).unwrap();
        }
    }
    csv.flush().unwrap();
    let report = json!({"version": 1, "rom_sha256": digest_hex(rom.image()), "start": 6800, "end": END,
        "frames_sha256": digest_hex(&std::fs::read(out.join("frames.csv")).unwrap()),
        "runtime_grid_sha256": digest_hex(&std::fs::read(out.join("runtime-grid.bin")).unwrap()),
        "static_bg1_grid_sha256": digest_hex(&static_grid), "grid_differences": grid_differences,
        "checkpoints": checkpoints});
    std::fs::write(
        out.join("route.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    std::process::exit(0);
}
