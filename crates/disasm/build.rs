//! Build script for the disasm crate.
//!
//! Assembles the SNES ROM through ca65/ld65 and verifies byte-for-byte
//! matching against the verified local dump. The assembly source starts
//! fully opaque (`.incbin` of the ROM) and is incrementally replaced with
//! real 65816 assembly as ranges are disassembled.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();

    println!("cargo:rerun-if-changed=asm/");
    println!("cargo:rerun-if-changed=build.rs");

    // Locate ca65 and ld65: prefer tools/cc65/ (local dev), then PATH.
    let (ca65, ld65) = find_tools(workspace_root);

    // Find the verified local ROM dump.
    let rom_path = find_rom(workspace_root);
    let rom_bytes = fs::read(&rom_path).expect("failed to read ROM");
    println!("cargo:rerun-if-changed={}", rom_path.display());

    // Strip SMC header if present.
    let clean = strip_smc_header(&rom_bytes);

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Write clean ROM to OUT_DIR.
    let clean_path = out_dir.join("rom-clean.bin");
    fs::write(&clean_path, clean).expect("failed to write clean ROM");

    // Patch asm/*.s files: replace `.incbin "rom-clean.bin"` with the
    // absolute path to the clean ROM in OUT_DIR, and write to OUT_DIR.
    let asm_dir = manifest_dir.join("asm");
    let mut obj_files: Vec<PathBuf> = Vec::new();

    for entry in fs::read_dir(&asm_dir).expect("failed to read asm/") {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("s") {
            continue;
        }
        let source = fs::read_to_string(&path).expect("failed to read .s");
        let patched = source.replace(
            ".incbin \"rom-clean.bin\"",
            &format!(".incbin \"{}\"", clean_path.display()),
        );
        let patched_path = out_dir.join(path.file_name().unwrap());
        fs::write(&patched_path, &patched).expect("failed to write patched .s");

        let obj = out_dir.join(path.file_stem().unwrap().to_str().unwrap().to_owned() + ".o");
        let status = Command::new(&ca65)
            .args([
                "--cpu",
                "65816",
                "-o",
                obj.to_str().unwrap(),
                patched_path.to_str().unwrap(),
            ])
            .status()
            .expect("failed to run ca65 — install cc65 or put ca65/ld65 in tools/cc65/");
        if !status.success() {
            panic!("ca65 failed on {}", path.display());
        }
        obj_files.push(obj);
    }

    // Write linker config: flat binary, full ROM size.
    let cfg_path = out_dir.join("rom.cfg");
    let cfg = format!(
        "MEMORY {{\n    ROM: start = $0000, size = ${:06x}, fill = yes;\n}}\n\
         SEGMENTS {{\n    ROM: load = ROM, type = ro;\n}}\n",
        clean.len()
    );
    fs::write(&cfg_path, &cfg).expect("failed to write linker config");

    // Link.
    let bin_path = out_dir.join("rom-built.bin");
    let mut ld_args: Vec<String> = vec![
        "-C".into(),
        cfg_path.to_str().unwrap().into(),
        "-o".into(),
        bin_path.to_str().unwrap().into(),
    ];
    for obj in &obj_files {
        ld_args.push(obj.to_str().unwrap().into());
    }
    let status = Command::new(&ld65)
        .args(&ld_args)
        .status()
        .expect("failed to run ld65 — install cc65 or put ca65/ld65 in tools/cc65/");
    if !status.success() {
        panic!("ld65 failed");
    }

    // Byte-for-byte comparison.
    let built = fs::read(&bin_path).expect("failed to read built ROM");
    assert_eq!(
        built.len(),
        clean.len(),
        "size mismatch: built {} bytes, expected {}",
        built.len(),
        clean.len()
    );

    let mut diffs: Vec<String> = Vec::new();
    for (i, (a, b)) in built.iter().zip(clean.iter()).enumerate() {
        if a != b {
            diffs.push(format!(
                "  offset ${:06x}: built ${:02x}, expected ${:02x}",
                i, a, b
            ));
            if diffs.len() >= 8 {
                break;
            }
        }
    }
    if !diffs.is_empty() {
        panic!(
            "byte mismatch at {} offset(s):\n{}\n\
             The assembly output does not reproduce the verified dump.",
            diffs.len(),
            diffs.join("\n")
        );
    }

    // Expose the clean ROM path for tests.
    println!("cargo:ROM_CLEAN={}", clean_path.display());
}

fn find_tools(workspace: &Path) -> (PathBuf, PathBuf) {
    let local = workspace.join("tools").join("cc65");
    let ca65 = if local.join("ca65").exists() {
        local.join("ca65")
    } else {
        which("ca65").expect("ca65 not found on PATH or in tools/cc65/")
    };
    let ld65 = if local.join("ld65").exists() {
        local.join("ld65")
    } else {
        which("ld65").expect("ld65 not found on PATH or in tools/cc65/")
    };
    (ca65, ld65)
}

fn which(name: &str) -> Option<PathBuf> {
    let output = Command::new("which").arg(name).output().ok()?;
    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Some(PathBuf::from(path));
        }
    }
    None
}

fn find_rom(workspace: &Path) -> PathBuf {
    let local = workspace.join("local");
    let candidates = ["Tenchi Souzou (Japan).sfc", "Terranigma (E) [!].smc"];
    for name in &candidates {
        let path = local.join(name);
        if path.exists() {
            return path;
        }
    }
    for entry in fs::read_dir(&local).expect("local/ not found") {
        let entry = entry.unwrap();
        let path = entry.path();
        if let Some("sfc" | "smc") = path.extension().and_then(|e| e.to_str()) {
            return path;
        }
    }
    panic!("no ROM dump found in local/");
}

fn strip_smc_header(bytes: &[u8]) -> &[u8] {
    if bytes.len() > 512 && ((bytes.len() - 512) & (bytes.len() - 513)) == 0 {
        &bytes[512..]
    } else {
        bytes
    }
}
