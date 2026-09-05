//! Local-only capture driver. Emulator sessions deliberately exit with the process.

use assets::maps::{LoadedMap, StaticLayer};
use oracle::{Button, Session};
use rom::{Revision, Rom};
use serde_json::{json, Value};
use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

mod opening_qualification;
mod qualification;
mod script_inspection;
mod visual_export;
mod visual_qualification;

const SAVE_SHA256: &str = "709c1cb67b8aff8db49cba05959f128b1c0a1ca32184c9bb62c415d537658055";
const VIEWER: &str = include_str!("../web/viewer.html");
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() {
    let status = match run(&env::args_os().skip(1).collect::<Vec<_>>()) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("error: {error}");
            1
        }
    };
    // ares owns process-global threads/statics; don't run their exit destructors.
    std::process::exit(status);
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn run(args: &[std::ffi::OsString]) -> Result<()> {
    let [mode, rom_path, parameter] = args else {
        return Err(invalid(
            "usage: map-inspector <capture|verify|qualify-loader|qualify-opening|trace-opening> <japanese-rom> <qualified-sram>\n       map-inspector decode-layer <japanese-rom> <hex-normalized-offset>\n       map-inspector <resolve-map|render-map> <japanese-rom> <hex-map-id>",
        )
        .into());
    };
    let export = match mode.to_str() {
        Some("capture") => true,
        Some("qualify-opening" | "trace-opening" | "verify" | "qualify-loader" | "decode-layer" | "resolve-map" | "render-map") => false,
        _ => return Err(invalid(
            "mode must be capture, verify, qualify-loader, qualify-opening, trace-opening, decode-layer, resolve-map or render-map",
        )
        .into()),
    };
    let rom = Rom::load(&fs::read(rom_path)?)?;
    if rom.revision() != Revision::Japan {
        return Err(invalid("map inspection is qualified only for the Japanese reference").into());
    }
    if mode == "render-map" {
        let map_id = u16::try_from(parse_hex(parameter)?)?;
        println!(
            "Local static map viewer: {}",
            visual_export::export(&rom, map_id)?.display()
        );
        return Ok(());
    }
    if mode == "resolve-map" {
        let map_id = u16::try_from(parse_hex(parameter)?)?;
        println!("{}", script_inspection::inspect(&rom, map_id)?);
        return Ok(());
    }
    if mode == "decode-layer" {
        println!("{}", decode_layer(&rom, parse_hex(parameter)?)?);
        return Ok(());
    }
    let save = fs::read(parameter)?;
    if sha256(&save) != SAVE_SHA256 {
        return Err(
            invalid("SRAM must match the qualified three-slot save; see docs/maps.md").into(),
        );
    }
    let mut session = Session::new_with_sram(&rom, &save)?;
    if mode == "qualify-loader" {
        println!("{}", qualification::run(&mut session, &rom)?);
        return Ok(());
    }
    if mode == "qualify-opening" || mode == "trace-opening" {
        println!(
            "{}",
            opening_qualification::run(&mut session, &rom, mode == "trace-opening")?
        );
        return Ok(());
    }
    let mut captures = Vec::new();
    for label in 0..=1840 {
        session.set_button(Button::Start, (400..408).contains(&label));
        session.set_button(Button::A, (1100..1112).contains(&label));
        session.set_button(Button::Right, (1800..1840).contains(&label));
        session.run_frame();
        if label == 1600 || label == 1840 {
            captures.push(checkpoint(&session, label)?);
        }
    }
    let manifest = json!({
        "schema_version":1,"revision":rom.revision().id(),"rom_sha256":sha256(rom.image()),
        "sram_sha256":SAVE_SHA256,"scenario":"qualified-slot-3-right-movement",
        "checkpoints":captures.iter().map(|capture|capture.metadata.clone()).collect::<Vec<_>>()
    });
    if export {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/maps");
        let directory = run_directory(&root)?;
        for capture in &captures {
            let label = capture.metadata["label"]
                .as_u64()
                .expect("numeric capture label");
            fs::write(
                directory.join(format!("reference-{label}.bmp")),
                bitmap(&capture.rgb, 256, 240)?,
            )?;
            fs::write(directory.join(format!("wram-{label}.bin")), &capture.wram)?;
            fs::write(directory.join(format!("vram-{label}.bin")), &capture.vram)?;
            fs::write(directory.join(format!("cgram-{label}.bin")), &capture.cgram)?;
        }
        fs::write(
            directory.join("capture.json"),
            serde_json::to_vec_pretty(&manifest)?,
        )?;
        fs::write(directory.join("index.html"), render_html(&manifest))?;
        println!(
            "Local map viewer: {}",
            directory.canonicalize()?.join("index.html").display()
        );
    } else {
        println!("{}", serde_json::to_string(&manifest)?);
    }
    Ok(())
}

fn parse_hex(value: &std::ffi::OsStr) -> Result<usize> {
    let text = value
        .to_str()
        .ok_or_else(|| invalid("expected hexadecimal text"))?;
    Ok(usize::from_str_radix(
        text.strip_prefix("0x").unwrap_or(text),
        16,
    )?)
}

fn decode_layer(rom: &Rom, offset: usize) -> Result<Value> {
    let layer = StaticLayer::from_rom(rom.image(), offset)?;
    Ok(json!({
        "schema_version":1,"kind":"static-layer-before-attributes",
        "revision":rom.revision().id(),"rom_sha256":sha256(rom.image()),
        "width":layer.width(),"height":layer.height(),
        "source_range":[layer.source_range().start,layer.source_range().end],
        "source_sha256":sha256(layer.source_bytes()),"layer_sha256":sha256(&layer.layer_bytes()),
        "cells":layer.cells().iter().map(|cell|cell.raw()).collect::<Vec<_>>()
    }))
}

struct Capture {
    metadata: Value,
    rgb: Vec<u8>,
    wram: Vec<u8>,
    vram: Vec<u8>,
    cgram: Vec<u8>,
}

fn checkpoint(session: &Session, label: u32) -> Result<Capture> {
    let wram = session.wram_image();
    let map = LoadedMap::from_wram(&wram)?;
    let expected_player = if label == 1600 {
        (776, 112)
    } else {
        (834, 128)
    };
    if map.map_id() != 0x0128
        || map.width() != 80
        || map.height() != 32
        || map.player() != expected_player
        || session.frame_state().frames != label + 1
    {
        return Err(
            invalid("checkpoint differs from qualified map $0128 / 80×32 / player state").into(),
        );
    }
    // This noninterlaced checkpoint has 512 horizontal samples and 240 rows;
    // take one sample per low-resolution pixel from the fixed 512×480 ABI.
    let rgb: Vec<_> = (0..240)
        .flat_map(|y| {
            (0..256).flat_map(move |x| {
                let p = &session.pixels()[(y * 512 + x * 2) * 4..][..4];
                [p[2], p[1], p[0]]
            })
        })
        .collect();
    let vram: Vec<_> = session
        .vram()
        .iter()
        .flat_map(|word| word.to_le_bytes())
        .collect();
    let cgram: Vec<_> = session
        .cgram()
        .iter()
        .flat_map(|word| word.to_le_bytes())
        .collect();
    let metadata = json!({
        "label":label,"frame":session.frame_state().frames,"map_id":map.map_id(),
        "width":map.width(),"height":map.height(),"camera":map.camera(),"player":map.player(),
        "cells":map.cells().iter().map(|cell|cell.raw()).collect::<Vec<_>>(),
        "layer_sha256":sha256(&map.layer_bytes()),"wram_sha256":sha256(&wram),
        "vram_sha256":sha256(&vram),"cgram_sha256":sha256(&cgram),"rgb_sha256":sha256(&rgb),
        "image":format!("reference-{label}.bmp")
    });
    Ok(Capture {
        metadata,
        rgb,
        wram,
        vram,
        cgram,
    })
}

fn sha256(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    rom::digests(bytes)
        .sha256
        .iter()
        .fold(String::new(), |mut text, byte| {
            write!(text, "{byte:02x}").unwrap();
            text
        })
}

fn run_directory(root: &Path) -> io::Result<PathBuf> {
    fs::create_dir_all(root)?;
    for sequence in 0..=u16::MAX {
        let path = root.join(format!("run-{}-{sequence:04X}", std::process::id()));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "no unused capture directory",
    ))
}

fn render_html(metadata: &Value) -> String {
    // Safe even if a future metadata field contains markup.
    VIEWER.replace(
        "__CAPTURE_JSON__",
        &metadata.to_string().replace('<', "\\u003c"),
    )
}

fn bitmap(rgb: &[u8], width: usize, height: usize) -> Result<Vec<u8>> {
    if width == 0 || height == 0 || width > 4096 || height > 4096 || rgb.len() != width * height * 3
    {
        return Err(invalid("invalid RGB bitmap dimensions or data length").into());
    }
    let stride = (width * 3 + 3) & !3;
    let size = 54 + stride * height;
    let mut bytes = vec![0; size];
    bytes[..2].copy_from_slice(b"BM");
    for (offset, value) in [
        (2, size),
        (10, 54),
        (14, 40),
        (18, width),
        (34, stride * height),
    ] {
        bytes[offset..offset + 4].copy_from_slice(&u32::try_from(value)?.to_le_bytes());
    }
    bytes[22..26].copy_from_slice(&(-i32::try_from(height)?).to_le_bytes());
    bytes[26..28].copy_from_slice(&1_u16.to_le_bytes());
    bytes[28..30].copy_from_slice(&24_u16.to_le_bytes());
    for y in 0..height {
        for x in 0..width {
            let from = (y * width + x) * 3;
            let to = 54 + y * stride + x * 3;
            bytes[to..to + 3].copy_from_slice(&[rgb[from + 2], rgb[from + 1], rgb[from]]);
        }
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::{bitmap, render_html};

    #[test]
    fn bitmap_encodes_rgb_in_top_down_bgr_rows() {
        let bytes = bitmap(&[255, 0, 0, 0, 255, 0], 2, 1).unwrap();
        assert_eq!(&bytes[..2], b"BM");
        assert_eq!(bytes.len(), 62);
        assert_eq!(&bytes[54..], [0, 0, 255, 0, 255, 0, 0, 0]);
        assert_eq!(i32::from_le_bytes(bytes[22..26].try_into().unwrap()), -1);
        assert!(bitmap(&[0], 2, 1).is_err());
        assert!(bitmap(&[], 0, 0).is_err());
    }

    #[test]
    fn html_embeds_json_without_script_termination() {
        let html = render_html(&serde_json::json!({"value":"</script><script>alert(1)</script>"}));
        assert!(!html.contains("</script><script>alert"));
        assert!(html.contains("\\u003c/script>"));
        assert!(!html.contains("__CAPTURE_JSON__"));
    }
}
