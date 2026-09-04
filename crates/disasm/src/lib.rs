//! ROM reconstruction support for the Terranigma matching disassembly.
//!
//! The ordinary Rust workspace build is ROM-free. Invoke the `disasm` binary
//! explicitly to validate a local Japanese dump, assemble the reconstruction,
//! and compare the resulting image byte-for-byte.

use std::fmt;

const HIROM_BASE: usize = 0xC0_0000;

/// The first difference between an expected ROM image and a reconstructed one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageMismatch {
    offset: usize,
    expected: Option<u8>,
    built: Option<u8>,
    expected_len: usize,
    built_len: usize,
}

impl ImageMismatch {
    /// Returns the normalized ROM file offset where the images first differ.
    #[must_use]
    pub fn offset(&self) -> usize {
        self.offset
    }
}

impl fmt::Display for ImageMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let address = HIROM_BASE + self.offset;
        let bank = address >> 16;
        let offset = address & 0xFFFF;

        match (self.built, self.expected) {
            (Some(built), Some(expected)) => write!(
                f,
                "byte mismatch at file offset ${:06X} (SNES ${bank:02X}:{offset:04X}): built ${built:02X}, expected ${expected:02X}",
                self.offset
            ),
            (built, expected) => {
                let built = built.map_or_else(|| "ended".to_owned(), |b| format!("${b:02X}"));
                let expected =
                    expected.map_or_else(|| "ended".to_owned(), |b| format!("${b:02X}"));
                write!(
                    f,
                    "length mismatch at file offset ${:06X} (SNES ${bank:02X}:{offset:04X}): built {built}, expected {expected}; built length {}, expected {}",
                    self.offset, self.built_len, self.expected_len
                )
            }
        }
    }
}

impl std::error::Error for ImageMismatch {}

/// Compares a reconstructed ROM with the normalized reference image.
///
/// # Errors
///
/// Returns the first differing byte or the first offset missing from either
/// image. The diagnostic includes both normalized file and canonical `HiROM`
/// addresses.
pub fn compare_images(expected: &[u8], built: &[u8]) -> Result<(), ImageMismatch> {
    let shared_len = expected.len().min(built.len());
    let offset = expected[..shared_len]
        .iter()
        .zip(&built[..shared_len])
        .position(|(expected, built)| expected != built)
        .or_else(|| (expected.len() != built.len()).then_some(shared_len));

    offset.map_or(Ok(()), |offset| {
        Err(ImageMismatch {
            offset,
            expected: expected.get(offset).copied(),
            built: built.get(offset).copied(),
            expected_len: expected.len(),
            built_len: built.len(),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::compare_images;

    #[test]
    fn matching_images_are_accepted() {
        assert_eq!(compare_images(&[0x10, 0x20], &[0x10, 0x20]), Ok(()));
    }

    #[test]
    fn one_byte_mismatch_reports_file_and_snes_offsets() {
        let error = compare_images(&[0x10, 0x20, 0x30], &[0x10, 0x21, 0x30])
            .expect_err("changed byte must be rejected");

        assert_eq!(error.offset(), 1);
        assert_eq!(
            error.to_string(),
            "byte mismatch at file offset $000001 (SNES $C0:0001): built $21, expected $20"
        );
    }

    #[test]
    fn length_mismatch_reports_first_missing_offset() {
        let error =
            compare_images(&[0x10, 0x20], &[0x10]).expect_err("truncated output must be rejected");

        assert_eq!(error.offset(), 1);
        assert_eq!(
            error.to_string(),
            "length mismatch at file offset $000001 (SNES $C0:0001): built ended, expected $20; built length 1, expected 2"
        );
    }

    #[test]
    fn longer_output_reports_first_extra_offset() {
        let error =
            compare_images(&[0x10], &[0x10, 0x20]).expect_err("extended output must be rejected");

        assert_eq!(error.offset(), 1);
        assert!(error.to_string().contains("built $20, expected ended"));
    }
}
