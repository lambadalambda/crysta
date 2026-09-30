//! The cartridge's battery: the SRAM kept in a `.srm` beside the ROM, as
//! emulators keep it, so native saves and ours are the same file.

use crysta_runtime::sram::{Sram, SramSize};
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

/// Why a `.srm` could not be read.
#[derive(Debug)]
pub enum SramFileError {
    /// Reading failed.
    Io(io::Error),
    /// Not native SRAM; the file is left as it is.
    Size(SramSize),
}

impl fmt::Display for SramFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Size(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for SramFileError {}

/// The `.srm` beside `rom`.
#[must_use]
pub fn path(rom: &Path) -> PathBuf {
    rom.with_extension("srm")
}

/// The SRAM in `path`, or `None` when there is no file yet.
///
/// # Errors
/// A file that cannot be read or is not 8 KiB.
pub fn load(path: &Path) -> Result<Option<Sram>, SramFileError> {
    match std::fs::read(path) {
        Ok(bytes) => Sram::from_bytes(&bytes)
            .map(Some)
            .map_err(SramFileError::Size),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(SramFileError::Io(error)),
    }
}

/// Writes `sram` to `path` whole: a temporary file beside it, then a
/// rename, so a crash leaves the old file or the new one.
///
/// # Errors
/// Writing or renaming failed.
pub fn store(path: &Path, sram: &Sram) -> io::Result<()> {
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".tmp");
    std::fs::write(&temporary, sram.bytes())?;
    std::fs::rename(&temporary, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crysta_runtime::save::SaveSlot;

    fn scratch(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!("crysta-sram-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        directory.join(name)
    }

    #[test]
    fn the_srm_sits_beside_the_rom_and_round_trips() {
        assert_eq!(
            path(Path::new("/roms/Terranigma (E) [!].smc")),
            Path::new("/roms/Terranigma (E) [!].srm")
        );
        let file = scratch("round.srm");
        assert!(load(&file).unwrap().is_none(), "no file yet");
        let mut sram = Sram::default();
        sram.write_slot(2, &SaveSlot::default());
        store(&file, &sram).unwrap();
        assert_eq!(load(&file).unwrap(), Some(sram));
    }

    #[test]
    fn a_file_that_is_not_sram_is_refused_and_kept() {
        let file = scratch("short.srm");
        std::fs::write(&file, [1, 2, 3]).unwrap();
        assert!(matches!(load(&file), Err(SramFileError::Size(SramSize(3)))));
        assert_eq!(std::fs::read(&file).unwrap(), [1, 2, 3]);
    }
}
