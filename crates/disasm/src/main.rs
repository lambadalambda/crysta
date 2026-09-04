//! Explicit local driver for the byte-matching reconstruction workflow.

use disasm::compare_images;
use rom::{Revision, Rom};
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
    if command != "reconstruct" || args.len() != 2 {
        return Err(usage_error());
    }

    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("disasm crate must be inside the workspace")
        .to_owned();
    reconstruct(&workspace, Path::new(&args[1]))
}

fn usage_error() -> Box<dyn std::error::Error> {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "usage: cargo run -p disasm -- reconstruct <path-to-japanese-rom>",
    )
    .into()
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

    let ca65 = find_tool(workspace, "ca65")?;
    let ld65 = find_tool(workspace, "ld65")?;
    print_tool_version(&ca65)?;
    print_tool_version(&ld65)?;

    let output_dir = create_run_directory(&workspace.join("local/disasm"))?;
    let clean_path = output_dir.join("rom-clean.bin");
    fs::write(&clean_path, reference.image())?;

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
    use super::{assembly_sources, create_run_directory, hex};
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
    fn digest_format_is_lowercase_hex() {
        assert_eq!(hex(&[0x00, 0xAB, 0xFF]), "00abff");
    }
}
