//! Explicit local driver for the byte-matching reconstruction workflow.

use disasm::{
    canonical_rom_map_include, canonical_symbol_include, compare_images, DataKind, RegionClass,
    RomMap,
};
use rom::{CanonicalRomAddress, NormalizedOffset, Revision, Rom, RuntimeRomAddress};
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let args: Vec<OsString> = env::args_os().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[OsString]) -> Result<(), Box<dyn std::error::Error>> {
    let Some(command) = args.first().and_then(|arg| arg.to_str()) else {
        return Err(usage_error());
    };
    match (command, args) {
        ("reconstruct", [_, rom_path]) => {
            let workspace = workspace_root();
            reconstruct(&workspace, Path::new(rom_path))
        }
        ("inspect-rom", [_, location]) => {
            let location = location.to_str().ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "address must be UTF-8")
            })?;
            println!("{}", inspect_rom_location(location)?);
            Ok(())
        }
        _ => Err(usage_error()),
    }
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("disasm crate must be inside the workspace")
        .to_owned()
}

fn usage_error() -> Box<dyn std::error::Error> {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "usage:\n  cargo run -p disasm -- reconstruct <path-to-japanese-rom>\n  cargo run -p disasm -- inspect-rom <offset:XXXXXX|canonical:XXXXXX|runtime:XX:XXXX>",
    )
    .into()
}

fn inspect_rom_location(query: &str) -> Result<String, Box<dyn std::error::Error>> {
    let (kind, value) = query.split_once(':').ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "address must include offset:, canonical:, or runtime:",
        )
    })?;
    let parsed = parse_hex(value)?;
    let normalized = match kind {
        "offset" => NormalizedOffset::new(parsed)?,
        "canonical" => CanonicalRomAddress::new(parsed)?.normalized(),
        "runtime" => RuntimeRomAddress::new(parsed)?.normalized(),
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "address must use offset:, canonical:, or runtime:",
            )
            .into());
        }
    };
    let map = RomMap::built_in_japan()?;
    let region = map.region_at(normalized).map_or_else(
        || "region=unclassified".to_owned(),
        |region| {
            let class = match region.class {
                RegionClass::Code => "class=code".to_owned(),
                RegionClass::Data(kind) => {
                    format!("class=data kind={}", data_kind_name(kind))
                }
            };
            format!("region={} {class}", region.id)
        },
    );
    let entry = map.entry_at(normalized).map_or_else(
        || "entry=none".to_owned(),
        |entry| format!("entry={}", entry.id),
    );
    let canonical = normalized.canonical().value();
    Ok(format!(
        "normalized={normalized} canonical=${:02X}:{:04X} {region} {entry}",
        canonical >> 16,
        canonical & 0xFFFF
    ))
}

fn data_kind_name(kind: DataKind) -> &'static str {
    match kind {
        DataKind::Raw => "raw",
        DataKind::VectorTable => "vector_table",
        DataKind::FunctionPointerTable => "function_pointer_table",
        DataKind::CallbackRecords => "callback_records",
        DataKind::ScriptEntryTable => "script_entry_table",
    }
}

fn parse_hex(value: &str) -> Result<u32, Box<dyn std::error::Error>> {
    let compact = value
        .strip_prefix('$')
        .or_else(|| value.strip_prefix("0x"))
        .unwrap_or(value)
        .replace(':', "");
    u32::from_str_radix(&compact, 16)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error).into())
}

fn reconstruct(workspace: &Path, rom_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let source = fs::read(rom_path)?;
    let reference = Rom::load(&source)?;
    if reference.revision() != Revision::Japan {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "matching reconstruction requires the Japanese reference revision",
        )
        .into());
    }

    let rom_map = RomMap::built_in_japan()?;
    rom_map.validate_rom(&reference)?;
    let cop_targets = rom_map.resolve_dispatch("cop_services", &reference)?;

    let ca65 = find_tool(workspace, "ca65")?;
    let ld65 = find_tool(workspace, "ld65")?;
    print_tool_version(&ca65)?;
    print_tool_version(&ld65)?;

    let output_dir = create_run_directory(&workspace.join("local/disasm"))?;
    let clean_path = output_dir.join("rom-clean.bin");
    fs::write(&clean_path, reference.image())?;
    fs::write(
        output_dir.join("memory-symbols.inc"),
        canonical_symbol_include()?,
    )?;
    fs::write(output_dir.join("rom-map.inc"), canonical_rom_map_include()?)?;

    let asm_dir = workspace.join("crates/disasm/asm");
    let sources = assembly_sources(&asm_dir)?;

    let mut objects = Vec::with_capacity(sources.len());
    for source_path in sources {
        let stem = source_path.file_stem().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "assembly source has no stem")
        })?;
        let object_path = output_dir.join(stem).with_extension("o");
        let listing_path = output_dir.join(stem).with_extension("lst");

        run_command(
            Command::new(&ca65)
                .arg("--cpu")
                .arg("65816")
                .arg("-W")
                .arg("2")
                .arg("--bin-include-dir")
                .arg(&output_dir)
                .arg("--include-dir")
                .arg(&output_dir)
                .arg("--listing")
                .arg(&listing_path)
                .arg("-o")
                .arg(&object_path)
                .arg(&source_path),
            "ca65",
        )?;
        objects.push(object_path);
    }

    let built_path = output_dir.join("rom-built.bin");
    let map_path = output_dir.join("rom.map");
    let linker_config = workspace.join("crates/disasm/linker.cfg");
    let mut linker = Command::new(&ld65);
    linker
        .arg("--config")
        .arg(linker_config)
        .arg("--mapfile")
        .arg(map_path)
        .arg("-o")
        .arg(&built_path);
    linker.args(&objects);
    run_command(&mut linker, "ld65")?;

    let built = fs::read(&built_path)?;
    compare_images(reference.image(), &built)?;
    let digest = rom::digests(&built).sha256;
    println!(
        "matched Japanese reference: {} bytes, sha256={}",
        built.len(),
        hex(&digest)
    );
    println!(
        "validated ROM map: {} regions, {} entries, {} COP targets",
        rom_map.regions().len(),
        rom_map.entry_points().len(),
        cop_targets.len()
    );
    println!("local output: {}", output_dir.display());
    Ok(())
}

fn create_run_directory(root: &Path) -> io::Result<PathBuf> {
    fs::create_dir_all(root)?;
    for sequence in 0..=u16::MAX {
        let candidate = root.join(format!("run-{}-{sequence:04X}", std::process::id()));
        match fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!("no unused run directory available under {}", root.display()),
    ))
}

fn assembly_sources(directory: &Path) -> io::Result<Vec<PathBuf>> {
    let mut sources = Vec::new();
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.extension() == Some(OsStr::new("s")) {
            sources.push(path);
        }
    }
    if sources.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("no assembly sources found in {}", directory.display()),
        ));
    }
    sources.sort();
    Ok(sources)
}

fn find_tool(workspace: &Path, name: &str) -> io::Result<PathBuf> {
    let executable = format!("{name}{}", env::consts::EXE_SUFFIX);
    let configured = env::var_os("TERRANIGMA_CC65_DIR").map(PathBuf::from);
    let local = workspace.join("tools/cc65");
    let path_directories = env::var_os("PATH")
        .map(|path| env::split_paths(&path).collect::<Vec<_>>())
        .unwrap_or_default();
    let candidates = configured
        .into_iter()
        .chain(std::iter::once(local))
        .chain(path_directories)
        .map(|directory| directory.join(&executable));

    candidates
        .into_iter()
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "{name} not found; install cc65, set TERRANIGMA_CC65_DIR, or place tools in tools/cc65/"
                ),
            )
        })
}

fn print_tool_version(tool: &Path) -> io::Result<()> {
    let output = Command::new(tool).arg("--version").output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "{} --version exited with {}",
            tool.display(),
            output.status
        )));
    }
    let version = if output.stdout.is_empty() {
        &output.stderr
    } else {
        &output.stdout
    };
    println!("{}", String::from_utf8_lossy(version).trim());
    Ok(())
}

fn run_command(command: &mut Command, name: &str) -> io::Result<()> {
    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "{name} exited with status {status}"
        )))
    }
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|byte| {
            [
                DIGITS[(byte >> 4) as usize] as char,
                DIGITS[(byte & 0x0F) as usize] as char,
            ]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{assembly_sources, create_run_directory, hex, inspect_rom_location};
    use std::fs;

    #[test]
    fn run_directories_never_reuse_existing_outputs() {
        let root =
            std::env::temp_dir().join(format!("terranigma-disasm-runs-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);

        let first = create_run_directory(&root).unwrap();
        fs::write(first.join("rom-clean.bin"), "do not overwrite").unwrap();
        let second = create_run_directory(&root).unwrap();

        assert_ne!(first, second);
        assert_eq!(
            fs::read_to_string(first.join("rom-clean.bin")).unwrap(),
            "do not overwrite"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_discovery_is_sorted() {
        let root = std::env::temp_dir().join(format!("terranigma-disasm-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("z.s"), "").unwrap();
        fs::write(root.join("a.s"), "").unwrap();
        fs::write(root.join("ignored.txt"), "").unwrap();

        let sources = assembly_sources(&root).unwrap();
        let names: Vec<_> = sources
            .iter()
            .map(|path| path.file_name().unwrap().to_owned())
            .collect();
        assert_eq!(names, ["a.s", "z.s"]);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn inspection_normalizes_explicit_address_spaces() {
        let offset = inspect_rom_location("offset:008000").unwrap();
        let canonical = inspect_rom_location("canonical:C08000").unwrap();
        let runtime = inspect_rom_location("runtime:80:8000").unwrap();
        assert_eq!(offset, canonical);
        assert_eq!(offset, runtime);
        assert!(offset.contains("normalized=$008000"));
        assert!(offset.contains("canonical=$C0:8000"));
        assert!(offset.contains("region=boot_main class=code"));
        assert!(offset.contains("entry=reset"));

        let table = inspect_rom_location("offset:0083B2").unwrap();
        assert!(table.contains("region=cop_dispatch_table class=data"));
        assert!(table.contains("kind=function_pointer_table"));
    }

    #[test]
    fn inspection_rejects_ambiguous_or_unmapped_addresses() {
        assert!(inspect_rom_location("808000").is_err());
        assert!(inspect_rom_location("runtime:00:7fff").is_err());
        assert!(inspect_rom_location("offset:400000").is_err());
    }

    #[test]
    fn digest_format_is_lowercase_hex() {
        assert_eq!(hex(&[0x00, 0xAB, 0xFF]), "00abff");
    }
}
