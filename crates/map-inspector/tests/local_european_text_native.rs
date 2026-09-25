//! Headless, empty-SRAM European bedroom text compared with its native tiles.
//! ares boots once per process; the owned-ROM witness runs in a fresh child.
use assets::text::window::WindowArt;
use assets::text::{Acknowledgement, HouseDialogue, Placement};
use oracle::{Button, Session};
use rom::{Revision, Rom};
use std::{path::Path, process::Command};

const TEST: &str = "european_first_bedroom_page_matches_native_glyphs_and_frame";

#[test]
fn european_first_bedroom_page_matches_native_glyphs_and_frame() {
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
    if std::env::var("EU_TEXT_CHILD").is_ok() {
        inspect_native_page(&rom);
        std::process::exit(0);
    }
    let out = Command::new(std::env::current_exe().expect("test executable"))
        .args(["--exact", TEST, "--nocapture"])
        .env("EU_TEXT_CHILD", "1")
        .output()
        .expect("fresh native child");
    assert!(
        out.status.success()
            && String::from_utf8_lossy(&out.stderr).contains(
                "EU native first bedroom page: 4992 glyph-cell and 4864 frame pixels match"
            ),
        "European text witness failed or skipped\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn inspect_native_page(rom: &Rom) {
    let pages = HouseDialogue::decode_at(rom.image(), 0x88_9c15).unwrap();
    let page = &pages[0];
    assert_eq!(page.placement(), Placement::Bottom);
    assert_eq!(page.acknowledgement(), Acknowledgement::Next);
    assert_eq!((page.width(), page.height()), (224, 64));
    assert_eq!(page.background_index(), 3);
    assert_eq!(page.speaker().raw(), 0x5e3f);
    assert_eq!(page.glyphs()[0].palette, 1);
    assert_eq!(page.glyphs().len(), 26, "entire first English page");
    assert_eq!(page.indexed().len(), 224 * 64);
    let mut session = Session::new(rom).expect("empty-SRAM European emulator");
    // Input-only New Game and default name, shared with the native EU route.
    run(&mut session, 1800, None);
    run(&mut session, 10, Some(Button::Start));
    run(&mut session, 150, None);
    for _ in 0..3 {
        run(&mut session, 12, Some(Button::Down));
        run(&mut session, 18, None);
    }
    run(&mut session, 100, None);
    run(&mut session, 12, Some(Button::A));
    run(&mut session, 300, None);
    run(&mut session, 12, Some(Button::Start));
    run(&mut session, 800, None);
    run(&mut session, 12, Some(Button::A));

    let word = |w: &[u8], offset: usize| u16::from_le_bytes([w[offset], w[offset + 1]]);
    let mut waiting = false;
    for _ in 0..1200 {
        run(&mut session, 1, None);
        let w = session.wram_image();
        let at = u32::from(w[0x0dc2]) << 16 | u32::from(word(&w, 0x0dc0));
        if at == page.boundary_source() {
            waiting = true;
            break;
        }
    }
    assert!(waiting, "native EU page never reached its D5 wait");
    run(&mut session, 2, None); // let the completed tile upload be visible, without A.
    let wram = session.wram_image();
    let at = u32::from(wram[0x0dc2]) << 16 | u32::from(word(&wram, 0x0dc0));
    assert_eq!(at, page.boundary_source(), "same unacknowledged page");
    assert_eq!(word(&wram, 0x047e), 0x0f);
    assert_eq!((word(&wram, 0x1000), word(&wram, 0x1002)), (304, 112));
    assert_eq!(wram[0x06c4] & 1, 0, "Elle has not granted $20");
    assert_eq!(word(&wram, 0x0db6), 0x04c4, "European window anchor");
    let vram: Vec<u8> = session
        .vram()
        .iter()
        .flat_map(|w| w.to_le_bytes())
        .collect();
    let mut seen = vec![false; page.indexed().len()];
    let (mut count, mut ink) = (0, 0);
    for glyph in page.glyphs() {
        let [gx, gy] = glyph.position.map(usize::from);
        for y in gy..gy + 16 {
            for x in gx..gx + 12 {
                let offset = y * 224 + x;
                if seen[offset] {
                    continue;
                }
                seen[offset] = true;
                let actual = native_index(&wram, &vram, x, y);
                let expected = page.indexed()[offset];
                assert_eq!(
                    actual, expected,
                    "native EU glyph text={:#x} font={:#x} at ({x},{y})",
                    glyph.text_source, glyph.font_source
                );
                ink += usize::from(actual != page.background_index());
                count += 1;
            }
        }
    }
    assert_eq!(count, 4_992, "every first-page glyph cell compared");
    assert!(ink > 0, "native page has real foreground ink");
    let frame = WindowArt::from_rom(rom.image()).expect("European window frame");
    let border = compare_native_frame(&wram, &vram, &frame, page.width(), page.height());
    assert_eq!(border, 4_864, "all 76 frame tiles compared");
    eprintln!("EU native first bedroom page: {count} glyph-cell and {border} frame pixels match");
}

fn run(session: &mut Session, frames: usize, held: Option<Button>) {
    for button in [Button::Start, Button::Down, Button::A] {
        session.set_button(button, held == Some(button));
    }
    for _ in 0..frames {
        session.run_frame();
    }
}

fn compare_native_frame(
    wram: &[u8],
    vram: &[u8],
    art: &WindowArt,
    width: u16,
    height: u16,
) -> usize {
    let (columns, rows) = (i32::from(width / 8), i32::from(height / 8));
    let (mut count, mut ink) = (0, 0);
    for row in -1..=rows {
        for column in -1..=columns {
            let kind = match (column == -1, column == columns, row == -1, row == rows) {
                (true, _, true, _) => 0,
                (_, true, true, _) => 2,
                (_, _, true, _) => 1,
                (true, _, _, true) => 5,
                (_, true, _, true) => 7,
                (_, _, _, true) => 6,
                (true, ..) => 3,
                (_, true, ..) => 4,
                _ => continue,
            };
            let cell = 0x04c4_i32 + row * 64 + column * 2;
            let at = 0x1_d000 + usize::try_from(cell).expect("frame tilemap address");
            let tile = u16::from_le_bytes([wram[at], wram[at + 1]]);
            assert_eq!(tile & 0x03ff, 0x10 + kind, "frame tile at ({column},{row})");
            for (pixel, &expected) in art.frame[usize::from(kind)].iter().enumerate() {
                let actual = tile_index(vram, tile, pixel % 8, pixel / 8);
                assert_eq!(actual, expected, "frame ({column},{row}) pixel {pixel}");
                ink += usize::from(actual != 0);
                count += 1;
            }
        }
    }
    assert!(ink > 0, "native frame has visible edges");
    count
}

fn native_index(wram: &[u8], vram: &[u8], x: usize, y: usize) -> u8 {
    // $7F:D4C4: European content tilemap (32 tiles wide, 2 bytes per tile).
    let at = 0x1_d000 + 0x04c4 + (y / 8) * 64 + (x / 8) * 2;
    let tile = u16::from_le_bytes([wram[at], wram[at + 1]]);
    tile_index(vram, tile, x % 8, y % 8)
}

fn tile_index(vram: &[u8], tile: u16, x: usize, y: usize) -> u8 {
    let tx = if tile & 0x4000 != 0 { 7 - x } else { x };
    let ty = if tile & 0x8000 != 0 { 7 - y } else { y };
    let address = (0xe000 + usize::from(tile & 0x03ff) * 16 + ty * 2) & 0xffff;
    (vram[address] >> (7 - tx) & 1) | ((vram[address + 1] >> (7 - tx) & 1) << 1)
}
