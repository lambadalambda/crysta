//! One empty-SRAM European Crysta title phase, in a fresh native child.
//! This witnesses source-shaped OBJ uploads, palette and current OAM at one
//! visible-position phase, not exact animation cadence or framebuffer RGB.
//! Crystal Thread ($32) remains source-decoding evidence: showing it naturally
//! requires an out-of-route shop fixture, not a patched inventory/map state.
use assets::graphics::Tile4bpp;
use assets::labels::{area_title, label_palette, Glyph, TitleMotion};
use assets::maps::scripts::EventFlags;
use oracle::{Button, Session};
use rom::{Revision, Rom};
use std::{path::Path, process::Command};

const TEST: &str = "crysta_area_title_is_uploaded_and_configured_natively";

#[test]
fn crysta_area_title_is_uploaded_and_configured_natively() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/Terranigma (E) [!].smc");
    let image = match std::fs::read(path) {
        Ok(image) => image,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("skipping: owned European ROM absent");
            return;
        }
        Err(error) => panic!("cannot read owned European ROM: {error}"),
    };
    let rom = Rom::load(&image).expect("authenticated European dump");
    assert_eq!(rom.revision(), Revision::EuropeEnglish);
    if std::env::var_os("EU_TITLE_CHILD").is_some() {
        inspect(&rom);
        std::process::exit(0); // ares is a process singleton
    }
    let out = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", TEST, "--nocapture"])
        .env("EU_TITLE_CHILD", "1")
        .output()
        .expect("fresh native child");
    assert!(
        out.status.success()
            && String::from_utf8_lossy(&out.stderr).contains("EU native Crysta title:"),
        "title witness failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn word(w: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([w[at], w[at + 1]])
}

fn run(s: &mut Session, n: usize) {
    for _ in 0..n {
        s.run_frame();
    }
}

fn inspect(rom: &Rom) {
    let glyphs = area_title(rom.image(), 0x0a, EventFlags::Bitmap(&[0; 512]))
        .unwrap()
        .unwrap();
    let palette = label_palette(rom.image()).unwrap();
    let motion = TitleMotion::from_rom(rom.image()).unwrap();
    assert_eq!(glyphs.len(), 6, "Crysta, not another map's title");
    assert!(glyphs.iter().all(|g| g.iter().any(|&p| p != 0)));
    assert!(glyphs.iter().flatten().all(|&p| p <= 2));
    assert_ne!(palette[1], palette[2]);
    assert!(
        motion.positions(6, 0).is_empty(),
        "not a constant position table"
    );

    let mut s = Session::new(rom).expect("fresh empty-SRAM European session");
    run(&mut s, 1800);
    s.set_button(Button::Start, true);
    run(&mut s, 10);
    s.set_button(Button::Start, false);
    run(&mut s, 150);
    // Retained native real-button tour; first 73 commands naturally enter $0A
    // from Elder's house. No state restore, map seed, or WRAM writes.
    let route = include_str!("../../oracle/tests/fixtures/eu-pandora-tour.inputs");
    assert_eq!(route.lines().count(), 453);
    for line in route.lines().take(73) {
        let mut parts = line.split_whitespace();
        let n: usize = parts.next().unwrap().parse().unwrap();
        let held: Vec<_> = parts.collect();
        for (button, name) in [
            (Button::Start, "Start"),
            (Button::A, "A"),
            (Button::Down, "Down"),
            (Button::Up, "Up"),
            (Button::Left, "Left"),
            (Button::Right, "Right"),
        ] {
            s.set_button(button, held.contains(&name));
        }
        run(&mut s, n);
    }
    let w = s.wram_image();
    assert_eq!(word(&w, 0x47e), 0x0a, "natural arrival in Crysta");
    assert_eq!(
        w[0x6c0 + 0x14 / 8] & (1 << (0x14 % 8)),
        0,
        "title not suppressed"
    );
    // Command 74 is 140 neutral frames. Find a stable *current* six-piece
    // phase, then advance and recapture OAM/VRAM/CGRAM together. Do not
    // treat OAM as a latch for any prior framebuffer.
    let mut found = false;
    for _ in 0..140 {
        s.run_frame();
        let hw = s.sprite_state();
        let phase = (0..=motion.length() + 24).find(|&t| {
            let positions = motion.positions(6, i64::from(t));
            positions.len() == 6
                && positions.iter().enumerate().all(|(i, &(index, x, y))| {
                    let b = &hw.oam[i * 4..i * 4 + 4];
                    index == i && i32::from(b[0]) == x && i32::from(b[1]) == y
                })
        });
        if let Some(t) = phase {
            // Only accept a fully settled phase, not a transient coincidence.
            if motion.positions(6, i64::from(t)) != motion.positions(6, i64::from(t + 2)) {
                continue;
            }
            run(&mut s, 2);
            let now = s.sprite_state();
            if (0..6).any(|i| now.oam[i * 4..i * 4 + 2] != hw.oam[i * 4..i * 4 + 2]) {
                continue;
            }
            assert_eq!(word(&s.wram_image(), 0x47e), 0x0a);
            compare_phase(&s, &glyphs, &palette, &motion, t + 2);
            found = true;
            break;
        }
    }
    assert!(
        found,
        "no stable source-motion six-glyph title phase on natural entry"
    );
    eprintln!("EU native Crysta title: six current OAM pieces configured at a visible position, with uploaded OBJ glyph pixels and palette at one settled phase");
}

fn compare_phase(
    s: &Session,
    glyphs: &[Glyph],
    palette: &[assets::graphics::Bgr555; 16],
    motion: &TitleMotion,
    t: u32,
) {
    let hw = s.sprite_state();
    assert_eq!(
        hw.obsel & 0xe7,
        2,
        "OBJ base $8000, 8/16 size mode, no alternate base"
    );
    assert_eq!(hw.first_sprite, 0);
    let positions = motion.positions(6, i64::from(t));
    assert_eq!(positions.len(), 6);
    let vram: Vec<u8> = s.vram().iter().flat_map(|v| v.to_le_bytes()).collect();
    let cgram = s.cgram();
    let mut ink = 0;
    let mut used = [false; 3];
    for (i, &(index, x, y)) in positions.iter().enumerate() {
        assert_eq!(index, i);
        let b = &hw.oam[i * 4..i * 4 + 4];
        let hi = (hw.oam[512 + i / 4] >> (2 * (i % 4))) & 3;
        assert_eq!(
            (b[0], b[1], hi),
            (u8::try_from(x).unwrap(), u8::try_from(y).unwrap(), 2),
            "current 16x16 title OBJ"
        );
        assert_eq!(b[3], 0x34, "OBJ palette 2, no name-select or flip");
        // Six contiguous tile pairs in the label upload area, not Ark's
        // separate OBJ pieces. SNES 16x16 uses +1 and +16 for right/bottom.
        assert_eq!(b[2], 0xb0 + 2 * u8::try_from(i).unwrap());
        for gy in 0..16 {
            for gx in 0..16 {
                let tile = usize::from(b[2]) + gx / 8 + (gy / 8) * 16;
                let at = 0x8000 + tile * 32;
                let actual = Tile4bpp::decode(&vram[at..at + 32])
                    .unwrap()
                    .pixel(gx % 8, gy % 8)
                    .unwrap();
                let expected = glyphs[i][gy * 16 + gx];
                assert_eq!(
                    actual, expected,
                    "glyph {i} pixel ({gx},{gy}) in current OBJ VRAM"
                );
                ink += usize::from(actual != 0);
                used[usize::from(actual)] = true;
            }
        }
    }
    assert!(ink > 100, "nonempty six-glyph upload");
    assert!(used[1] && used[2], "both source ink colors uploaded");
    // Refuse ambiguous title ownership: all other current OAM pieces must
    // miss the sampled title band, even if the scene has Ark below it.
    let left = positions[0].1;
    let right = positions[5].1 + 16;
    let top = positions[0].2;
    for slot in 6..128 {
        let b = &hw.oam[slot * 4..slot * 4 + 4];
        let hi = (hw.oam[512 + slot / 4] >> (2 * (slot % 4))) & 3;
        let sx = i32::from(b[0]) + i32::from(hi & 1) * 256;
        let sy = i32::from(b[1]);
        let size = if hi & 2 == 0 { 8 } else { 16 }; // OBJSEL size mode 0
        assert!(
            sx >= right || sx + size <= left || sy >= top + 16 || sy + size <= top,
            "unowned OAM slot {slot} overlaps title band"
        );
    }
    for color in 1..=2 {
        assert_eq!(
            cgram[128 + 2 * 16 + color] & 0x7fff,
            palette[color].raw() & 0x7fff,
            "native OBJ palette 2 from European source"
        );
    }
    // Negative controls: wrong glyph order / empty tiles / palette 1 must not
    // pass the same witness, even when the surrounding scene has sprites.
    assert_ne!(glyphs[0], glyphs[1]);
    assert_ne!(cgram[128 + 16 + 1] & 0x7fff, palette[1].raw() & 0x7fff);
}
