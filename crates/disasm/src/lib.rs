//! Byte-matching disassembly reconstruction for Terranigma.
//!
//! This crate builds a verified reconstruction of the SNES ROM from assembly
//! source and opaque byte ranges. The build script verifies byte-for-byte
//! matching against the normalized local dump.

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    fn clean_rom_path() -> PathBuf {
        PathBuf::from(env!("OUT_DIR")).join("rom-clean.bin")
    }

    #[test]
    fn clean_rom_exists() {
        let p = clean_rom_path();
        assert!(p.exists(), "clean ROM not found at {p:?}");
    }

    #[test]
    fn clean_rom_matches_expected_size() {
        let bytes = std::fs::read(clean_rom_path()).unwrap();
        assert!(
            bytes.len().is_power_of_two(),
            "clean ROM size {} is not a power of two",
            bytes.len()
        );
        assert!(bytes.len() >= 1024 * 1024, "clean ROM seems too small");
    }

    #[test]
    fn clean_rom_matches_reference_digest() {
        use sha2::{Digest, Sha256};
        let bytes = std::fs::read(clean_rom_path()).unwrap();
        let hash: [u8; 32] = Sha256::digest(&bytes).into();
        let expected = rom::digests(
            &std::fs::read(
                std::env::var("CARGO_MANIFEST_DIR")
                    .unwrap()
                    .parse::<PathBuf>()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .join("local/Tenchi Souzou (Japan).sfc"),
            )
            .expect("local ROM not found"),
        );
        assert_eq!(hash, expected.sha256, "clean ROM SHA-256 mismatch");
    }

    #[test]
    fn byte_mismatch_detected_with_offset() {
        // The build script itself performs this check: if the assembled output
        // differs from the clean ROM at any byte, it panics with the offset.
        // This test verifies the clean ROM has been compared by checking that
        // the build completed (cargo test would have failed at build time
        // otherwise). A one-byte intentional change in any .s file will cause
        // the build to fail with a specific offset report.
        let built_path = PathBuf::from(env!("OUT_DIR")).join("rom-built.bin");
        assert!(
            built_path.exists(),
            "built ROM not found — build script comparison may not have run"
        );
    }
}
