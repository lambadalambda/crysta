//! ROM normalization, validation, and addressing for Terranigma cartridge
//! dumps.
//!
//! This crate never reads or writes files. Callers own the bytes; everything
//! here operates on in-memory images. The canonical local tooling entry point
//! that loads files lives in `tools/` and always writes ROM-derived output
//! under the ignored `local/` directory.

/// A known, supported cartridge revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision {
    /// Tenchi Souzou, the original Japanese release.
    Japan,
    /// Terranigma, the European English release.
    EuropeEnglish,
}

impl Revision {
    /// The normalized-image SHA-256 digest of this revision.
    #[must_use]
    pub fn sha256(self) -> [u8; 32] {
        match self {
            Self::Japan => {
                hex_to_bytes("f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548")
            }
            Self::EuropeEnglish => {
                hex_to_bytes("93ba50d853e98e1ca227a2ca72389c0e3ac18d6b50c946b3f618c16c2d3edd38")
            }
        }
    }

    /// The normalized-image CRC32 of this revision.
    #[must_use]
    pub fn crc32(self) -> u32 {
        match self {
            Self::Japan => 0x3CC7_FDF4,
            Self::EuropeEnglish => 0x9745_23FF,
        }
    }

    /// The internal title recorded in the SNES header at file offset
    /// `0xFFC0` of the normalized image.
    #[must_use]
    pub fn internal_title(self) -> &'static str {
        match self {
            Self::Japan => "TENCHI-JPN",
            Self::EuropeEnglish => "TERRANIGMA P",
        }
    }
}

fn hex_to_bytes(s: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (i, pair) in s.as_bytes().chunks(2).enumerate() {
        let hi = (pair[0] as char).to_digit(16).expect("valid hex");
        let lo = (pair[1] as char).to_digit(16).expect("valid hex");
        out[i] = (hi * 16 + lo).try_into().expect("hex digit pair fits u8");
    }
    out
}

/// Errors produced while loading a cartridge image.
#[derive(Debug, PartialEq, Eq)]
pub enum LoadError {
    /// The image is too small to contain an SNES header at all.
    TooSmall {
        /// Actual image length in bytes.
        len: usize,
    },
    /// The length matches no known image layout.
    UnknownSize {
        /// Actual image length in bytes.
        len: usize,
    },
    /// The image has a recognized size but its normalized form matches no
    /// supported revision.
    UnknownRevision {
        /// Computed SHA-256 of the normalized image.
        sha256: [u8; 32],
        /// Which normalization was attempted.
        attempted: &'static str,
    },
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooSmall { len } => {
                write!(f, "image is too small to contain an SNES ROM ({len} bytes)")
            }
            Self::UnknownSize { len } => write!(
                f,
                "image size {len} matches no known SNES layout \
                 (expected 4 MiB headerless or 4 MiB + 512-byte copier header)"
            ),
            Self::UnknownRevision { sha256, attempted } => write!(
                f,
                "normalized image ({attempted}) matches no supported revision; \
                 sha256={}",
                bytes_to_hex(sha256)
            ),
        }
    }
}

impl std::error::Error for LoadError {}

fn bytes_to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0F) as usize] as char);
    }
    s
}

/// An in-memory, headerless, validated cartridge image.
#[derive(Debug, Clone)]
pub struct Rom {
    revision: Revision,
    image: Vec<u8>,
}

impl Rom {
    /// Number of bytes in a 512-byte copier header.
    pub const HEADER_SIZE: usize = 512;

    /// Expected normalized image size: 32 Mbit (4 MiB).
    pub const IMAGE_SIZE: usize = 4 * 1024 * 1024;

    /// Detects whether `image` begins with a 512-byte copier header by
    /// checking whether the SNES header checksum-complement field at
    /// `0xFFDC` of the candidate headerless image holds the one's complement
    /// of the checksum at `0xFFDE`.
    ///
    /// This is a structural test, not a filename convention.
    #[must_use]
    fn has_copier_header(image: &[u8]) -> bool {
        const CHECKSUM_FIELD: usize = 0xFFDC;
        if image.len() < Self::HEADER_SIZE + CHECKSUM_FIELD + 4 {
            return false;
        }
        let body = &image[Self::HEADER_SIZE..];
        let complement = u16::from_le_bytes([body[CHECKSUM_FIELD], body[CHECKSUM_FIELD + 1]]);
        let checksum = u16::from_le_bytes([body[CHECKSUM_FIELD + 2], body[CHECKSUM_FIELD + 3]]);
        complement == !checksum
    }

    /// Normalizes `image` in memory: returns the headerless slice and a
    /// description of the normalization performed. The input is never
    /// modified.
    #[must_use]
    fn normalize(image: &[u8]) -> (&[u8], &'static str) {
        if Self::has_copier_header(image) {
            (&image[Self::HEADER_SIZE..], "copier header stripped")
        } else {
            (image, "headerless")
        }
    }

    /// Loads and validates a cartridge image, normalizing any copier header
    /// in memory. Rejects unknown, truncated, and unrecognized images.
    ///
    /// # Errors
    ///
    /// Returns a [`LoadError`] describing why the image was rejected.
    pub fn load(image: &[u8]) -> Result<Self, LoadError> {
        use sha2::{Digest, Sha256};
        const CHECKSUM_FIELD: usize = 0xFFDC;
        if image.len() < Self::HEADER_SIZE + CHECKSUM_FIELD + 4 {
            return Err(LoadError::TooSmall { len: image.len() });
        }
        let (body, attempted) = Self::normalize(image);
        if body.len() != Self::IMAGE_SIZE {
            return Err(LoadError::UnknownSize { len: image.len() });
        }
        let digest: [u8; 32] = Sha256::digest(body).into();
        let revision = [Revision::Japan, Revision::EuropeEnglish]
            .into_iter()
            .find(|r| r.sha256() == digest)
            .ok_or(LoadError::UnknownRevision {
                sha256: digest,
                attempted,
            })?;
        Ok(Self {
            revision,
            image: body.to_vec(),
        })
    }

    /// The detected revision.
    #[must_use]
    pub fn revision(&self) -> Revision {
        self.revision
    }

    /// The normalized, headerless image bytes.
    #[must_use]
    pub fn image(&self) -> &[u8] {
        &self.image
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a synthetic image of `size` bytes with a plausible SNES
    /// internal header at `0xFFC0`, whose checksum fields are either
    /// consistent (used to fake "structural" header detection) or zero.
    fn synthetic_image(size: usize, valid_checksum: bool) -> Vec<u8> {
        let mut v = vec![0u8; size];
        // internal title
        v[0xFFC0..0xFFC0 + 4].copy_from_slice(b"SYNT");
        if valid_checksum {
            let checksum: u16 = 0x1234;
            v[0xFFDC..0xFFDE].copy_from_slice(&(!checksum).to_le_bytes());
            v[0xFFDE..0xFFE0].copy_from_slice(&checksum.to_le_bytes());
        }
        v
    }

    #[test]
    fn rejects_too_small() {
        let err = Rom::load(&[0u8; 64]).unwrap_err();
        assert_eq!(err, LoadError::TooSmall { len: 64 });
    }

    #[test]
    fn rejects_unknown_size() {
        // big enough to structurally inspect, wrong total size
        let img = synthetic_image(Rom::IMAGE_SIZE + 1024, false);
        let err = Rom::load(&img).unwrap_err();
        assert!(matches!(err, LoadError::UnknownSize { .. }));
    }

    #[test]
    fn header_detection_uses_structure_not_filename() {
        // A headerless image has its checksum fields at 0xFFDC/0xFFDE.
        let img = synthetic_image(Rom::IMAGE_SIZE, true);
        assert!(!Rom::has_copier_header(&img));

        // Prepend a copier header: the same fields now sit 512 bytes later,
        // and the bytes at 0xFFDC of the *whole* image are zeros, while the
        // structural test must find the valid pair at +512.
        let mut with_header = vec![0u8; 512];
        with_header.extend_from_slice(&img);
        assert!(Rom::has_copier_header(&with_header));
    }

    #[test]
    fn error_messages_are_actionable() {
        let msg = LoadError::UnknownRevision {
            sha256: [0xAB; 32],
            attempted: "copier header stripped",
        }
        .to_string();
        assert!(msg.contains("sha256="));
        assert!(msg.contains("abab"));
        assert!(msg.contains("copier"));
    }
}
