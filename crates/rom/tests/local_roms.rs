//! Optional integration tests against user-provided cartridge dumps.
//!
//! These tests skip unless one or both supported dumps are present under
//! `local/` (git-ignored). They never modify the source files and are excluded
//! from public CI, which runs without ROMs.

use rom::{LoadError, Revision, Rom};

/// Reads `local/<name>` if present.
fn local_rom(name: &str) -> Option<Vec<u8>> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../local")
        .join(name);
    std::fs::read(path).ok()
}

#[test]
fn japan_dump_loads() {
    let Some(image) = local_rom("Tenchi Souzou (Japan).sfc") else {
        eprintln!("skipping: local Japanese dump not present");
        return;
    };
    let rom = Rom::load(&image).expect("known-good Japanese dump must load");
    assert_eq!(rom.revision(), Revision::Japan);
    assert_eq!(rom.image().len(), Rom::IMAGE_SIZE);
    assert_eq!(Revision::of_image(rom.image()), Some(Revision::Japan));
}

#[test]
fn europe_dump_normalizes_and_loads() {
    let Some(image) = local_rom("Terranigma (E) [!].smc") else {
        eprintln!("skipping: local European dump not present");
        return;
    };
    // This particular dump carries a 512-byte copier header; the loader must
    // normalize it in memory and still recognize the revision.
    let rom = Rom::load(&image).expect("known-good European dump must load");
    assert_eq!(rom.revision(), Revision::EuropeEnglish);
    assert_eq!(rom.image().len(), Rom::IMAGE_SIZE);
    assert_eq!(
        Revision::of_image(rom.image()),
        Some(Revision::EuropeEnglish)
    );
    // The normalized image must expose the internal title at 0xFFC0.
    let title = &rom.image()[0xFFC0..0xFFC0 + 12];
    assert_eq!(
        &title[..Revision::EuropeEnglish.internal_title().len()],
        Revision::EuropeEnglish.internal_title().as_bytes()
    );
}

#[test]
fn corrupted_image_is_rejected_with_hash() {
    let Some(image) = local_rom("Tenchi Souzou (Japan).sfc") else {
        eprintln!("skipping: local Japanese dump not present");
        return;
    };
    let mut corrupted = image.clone();
    corrupted[0x100] ^= 0xFF;
    let err = Rom::load(&corrupted).unwrap_err();
    match err {
        LoadError::UnknownRevision {
            sha256,
            crc32,
            attempted,
        } => {
            assert_ne!(sha256, Revision::Japan.sha256());
            assert_ne!(crc32, Revision::Japan.crc32());
            assert_eq!(attempted, rom::HeaderLayout::Headerless);
        }
        other => panic!("expected UnknownRevision, got {other:?}"),
    }
}
