//! Empty-SRAM European Elder choices: ROM pages and cursor art versus native
//! indexed tiles. The first choice also checks uploaded BG3 window words; the
//! late doorway mission is input-only. Each owned-ROM witness runs in its own
//! fresh child because ares boots once per process. No composed RGB is claimed.
use assets::text::window::WindowArt;
use assets::text::{Acknowledgement, DialogueChoice, DialoguePage, HouseDialogue};
use oracle::{Button, Session};
use rom::{Revision, Rom};
use std::{path::Path, process::Command};

const TEST: &str = "european_first_elder_choice_matches_native_page_and_labels";
const MISSION_TEST: &str = "european_late_elder_mission_page_matches_native_indices";

#[test]
fn european_late_elder_mission_page_matches_native_indices() {
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
    if std::env::var("EU_MISSION_CHILD").is_ok() {
        inspect_mission(&rom);
        std::process::exit(0);
    }
    let out = Command::new(std::env::current_exe().expect("test executable"))
        .args(["--exact", MISSION_TEST, "--nocapture"])
        .env("EU_MISSION_CHILD", "1")
        .output()
        .expect("fresh native child");
    assert!(
        out.status.success()
            && String::from_utf8_lossy(&out.stderr).contains("EU native late Elder mission:"),
        "European mission witness failed or skipped\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn inspect_mission(rom: &Rom) {
    let pages = HouseDialogue::decode_at(rom.image(), 0x88_8d6c).unwrap();
    assert_eq!(pages.len(), 7, "late Elder dialogue");
    let edge = &pages[3];
    let question = pages.last().unwrap();
    let choice = HouseDialogue::choice_at(rom.image(), 2).unwrap();
    assert_eq!(
        (edge.boundary_source(), question.boundary_source()),
        (0x88_8e0e, 0x88_8e85)
    );
    assert_eq!((edge.width(), edge.height()), (216, 64));
    assert_eq!((question.width(), question.height()), (216, 64));
    assert_eq!(edge.acknowledgement(), Acknowledgement::Next);
    assert_eq!(edge.end(), [180, 48], "source-positioned D5 prompt");
    assert_eq!(question.acknowledgement(), Acknowledgement::None);
    let edge_glyph = edge
        .glyphs()
        .iter()
        .find(|g| g.text_source == 0x88_8de4 && g.position[0] == 204)
        .expect("rightmost source glyph");
    assert_eq!(
        choice.options.map(|o| (o.result, o.position)),
        [(1, [0, 32]), (2, [0, 48])]
    );
    let option_ink = assert_mission_option_ink(question, &choice);
    let mut session = Session::new(rom).expect("empty-SRAM European emulator");
    run(&mut session, 1800, None);
    run(&mut session, 10, Some(Button::Start));
    run(&mut session, 150, None);
    let tour = include_str!("../../oracle/tests/fixtures/eu-pandora-tour.inputs");
    let world = include_str!("../../oracle/tests/fixtures/eu-world-map.inputs");
    assert_eq!((tour.lines().count(), world.lines().count()), (453, 212));
    for line in tour.lines().chain(world.lines()).take(612) {
        replay(&mut session, line);
    }
    let w = session.wram_image();
    assert_eq!(
        (word(&w, 0x047e), word(&w, 0x1000), word(&w, 0x1002)),
        (0x0d, 120, 704)
    );
    assert_mission_state(&w);
    let remainder: Vec<_> = world.lines().skip(612 - 453).collect();
    let row = reach_boundary(&mut session, &remainder, edge.boundary_source());
    assert_eq!(row, 618 - 613, "edge page on the recorded route");
    run(&mut session, 2, None); // completed native tile upload; no acknowledgement
    let w = session.wram_image();
    assert_mission_state(&w);
    assert_eq!(
        text_source(&w),
        edge.boundary_source(),
        "edge page unacknowledged"
    );
    let pixels = compare_mission_page(&session, edge, None);
    assert_eq!(pixels, 216 * 64 - 16 * 16);
    // Compare all 12 in-bounds columns of the x204 glyph, not just its metadata.
    let glyph_y = usize::from(edge_glyph.position[1]);
    assert!((glyph_y..glyph_y + 16)
        .any(|y| (204..216).any(|x| edge.indexed()[y * 216 + x] != edge.background_index())));

    // The recorded row is idle; its unplayed tail is irrelevant. Continue at
    // the next input edge, and stop on the decoded final D4/choice wait.
    let choice_row = reach_boundary(
        &mut session,
        &remainder[row + 1..],
        question.boundary_source(),
    );
    eprintln!("mission choice row {}", 619 + choice_row);
    // D4 can enter the unanswered choice before the final text tiles reach
    // VRAM. Poll the decoded, uncovered raster instead of assuming a delay.
    let settled = (0..300).any(|_| {
        run(&mut session, 1, None);
        let w = session.wram_image();
        word(&w, 0x0dc2) == 0xffff && mission_raster_ready(&session, question, Some((&choice, 0)))
    });
    assert!(
        settled,
        "final mission question never reached VRAM while unanswered"
    );
    let w = session.wram_image();
    assert_mission_state(&w);
    assert_eq!(word(&w, 0x0dc2), 0xffff, "mission choice unanswered");
    assert_eq!(word(&w, 0x0dce), 0, "initial option");
    let first = compare_mission_page(&session, question, Some((&choice, 0)));
    run(&mut session, 1, Some(Button::Down));
    let moved = (0..300).any(|_| {
        run(&mut session, 1, None);
        let w = session.wram_image();
        word(&w, 0x0dc2) == 0xffff
            && word(&w, 0x0dce) == 1
            && mission_raster_ready(&session, question, Some((&choice, 1)))
    });
    assert!(
        moved,
        "second option never reached stable VRAM while unanswered"
    );
    let w = session.wram_image();
    assert_mission_state(&w);
    assert_eq!(word(&w, 0x0dc2), 0xffff, "Down does not answer");
    assert_eq!(word(&w, 0x0dce), 1, "second option");
    let second = compare_mission_page(&session, question, Some((&choice, 1)));
    eprintln!("EU native late Elder mission: {pixels} edge-page + {first}+{second} choice-page indexed pixels match, option ink {option_ink:?}, map $0D, $21 set, $296 clear, unanswered");
}

fn assert_mission_option_ink(page: &DialoguePage, choice: &DialogueChoice) -> [usize; 2] {
    let width = usize::from(page.width());
    choice.options.map(|option| {
        let [cx, cy] = option.position.map(usize::from);
        // The catalog supplies y32/48 cursor positions; the decoded mission
        // page places the corresponding label glyph cells at y16/32.
        let label_y = cy.checked_sub(16).expect("option label above cursor");
        assert!(
            page.glyphs().iter().any(|glyph| {
                usize::from(glyph.position[1]) == label_y
                    && usize::from(glyph.position[0]) >= cx + 8
            }),
            "option {} has a glyph on its own label row",
            option.result
        );
        let ink = (label_y..cy)
            .flat_map(|y| (cx + 8..width).map(move |x| page.indexed()[y * width + x]))
            .filter(|&pixel| pixel != page.background_index())
            .count();
        assert!(
            ink > 0,
            "option {} has ink outside its cursor cell",
            option.result
        );
        ink
    })
}

fn assert_mission_state(w: &[u8]) {
    assert_eq!(
        (word(w, 0x047e), word(w, 0x1000), word(w, 0x1002)),
        (0x0d, 120, 704)
    );
    for (id, expected) in [(0x21, true), (0x26, true), (0x296, false)] {
        assert_eq!(
            w[0x6c0 + id / 8] & (1 << (id % 8)) != 0,
            expected,
            "event {id:#x}"
        );
    }
    assert_eq!(word(w, 0x0db6), 0x00c4, "EU narrow content tilemap anchor");
}

fn text_source(w: &[u8]) -> u32 {
    u32::from(w[0x0dc2]) << 16 | u32::from(word(w, 0x0dc0))
}

fn reach_boundary(session: &mut Session, lines: &[&str], boundary: u32) -> usize {
    for (row, line) in lines.iter().enumerate() {
        let mut words = line.split_whitespace();
        let frames: usize = words.next().unwrap().parse().expect("frames");
        let held: Vec<_> = words.collect();
        set_held(session, &held);
        for _ in 0..frames {
            session.run_frame();
            let w = session.wram_image();
            if text_source(&w) == boundary || (boundary == 0x88_8e85 && word(&w, 0x0dc2) == 0xffff)
            {
                assert!(
                    held.is_empty(),
                    "boundary reached while input held on row {row}"
                );
                return row;
            }
        }
    }
    panic!("native text never reached boundary {boundary:#x}");
}

fn overlay(
    page: &DialoguePage,
    cursor: Option<(&DialogueChoice, usize)>,
    x: usize,
    y: usize,
) -> bool {
    if let Some((choice, selected)) = cursor {
        let [cx, cy] = choice.options[selected].position.map(usize::from);
        (cx..cx + 8).contains(&x) && (cy..cy + 16).contains(&y)
    } else if page.acknowledgement() == Acknowledgement::Next {
        // Native D5 prompt: one 16x16 cell at the decoded text end.
        let [px, py] = page.end().map(usize::from);
        (px..px + 16).contains(&x) && (py..py + 16).contains(&y)
    } else {
        false
    }
}

fn mission_raster_ready(
    session: &Session,
    page: &DialoguePage,
    cursor: Option<(&DialogueChoice, usize)>,
) -> bool {
    let w = session.wram_image();
    let vram: Vec<u8> = session
        .vram()
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    let width = usize::from(page.width());
    (0..usize::from(page.height())).all(|y| {
        (0..width).all(|x| {
            overlay(page, cursor, x, y)
                || native_index_at(&w, &vram, x, y, 0x00c4) == page.indexed()[y * width + x]
        })
    })
}

fn compare_mission_page(
    session: &Session,
    page: &DialoguePage,
    cursor: Option<(&DialogueChoice, usize)>,
) -> usize {
    let w = session.wram_image();
    let vram: Vec<u8> = session
        .vram()
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    let width = usize::from(page.width());
    let mut count = 0;
    for y in 0..usize::from(page.height()) {
        for x in 0..width {
            let expected = page.indexed()[y * width + x];
            if overlay(page, cursor, x, y) {
                assert_eq!(
                    expected,
                    page.background_index(),
                    "overlay source cell is blank"
                );
                continue;
            }
            assert_eq!(
                native_index_at(&w, &vram, x, y, 0x00c4),
                expected,
                "mission at ({x},{y})"
            );
            count += 1;
        }
    }
    let excluded = if cursor.is_some() {
        8 * 16
    } else if page.acknowledgement() == Acknowledgement::Next {
        16 * 16
    } else {
        0
    };
    assert_eq!(count, width * 64 - excluded);
    count
}

fn replay(session: &mut Session, line: &str) {
    let mut words = line.split_whitespace();
    let frames: usize = words.next().unwrap().parse().expect("frame count");
    let held: Vec<_> = words.collect();
    set_held(session, &held);
    for _ in 0..frames {
        session.run_frame();
    }
}

fn set_held(session: &mut Session, held: &[&str]) {
    for (name, button) in [
        ("Start", Button::Start),
        ("Down", Button::Down),
        ("A", Button::A),
        ("Right", Button::Right),
        ("Left", Button::Left),
        ("Up", Button::Up),
    ] {
        session.set_button(button, held.contains(&name));
    }
    assert!(held
        .iter()
        .all(|name| matches!(*name, "Start" | "Down" | "A" | "Right" | "Left" | "Up")));
}

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
    native_index_at(wram, vram, x, y, 0x04c4)
}

fn native_index_at(wram: &[u8], vram: &[u8], x: usize, y: usize, anchor: usize) -> u8 {
    // $7F:D000 + $0DB6: European content tilemap (32 tiles wide, 2 bytes per tile).
    let at = 0x1_d000 + anchor + (y / 8) * 64 + (x / 8) * 2;
    let tile = word(wram, at);
    let tx = if tile & 0x4000 != 0 { 7 - x % 8 } else { x % 8 };
    let ty = if tile & 0x8000 != 0 { 7 - y % 8 } else { y % 8 };
    let address = (0xe000 + usize::from(tile & 0x03ff) * 16 + ty * 2) & 0xffff;
    (vram[address] >> (7 - tx) & 1) | ((vram[address + 1] >> (7 - tx) & 1) << 1)
}
