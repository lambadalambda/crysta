//! Empty-SRAM European Elder's first choice: ROM page/cursor art versus native
//! indexed tiles and uploaded BG3 choice-window words (not composed RGB/video).
//! ares boots once per process, so this owned-ROM witness runs in a fresh child.
use assets::text::window::WindowArt;
use assets::text::{Acknowledgement, DialogueChoice, DialoguePage, HouseDialogue};
use oracle::{Button, Session};
use rom::{Revision, Rom};
use std::{path::Path, process::Command};

const TEST: &str = "european_first_elder_choice_matches_native_page_and_labels";

#[test]
fn european_first_elder_choice_matches_native_page_and_labels() {
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
    if std::env::var("EU_CHOICE_CHILD").is_ok() {
        inspect_native_choice(&rom);
        std::process::exit(0);
    }
    let out = Command::new(std::env::current_exe().expect("test executable"))
        .args(["--exact", TEST, "--nocapture"])
        .env("EU_CHOICE_CHILD", "1")
        .output()
        .expect("fresh native child");
    assert!(
        out.status.success()
            && String::from_utf8_lossy(&out.stderr).contains("EU native first Elder choice:"),
        "European choice witness failed or skipped\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn inspect_native_choice(rom: &Rom) {
    let pages = HouseDialogue::decode_at(rom.image(), 0x88_91c6).unwrap();
    let page = pages.last().expect("retained Elder question");
    let choice = HouseDialogue::choice_at(rom.image(), 2).unwrap();
    assert_eq!(
        page.acknowledgement(),
        Acknowledgement::None,
        "D4 retains the question"
    );
    assert_eq!(page.boundary_source(), 0x88_925b);
    assert_eq!((page.width(), page.height()), (224, 64));
    assert_eq!(
        choice.options.map(|o| (o.result, o.position)),
        [(1, [0, 32]), (2, [0, 48])]
    );
    // The requested y24/40 option-row bands include the ROM's glyph rows
    // at y32/48. Check ink in each label's own lower eight lines, not just
    // neighboring question text (the cursor occupies only x0..8).
    for (row, option) in [24, 40].into_iter().zip(choice.options) {
        let glyph_y = usize::from(option.position[1]);
        assert_eq!(glyph_y, row + 8);
        assert!(page
            .glyphs()
            .iter()
            .any(|glyph| usize::from(glyph.position[1]) == glyph_y));
        assert!(
            label_ink(page, glyph_y) > 0,
            "option label in y{row} row band"
        );
    }

    let art = WindowArt::from_rom(rom.image()).expect("European window art");
    assert!(art.cursor.iter().flatten().any(|&index| index != 3));
    assert!(art.interior.iter().all(|&index| index == 3));
    let mut session = Session::new(rom).expect("empty-SRAM European emulator");
    reach_elder_choice(&mut session);
    check_selection(&mut session, page, &choice, &art, 0);
    // One Down edge selects result 2, without answering.
    run(&mut session, 1, Some(Button::Down));
    run(&mut session, 18, None);
    check_selection(&mut session, page, &choice, &art, 1);
    eprintln!("EU native first Elder choice: both selections, visible/hidden BG3 cursors and 224 uploaded choice-window tilemap words/attributes match");
}

// Sample both phases without claiming an exact blink period or framebuffer timing.
fn check_selection(
    session: &mut Session,
    page: &DialoguePage,
    choice: &DialogueChoice,
    art: &WindowArt,
    selected: usize,
) {
    let mut seen = [false; 2];
    for _ in 0..80 {
        let wram = session.wram_image();
        assert_choice_wait(&wram);
        assert_eq!(word(&wram, 0x0dce), u16::try_from(selected).unwrap());
        let bg = session.bg3_state();
        assert_eq!(
            (
                bg.screen_address,
                bg.tiledata_address,
                bg.screen_size,
                bg.mode,
                bg.above_enable
            ),
            (0x6800, 0x7000, 0, 0, true),
            "BG3 active 32x32 2bpp window, $E000 characters"
        );
        let vram = session.vram();
        let [x, y] = choice.options[selected].position.map(usize::from);
        assert_eq!(x, 0);
        let at = usize::from(bg.screen_address) + 0x04c4 / 2 + y / 8 * 32;
        let visible = match [vram[at], vram[at + 32]] {
            [0x202c, 0x203c] => true,
            [0x2020, 0x2020] => false,
            other => panic!("unexpected selected BG3 cursor words {other:04x?}"),
        };
        if !seen[usize::from(visible)] {
            let bytes: Vec<u8> = vram.iter().flat_map(|w| w.to_le_bytes()).collect();
            compare_page(&wram, &bytes, page, choice, selected);
            compare_uploaded_window(&wram, &vram, page, choice, art, visible, bg.screen_address);
            seen[usize::from(visible)] = true;
        }
        if seen == [true, true] {
            return;
        }
        run(session, 1, None);
    }
    panic!(
        "both BG3 cursor phases not observed for result {}: {seen:?}",
        choice.options[selected].result
    );
}

fn compare_uploaded_window(
    wram: &[u8],
    vram: &[u16],
    page: &DialoguePage,
    choice: &DialogueChoice,
    art: &WindowArt,
    visible: bool,
    screen: u16,
) {
    let selected = usize::from(word(wram, 0x0dce));
    let bytes: Vec<u8> = vram.iter().flat_map(|w| w.to_le_bytes()).collect();
    let [cx, cy] = choice.options[selected].position.map(usize::from);
    let mut distinct = 0;
    // Content rectangle only: 28 columns x 8 rows, no neighboring BGs.
    for row in 0..8 {
        for column in 0..28 {
            let at = row * 32 + column;
            let staged = word(wram, 0x1_d000 + 0x04c4 + at * 2);
            let uploaded = vram[usize::from(screen) + 0x04c4 / 2 + at];
            let cursor_row = column == cx / 8 && (cy / 8..cy / 8 + 2).contains(&row);
            if cursor_row {
                let tile_id = if visible {
                    0x2c + (row - cy / 8) * 16
                } else {
                    0x20
                };
                let expected = u16::try_from(0x2000 | tile_id).unwrap();
                assert_eq!(staged, expected, "staged cursor tile/palette/priority/flip");
                assert_eq!(
                    uploaded, expected,
                    "uploaded cursor tile/palette/priority/flip"
                );
            } else {
                assert_eq!(
                    uploaded & 0x03ff,
                    staged & 0x03ff,
                    "uploaded tile ID at ({column},{row})"
                );
                assert_eq!(
                    uploaded & 0xfc00,
                    staged & 0xfc00,
                    "uploaded palette/priority/flip at ({column},{row})"
                );
                assert_eq!(uploaded, staged, "uploaded BG3 word at ({column},{row})");
            }
            distinct += usize::from(uploaded & 0x03ff != 0x20);
        }
    }
    assert!(
        distinct > 2,
        "nontrivial page tiles uploaded, not just cursor"
    );
    for row in 0..16 {
        for column in 0..8 {
            let expected = if visible {
                art.cursor[row / 8][row % 8 * 8 + column]
            } else {
                art.interior[row % 8 * 8 + column]
            };
            assert_eq!(
                page.indexed()[(cy + row) * 224 + cx + column],
                page.background_index(),
                "cursor source blank"
            );
            let tile = vram[usize::from(screen) + 0x04c4 / 2 + (cy + row) / 8 * 32 + cx / 8];
            assert_eq!(
                tile_index(&bytes, tile, column, row % 8),
                expected,
                "BG3 cursor indexed pixel ({column},{row}), visible={visible}, result {}",
                choice.options[selected].result
            );
        }
    }
}

fn tile_index(vram: &[u8], tile: u16, x: usize, y: usize) -> u8 {
    let tx = if tile & 0x4000 != 0 { 7 - x } else { x };
    let ty = if tile & 0x8000 != 0 { 7 - y } else { y };
    let at = (0xe000 + usize::from(tile & 0x03ff) * 16 + ty * 2) & 0xffff;
    (vram[at] >> (7 - tx) & 1) | ((vram[at + 1] >> (7 - tx) & 1) << 1)
}

fn reach_elder_choice(session: &mut Session) {
    // 1960 boot frames, then the native route's first 53 input rows: New
    // Game, Elle, walk to B, acknowledge three Elder pages, do not answer.
    run(session, 1800, None);
    run(session, 10, Some(Button::Start));
    run(session, 150, None);
    let rows: Vec<_> = include_str!("../../oracle/tests/fixtures/eu-pandora-tour.inputs")
        .lines()
        .take(53)
        .collect();
    assert_eq!(rows.len(), 53, "native European route prefix");
    for (row, line) in rows.into_iter().enumerate() {
        let mut words = line.split_whitespace();
        let frames: usize = words.next().unwrap().parse().expect("frame count");
        let held: Vec<_> = words
            .map(|button| match button {
                "Start" => Button::Start,
                "Down" => Button::Down,
                "A" => Button::A,
                "Right" => Button::Right,
                "Left" => Button::Left,
                "Up" => Button::Up,
                _ => panic!("unknown native route button {button} on row {}", row + 1),
            })
            .collect();
        for button in [
            Button::Start,
            Button::Down,
            Button::A,
            Button::Right,
            Button::Left,
            Button::Up,
        ] {
            session.set_button(button, held.contains(&button));
        }
        for _ in 0..frames {
            session.run_frame();
        }
    }
}

fn assert_choice_wait(wram: &[u8]) {
    assert_eq!(word(wram, 0x047e), 0x0b, "room B, before the Box");
    assert_eq!((word(wram, 0x1000), word(wram, 0x1002)), (120, 128));
    assert_ne!(
        wram[0x06c4] & (1 << (0x26 % 8)),
        0,
        "Elder grants $26 before answering"
    );
    assert_eq!(word(wram, 0x0dc2), 0xffff, "choice is still unanswered");
    assert_eq!(word(wram, 0x0db6), 0x04c4, "European window anchor");
}

fn label_ink(page: &DialoguePage, y: usize) -> usize {
    (y..y + 8)
        .flat_map(|row| (12..224).map(move |x| page.indexed()[row * 224 + x]))
        .filter(|&pixel| pixel != page.background_index())
        .count()
}

fn compare_page(
    wram: &[u8],
    vram: &[u8],
    page: &DialoguePage,
    choice: &DialogueChoice,
    selected: usize,
) -> usize {
    let [cursor_x, cursor_y] = choice.options[selected].position.map(usize::from);
    let mut count = 0;
    for y in 0..usize::from(page.height()) {
        for x in 0..usize::from(page.width()) {
            let expected = page.indexed()[y * 224 + x];
            if (cursor_x..cursor_x + 8).contains(&x) && (cursor_y..cursor_y + 16).contains(&y) {
                assert_eq!(expected, page.background_index(), "cursor source is blank");
                continue; // only the native blinking 8x16 cursor's source cell
            }
            assert_eq!(
                native_index(wram, vram, x, y),
                expected,
                "Elder choice at ({x},{y}), selected {}",
                choice.options[selected].result
            );
            count += 1;
        }
    }
    assert_eq!(
        count,
        224 * 64 - 8 * 16,
        "entire page except one cursor cell"
    );
    count
}

fn run(session: &mut Session, frames: usize, held: Option<Button>) {
    for button in [
        Button::Start,
        Button::Down,
        Button::A,
        Button::Right,
        Button::Left,
        Button::Up,
    ] {
        session.set_button(button, held == Some(button));
    }
    for _ in 0..frames {
        session.run_frame();
    }
}

fn word(wram: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([wram[at], wram[at + 1]])
}

fn native_index(wram: &[u8], vram: &[u8], x: usize, y: usize) -> u8 {
    // $7F:D4C4: European content tilemap (32 tiles wide, 2 bytes per tile).
    let at = 0x1_d000 + 0x04c4 + (y / 8) * 64 + (x / 8) * 2;
    let tile = word(wram, at);
    let tx = if tile & 0x4000 != 0 { 7 - x % 8 } else { x % 8 };
    let ty = if tile & 0x8000 != 0 { 7 - y % 8 } else { y % 8 };
    let address = (0xe000 + usize::from(tile & 0x03ff) * 16 + ty * 2) & 0xffff;
    (vram[address] >> (7 - tx) & 1) | ((vram[address + 1] >> (7 - tx) & 1) << 1)
}
