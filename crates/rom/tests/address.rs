//! Public behavior tests for typed Terranigma ROM addresses and identities.

use rom::{AddressError, CanonicalRomAddress, NormalizedOffset, Revision, Rom, RuntimeRomAddress};

#[test]
fn normalized_and_canonical_bounds_and_round_trips() {
    let zero = NormalizedOffset::new(0).unwrap();
    let max = NormalizedOffset::new(0x3f_ffff).unwrap();
    assert_eq!(zero.value(), 0);
    assert_eq!(max.value(), 0x3f_ffff);
    assert!(NormalizedOffset::new(0x40_0000).is_err());

    let first = CanonicalRomAddress::new(0xc0_0000).unwrap();
    let last = CanonicalRomAddress::new(0xff_ffff).unwrap();
    assert_eq!(first.value(), 0xc0_0000);
    assert_eq!(last.value(), 0xff_ffff);
    assert!(CanonicalRomAddress::new(0xbf_ffff).is_err());

    assert_eq!(CanonicalRomAddress::from(zero), first);
    assert_eq!(CanonicalRomAddress::from(max), last);
    assert_eq!(NormalizedOffset::from(first), zero);
    assert_eq!(NormalizedOffset::from(last), max);
    assert_eq!(zero.to_string(), "$000000");
    assert_eq!(last.to_string(), "$FFFFFF");
}

#[test]
fn runtime_windows_cover_all_hirom_mirror_classes() {
    let cases = [
        (0x00_8000, 0x00_8000),
        (0x85_f98f, 0x05_f98f),
        (0x40_0000, 0x00_0000),
        (0xc0_0000, 0x00_0000),
        (0xff_ffff, 0x3f_ffff),
    ];

    for (runtime, normalized) in cases {
        let runtime = RuntimeRomAddress::new(runtime).unwrap();
        let normalized = NormalizedOffset::new(normalized).unwrap();
        assert_eq!(runtime.normalized(), normalized);
        assert_eq!(runtime.canonical(), normalized.canonical());
    }

    assert_eq!(RuntimeRomAddress::new(0x85_f98f).unwrap().bank(), 0x85);
    assert_eq!(RuntimeRomAddress::new(0x85_f98f).unwrap().offset(), 0xf98f);
    assert_eq!(
        RuntimeRomAddress::new(0x85_f98f).unwrap().to_string(),
        "$85:F98F"
    );
}

#[test]
fn invalid_runtime_windows_are_rejected() {
    assert!(matches!(
        RuntimeRomAddress::new(0x00_7fff),
        Err(AddressError::UnmappedRuntimeAddress { .. })
    ));
    assert!(RuntimeRomAddress::new(0x80_0000).is_err());
    assert!(RuntimeRomAddress::new(0x7e_8000).is_err());
    assert!(RuntimeRomAddress::new(0x7f_ffff).is_err());
    assert!(matches!(
        RuntimeRomAddress::new(0x100_0000),
        Err(AddressError::RuntimeAddressOutOfRange { .. })
    ));
}

#[test]
fn canonical_runtime_conversion_round_trips() {
    for raw in [0, 0x8000, 0x05_f98f, 0x3f_ffff] {
        let normalized = NormalizedOffset::new(raw).unwrap();
        let canonical = normalized.canonical();
        let runtime = RuntimeRomAddress::from(canonical);
        assert_eq!(runtime.normalized(), normalized);
        assert_eq!(runtime.canonical(), canonical);
        assert_eq!(RuntimeRomAddress::from(normalized), runtime);
    }
}

#[test]
fn revision_ids_are_stable_and_lowercase() {
    assert_eq!(Revision::Japan.id(), "japan");
    assert_eq!(Revision::EuropeEnglish.id(), "europe-english");
}

#[test]
fn rom_digests_describe_actual_normalized_bytes() {
    let body = vec![0x5a; Rom::IMAGE_SIZE];
    let expected = rom::digests(&body);
    let known = [rom::KnownRom {
        revision: Revision::Japan,
        sha256: expected.sha256,
        crc32: expected.crc32,
    }];
    let mut input = vec![0xa5; Rom::HEADER_SIZE];
    input.extend_from_slice(&body);

    let loaded = Rom::load_with_known(&input, &known).unwrap();
    assert_eq!(loaded.digests(), expected);
    assert_eq!(loaded.digests(), rom::digests(loaded.image()));
}
