//! ROM normalization, validation, and addressing for Terranigma cartridge
//! dumps.
//!
//! This crate never reads or writes files. Callers own the bytes; everything
//! here operates on in-memory images. File-loading entry points belong to
//! tooling crates and must write ROM-derived output under the ignored
//! `local/` directory (see CONTRIBUTING.md).

mod address;

pub use address::{AddressError, CanonicalRomAddress, NormalizedOffset, RuntimeRomAddress};

/// A known, supported cartridge revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision {
    /// Tenchi Souzou, the original Japanese release.
    Japan,
    /// Terranigma, the European English release.
    EuropeEnglish,
}

impl Revision {
    /// Every supported revision.
    pub const ALL: [Self; 2] = [Self::Japan, Self::EuropeEnglish];

    /// Returns the stable lowercase identifier used for this revision.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Japan => "japan",
            Self::EuropeEnglish => "europe-english",
        }
    }

    /// The built-in known-ROM table used by [`Rom::load`].
    #[must_use]
    pub fn builtin_known() -> Vec<KnownRom> {
        Self::ALL
            .into_iter()
            .map(|r| KnownRom {
                revision: r,
                sha256: r.sha256(),
                crc32: r.crc32(),
            })
            .collect()
    }

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

    /// The internal title recorded at file offset `0xFFC0` of the normalized
    /// image, from the SNES cartridge header.
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

/// A recognized ROM: a revision plus the digests of its normalized image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KnownRom {
    /// The revision this entry recognizes.
    pub revision: Revision,
    /// Expected SHA-256 of the normalized image.
    pub sha256: [u8; 32],
    /// Expected CRC32 of the normalized image.
    pub crc32: u32,
}

/// Digests computed over a normalized image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Digests {
    /// SHA-256 of the normalized image.
    pub sha256: [u8; 32],
    /// CRC32 of the normalized image.
    pub crc32: u32,
}

/// Computes SHA-256 and CRC32 over a byte slice.
#[must_use]
pub fn digests(bytes: &[u8]) -> Digests {
    use sha2::{Digest, Sha256};
    Digests {
        sha256: Sha256::digest(bytes).into(),
        crc32: crc32fast::hash(bytes),
    }
}

/// How a cartridge image is laid out on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderLayout {
    /// A plain headerless image of exactly [`Rom::IMAGE_SIZE`] bytes.
    Headerless,
    /// A [`Rom::HEADER_SIZE`]-byte copier header followed by the image.
    CopierHeader,
}

/// Errors produced while loading a cartridge image.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LoadError {
    /// The image is smaller than a valid normalized image.
    TooSmall {
        /// Actual image length in bytes.
        len: usize,
    },
    /// The length matches no known image layout.
    UnknownSize {
        /// Actual image length in bytes.
        len: usize,
    },
    /// The image has a recognized layout but its normalized form matches no
    /// known revision.
    UnknownRevision {
        /// Computed SHA-256 of the normalized image.
        sha256: [u8; 32],
        /// Computed CRC32 of the normalized image.
        crc32: u32,
        /// Which normalization was applied.
        attempted: HeaderLayout,
    },
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooSmall { len } => {
                write!(f, "image is too small to be a valid ROM ({len} bytes)")
            }
            Self::UnknownSize { len } => write!(
                f,
                "image size {len} matches no known layout \
                 (expected {} bytes headerless or {} bytes with a copier header)",
                Rom::IMAGE_SIZE,
                Rom::IMAGE_SIZE + Rom::HEADER_SIZE
            ),
            Self::UnknownRevision {
                sha256,
                crc32,
                attempted,
            } => {
                let layout = match attempted {
                    HeaderLayout::Headerless => "headerless",
                    HeaderLayout::CopierHeader => "copier header stripped",
                };
                write!(
                    f,
                    "normalized image ({layout}) matches no supported revision; \
                     sha256={} crc32={crc32:08x}",
                    bytes_to_hex(sha256)
                )
            }
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

    /// Determines the layout of an image from its size.
    ///
    /// Size is the primary structural discriminator: the two supported
    /// layouts differ by exactly 512 bytes and are unambiguous. The SHA-256
    /// gate in [`Rom::load`] is the authority on whether the resulting
    /// normalized image is a supported revision.
    ///
    /// # Errors
    ///
    /// Returns a [`LoadError`] if the length matches no known layout.
    pub fn detect_layout(len: usize) -> Result<HeaderLayout, LoadError> {
        if len == Self::IMAGE_SIZE {
            Ok(HeaderLayout::Headerless)
        } else if len == Self::IMAGE_SIZE + Self::HEADER_SIZE {
            Ok(HeaderLayout::CopierHeader)
        } else if len < Self::IMAGE_SIZE {
            Err(LoadError::TooSmall { len })
        } else {
            Err(LoadError::UnknownSize { len })
        }
    }

    /// Returns the normalized (headerless) slice and the layout it was
    /// classified as. The input is never modified.
    ///
    /// # Errors
    ///
    /// Returns a [`LoadError`] if the image size matches no known layout.
    fn normalize(image: &[u8]) -> Result<(&[u8], HeaderLayout), LoadError> {
        match Self::detect_layout(image.len())? {
            HeaderLayout::Headerless => Ok((image, HeaderLayout::Headerless)),
            HeaderLayout::CopierHeader => {
                Ok((&image[Self::HEADER_SIZE..], HeaderLayout::CopierHeader))
            }
        }
    }

    /// Loads and validates a cartridge image against the built-in table of
    /// known revisions, normalizing any copier header in memory.
    ///
    /// # Errors
    ///
    /// Returns a [`LoadError`] describing why the image was rejected.
    pub fn load(image: &[u8]) -> Result<Self, LoadError> {
        Self::load_with_known(image, &Revision::builtin_known())
    }

    /// Loads and validates a cartridge image against a caller-supplied table
    /// of known ROMs. Useful for tests and for future revisions that are not
    /// part of the built-in table.
    ///
    /// # Errors
    ///
    /// Returns a [`LoadError`] describing why the image was rejected.
    pub fn load_with_known(image: &[u8], known: &[KnownRom]) -> Result<Self, LoadError> {
        let (body, layout) = Self::normalize(image)?;
        let d = digests(body);
        let revision = known
            .iter()
            .find(|k| k.sha256 == d.sha256 && k.crc32 == d.crc32)
            .map(|k| k.revision)
            .ok_or(LoadError::UnknownRevision {
                sha256: d.sha256,
                crc32: d.crc32,
                attempted: layout,
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

    /// Computes digests over this ROM's actual normalized image bytes.
    #[must_use]
    pub fn digests(&self) -> Digests {
        digests(&self.image)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A deterministic synthetic image of `size` bytes with a plausible SNES
    /// internal header at `0xFFC0`.
    fn synthetic_image(size: usize) -> Vec<u8> {
        let mut v = vec![0u8; size];
        for (i, b) in b"SYNTHETIC-TEST-ROM!".iter().enumerate() {
            v[i] = *b;
            v[size - 1 - i] = !*b;
        }
        v[0xFFC0..0xFFC0 + 4].copy_from_slice(b"SYNT");
        v
    }

    /// Builds a known-ROM table entry matching `image`'s actual digests, so
    /// the full success path can be exercised without any real dump.
    fn known_for(image: &[u8]) -> Vec<KnownRom> {
        vec![KnownRom {
            revision: Revision::Japan,
            sha256: digests(image).sha256,
            crc32: digests(image).crc32,
        }]
    }

    #[test]
    fn layout_detection_by_size() {
        assert_eq!(
            Rom::detect_layout(Rom::IMAGE_SIZE),
            Ok(HeaderLayout::Headerless)
        );
        assert_eq!(
            Rom::detect_layout(Rom::IMAGE_SIZE + Rom::HEADER_SIZE),
            Ok(HeaderLayout::CopierHeader)
        );
        assert_eq!(Rom::detect_layout(64), Err(LoadError::TooSmall { len: 64 }));
        assert_eq!(
            Rom::detect_layout(Rom::IMAGE_SIZE + 1),
            Err(LoadError::UnknownSize {
                len: Rom::IMAGE_SIZE + 1
            })
        );
        assert_eq!(
            Rom::detect_layout(Rom::IMAGE_SIZE + 1024),
            Err(LoadError::UnknownSize {
                len: Rom::IMAGE_SIZE + 1024
            })
        );
    }

    #[test]
    fn success_path_with_injected_digests_headerless() {
        let image = synthetic_image(Rom::IMAGE_SIZE);
        let rom = Rom::load_with_known(&image, &known_for(&image))
            .expect("synthetic image must load against its own digests");
        assert_eq!(rom.revision(), Revision::Japan);
        assert_eq!(rom.image().len(), Rom::IMAGE_SIZE);
        assert_eq!(rom.image(), image.as_slice());
    }

    #[test]
    fn success_path_with_injected_digests_copier_header() {
        let body = synthetic_image(Rom::IMAGE_SIZE);
        let mut with_header = vec![0xA5u8; Rom::HEADER_SIZE];
        with_header.extend_from_slice(&body);
        // Digests are computed over the normalized body, not the header.
        let rom = Rom::load_with_known(&with_header, &known_for(&body))
            .expect("headered synthetic image must load against body digests");
        assert_eq!(rom.image(), body.as_slice());
    }

    #[test]
    fn unknown_bytes_are_rejected_with_actionable_error() {
        let image = synthetic_image(Rom::IMAGE_SIZE);
        let err = Rom::load_with_known(&image, &known_for(&synthetic_image(Rom::IMAGE_SIZE + 1)))
            .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("sha256="), "message: {msg}");
        assert!(msg.contains("crc32="), "message: {msg}");
        assert!(msg.contains("headerless"), "message: {msg}");
    }

    #[test]
    fn headered_bytes_that_match_no_table_report_the_strip() {
        let body = synthetic_image(Rom::IMAGE_SIZE);
        let mut with_header = vec![0u8; Rom::HEADER_SIZE];
        with_header.extend_from_slice(&body);
        let err = Rom::load_with_known(&with_header, &[]).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("copier header stripped"), "message: {msg}");
    }

    #[test]
    fn builtin_table_covers_all_revisions_consistently() {
        for k in Revision::builtin_known() {
            assert_eq!(k.sha256, k.revision.sha256());
            assert_eq!(k.crc32, k.revision.crc32());
        }
    }
}
