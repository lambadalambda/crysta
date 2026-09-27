//! Headless, empty-SRAM European bedroom text compared with its native tiles.
//! ares boots once per process; the owned-ROM witness runs in a fresh child.
use assets::graphics::Bgr555;
use assets::text::window::WindowArt;
use assets::text::{Acknowledgement, DialoguePage, HouseDialogue, Placement};
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
            )
            && String::from_utf8_lossy(&out.stderr).contains(
                "8 stable CGRAM entries match (3 HDMA shade slots excluded); 3401 bounded native framebuffer samples unchanged (4512 shade-index candidates omitted); PAL row +29 control:"
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
    let cgram = session.cgram();
    // Slot 5 is the ROM-decoded speaker override rather than the generic
    // WindowArt palette colour; slots 3/7/11 are scanline HDMA shades.
    assert!(
        stable_cgram_matches(&cgram, &frame.colours, page.speaker()),
        "stable native BG3 CGRAM differs from decoded window/speaker colours"
    );
    let (compared, excluded, row_control, distinct) =
        check_native_framebuffer(&mut session, &wram, &vram, page);
    eprintln!("EU native first bedroom page: {count} glyph-cell and {border} frame pixels match; 8 stable CGRAM entries match (3 HDMA shade slots excluded); {compared} bounded native framebuffer samples unchanged ({excluded} shade-index candidates omitted); PAL row +29 control: {row_control} samples differ from unshifted rows; {distinct} composed RGB values, not attributed to BG3/CGRAM; no portable RGB parity or HDMA qualification");
}

fn check_native_framebuffer(
    session: &mut Session,
    wram: &[u8],
    vram: &[u8],
    page: &DialoguePage,
) -> (usize, usize, usize, usize) {
    let pixels = session.pixels().to_vec(); // last flushed frame, after the tile upload settled
    run(session, 1, None);
    let next_wram = session.wram_image();
    let at = u32::from(next_wram[0x0dc2]) << 16
        | u32::from(u16::from_le_bytes([next_wram[0x0dc0], next_wram[0x0dc1]]));
    assert_eq!(at, page.boundary_source(), "still waiting on the same page");
    let (samples, excluded) = stable_window_samples(wram, vram, page);
    // The owned ROM draws on its first visible scanline. In this PAL capture
    // the preceding non-overscan border is blank; row 29 is not. This catches
    // a mismatched overscan/viewport assumption in the source-derived offset.
    let lit = |row: usize| {
        (0..512)
            .filter(|&x| pixels[(row * 512 + x) * 4..][..3] != [0; 3])
            .count()
    };
    assert_eq!(
        (21..29).map(lit).sum::<usize>(),
        0,
        "pre-display PAL border"
    );
    assert!(lit(29) > 400, "first visible PAL scanline is buffer row 29");
    let row_control = samples
        .iter()
        .filter(|&&(x, y, _)| {
            let wrong = (y * 512 + x * 2) * 4;
            let mapped = (pal_buffer_row(y) * 512 + x * 2) * 4;
            pixels[wrong..wrong + 3] != pixels[mapped..mapped + 3]
        })
        .count();
    assert!(
        row_control > 100,
        "PAL row offset must change nontrivial crop samples"
    );
    let (compared, distinct) =
        consistent_crop(&pixels, session.pixels(), (8, 143, 240, 80), &samples)
            .expect("bounded stable-index crop changed between settled frames or is empty");
    assert!(
        compared > 500 && distinct >= 2,
        "non-vacuous native window crop"
    );
    (compared, excluded, row_control, distinct)
}

fn stable_cgram_matches(native: &[u16], decoded: &[Bgr555; 12], speaker: Bgr555) -> bool {
    (1..12)
        .filter(|index| ![3, 7, 11].contains(index))
        .all(|index| {
            let expected = if index == 5 { speaker } else { decoded[index] };
            native
                .get(index)
                .is_some_and(|&word| word == expected.raw())
        })
}

#[test]
fn stable_cgram_check_rejects_changed_entries_but_not_dynamic_shades() {
    let decoded = std::array::from_fn(|index| Bgr555::new(u16::try_from(index).unwrap()));
    let mut native: Vec<u16> = (0..256).collect();
    let speaker = Bgr555::new(0x5e3f);
    native[5] = speaker.raw();
    assert!(stable_cgram_matches(&native, &decoded, speaker));
    native[5] ^= 1;
    assert!(!stable_cgram_matches(&native, &decoded, speaker));
    native[5] ^= 1;
    native[2] ^= 1;
    assert!(!stable_cgram_matches(&native, &decoded, speaker));
    native[2] ^= 1;
    for index in [3, 7, 11] {
        native[index] ^= 1;
    }
    assert!(stable_cgram_matches(&native, &decoded, speaker));
    assert!(!stable_cgram_matches(&[], &decoded, speaker));
}

fn stable_window_samples(
    wram: &[u8],
    vram: &[u8],
    page: &DialoguePage,
) -> (Vec<(usize, usize, usize)>, usize) {
    // $04C4 is BG3 column 2, row 19. BG3 displays tile line y+1 at
    // screen line y; the 224x64 content begins at (16,151).
    let (mut samples, mut excluded) = (Vec::new(), 0);
    let (columns, rows) = (
        usize::from(page.width() / 8),
        usize::from(page.height() / 8),
    );
    for row in 0..rows + 2 {
        for column in 0..columns + 2 {
            let border = column == 0 || column == columns + 1 || row == 0 || row == rows + 1;
            let cell = 0x1_d000 + 0x04c4 - 64 - 2 + row * 64 + column * 2;
            let tile = u16::from_le_bytes([wram[cell], wram[cell + 1]]);
            for py in 0..8 {
                for px in 0..8 {
                    let (x, y) = (8 + column * 8 + px, 143 + row * 8 + py);
                    if !border {
                        let (gx, gy) = (x - 16, y - 151);
                        // Only the already-checked first-page glyph cells,
                        // not the shade-only interior or animated prompt.
                        if !page.glyphs().iter().any(|glyph| {
                            let [ax, ay] = glyph.position.map(usize::from);
                            (ax..ax + 12).contains(&gx) && (ay..ay + 16).contains(&gy)
                        }) {
                            continue;
                        }
                    }
                    let index = tile_index(vram, tile, px, py);
                    if index == 3 {
                        excluded += 1;
                    } else if index != 0 {
                        let slot = usize::from((tile >> 10) & 7) * 4 + usize::from(index);
                        assert!(slot < 12, "window BG3 palette outside decoded colours");
                        // Frame tiles and glyph cells have already been checked
                        // against the ROM's decoded indices above.
                        samples.push((x, y, slot));
                    }
                }
            }
        }
    }
    (samples, excluded)
}

// ares PPU-performance renders vcounter 1 at its first visible logical row;
// on non-overscan PAL it adds 8 lines plus a 20-line PAL border. The Screen
// exports its 288-row viewport; the shim crops width 564 to 512 but leaves
// height 288 uncropped in the 480-row buffer. Thus row y is y + 1 + 8 + 20.
const fn pal_buffer_row(logical_y: usize) -> usize {
    logical_y + 29
}

// Self-consistency of *native* RGB only: settled frames at the same bounded
// non-shade-index positions. Composition/scrolling can show other layers at
// those positions, so their RGB is not asserted to equal CGRAM or portable RGB.
// Horizontal samples use the established x*2 viewport sampling; the shim's
// centred 564->512 crop also shifts PAL content by two logical pixels, which
// is another reason not to attribute these composed samples to BG3 tiles.
fn consistent_crop(
    pixels: &[u8],
    next: &[u8],
    (left, top, width, height): (usize, usize, usize, usize),
    samples: &[(usize, usize, usize)],
) -> Option<(usize, usize)> {
    if pixels.len() != 512 * 480 * 4
        || next.len() != pixels.len()
        || samples.is_empty()
        || left + width > 256
        || top + height > 240
    {
        return None;
    }
    let mut colours = std::collections::HashSet::new();
    for &(x, y, slot) in samples {
        if x < left
            || x >= left + width
            || y < top
            || y >= top + height
            || slot >= 12
            || [3, 7, 11].contains(&slot)
        {
            return None;
        }
        let at = (pal_buffer_row(y) * 512 + x * 2) * 4;
        let rgb = [pixels[at + 2], pixels[at + 1], pixels[at]]; // XRGB8888
        if rgb != [next[at + 2], next[at + 1], next[at]] {
            return None;
        }
        colours.insert(rgb);
    }
    Some((samples.len(), colours.len()))
}

#[test]
fn native_crop_rejects_empty_and_corrupted_samples() {
    let pixels = vec![0u8; 512 * 480 * 4];
    let samples = [(16, 151, 1), (17, 151, 1)];
    assert_eq!(
        consistent_crop(&pixels, &pixels, (8, 143, 240, 80), &samples),
        Some((2, 1))
    );
    assert_eq!(
        consistent_crop(&pixels, &pixels, (8, 143, 240, 80), &[]),
        None
    );
    assert_eq!(
        consistent_crop(&[], &pixels, (8, 143, 240, 80), &samples),
        None
    );
    let mut corrupted = pixels.clone();
    corrupted[(pal_buffer_row(151) * 512 + 17 * 2) * 4 + 2] = 1;
    assert_eq!(
        consistent_crop(&pixels, &corrupted, (8, 143, 240, 80), &samples),
        None
    );
}

#[test]
fn pal_window_crop_uses_the_centered_native_scanline_not_logical_y() {
    assert_eq!(pal_buffer_row(0), 29);
    assert_eq!(pal_buffer_row(143), 172);
    assert_eq!(pal_buffer_row(222), 251);
    let mut pixels = vec![0; 512 * 480 * 4];
    let sample = [(16, 143, 1)];
    pixels[(172 * 512 + 16 * 2) * 4 + 2] = 17;
    let mut changed = pixels.clone();
    changed[(143 * 512 + 16 * 2) * 4 + 2] = 99;
    assert_eq!(
        consistent_crop(&pixels, &changed, (8, 143, 240, 80), &sample),
        Some((1, 1))
    );
    changed[(172 * 512 + 16 * 2) * 4 + 2] = 99;
    assert_eq!(
        consistent_crop(&pixels, &changed, (8, 143, 240, 80), &sample),
        None
    );
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
