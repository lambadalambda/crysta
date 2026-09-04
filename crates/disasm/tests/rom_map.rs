//! Integration tests for the revision-bound ROM classification map.

use disasm::{
    DataKind, DispatchResolveError, DispatchSource, EntryKind, PointerEncoding, RegionClass,
    RomMap, RomMapImageError, RomMapValidationError, ROM_MAP_SCHEMA_VERSION,
};
use rom::{CanonicalRomAddress, KnownRom, NormalizedOffset, Revision, Rom, RuntimeRomAddress};

const JAPAN_MAP: &str = include_str!("../data/rom-map/japan-v1.json");
const JAPAN_SHA256: &str = "f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548";

const SMALL_MAP: &str = r#"
{
  "schema_version": 1,
  "revision": "japan",
  "rom_sha256": "f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548",
  "sources": [{
    "id": "test_source",
    "title": "ROM-map generator test",
    "project_path": "crates/disasm/tests/rom_map.rs"
  }],
  "regions": [{
    "id": "test_code",
    "normalized": "$008000",
    "canonical": "$C08000",
    "length": 2,
    "class": {"type": "code"},
    "confidence": "static_corroborated",
    "evidence": [{
      "source_id": "test_source",
      "locator": "SMALL_MAP",
      "provenance": "static_analysis",
      "note": "Test-only executable region."
    }]
  }],
  "entry_points": [
    {
      "id": "zed_entry",
      "asm_name": "Zed",
      "normalized": "$008000",
      "canonical": "$C08000",
      "runtime": "$00:8000",
      "kind": "function",
      "confidence": "static_corroborated",
      "evidence": [{
        "source_id": "test_source",
        "locator": "SMALL_MAP zed_entry",
        "provenance": "static_analysis",
        "note": "Test-only entry."
      }]
    },
    {
      "id": "alpha_entry",
      "asm_name": "Alpha",
      "normalized": "$008001",
      "canonical": "$C08001",
      "runtime": "$80:8001",
      "kind": "internal",
      "confidence": "static_corroborated",
      "evidence": [{
        "source_id": "test_source",
        "locator": "SMALL_MAP alpha_entry",
        "provenance": "static_analysis",
        "note": "Test-only entry."
      }]
    }
  ],
  "indirect_dispatches": []
}
"#;

fn map_value() -> serde_json::Value {
    serde_json::from_str(JAPAN_MAP).unwrap()
}

fn region_index(value: &serde_json::Value, id: &str) -> usize {
    value["regions"]
        .as_array()
        .unwrap()
        .iter()
        .position(|region| region["id"] == id)
        .unwrap()
}

fn entry_index(value: &serde_json::Value, id: &str) -> usize {
    value["entry_points"]
        .as_array()
        .unwrap()
        .iter()
        .position(|entry| entry["id"] == id)
        .unwrap()
}

#[test]
fn built_in_map_is_revision_bound_and_sparse() {
    let map = RomMap::built_in_japan().expect("built-in ROM map must validate");

    assert_eq!(map.schema_version(), ROM_MAP_SCHEMA_VERSION);
    assert_eq!(map.revision(), Revision::Japan);
    assert_eq!(map.rom_sha256(), Revision::Japan.sha256());

    let reset = NormalizedOffset::new(0x0000_8000).unwrap();
    assert_eq!(map.region_at(reset).unwrap().class, RegionClass::Code);
    assert_eq!(
        map.region_at_canonical(CanonicalRomAddress::new(0xC0_8000).unwrap()),
        map.region_at(reset)
    );
    assert_eq!(
        map.region_at_runtime(RuntimeRomAddress::new(0x00_8000).unwrap()),
        map.region_at(reset)
    );
    assert!(map
        .region_at(NormalizedOffset::new(0x0001_0000).unwrap())
        .is_none());

    let vectors = map.region_by_id("native_and_emulation_vectors").unwrap();
    assert_eq!(vectors.class, RegionClass::Data(DataKind::VectorTable));
}

#[test]
fn entry_addresses_and_decoder_seeds_are_typed_and_consistent() {
    let map = RomMap::built_in_japan().unwrap();
    let reset = map.entry_by_id("reset").unwrap();

    assert_eq!(reset.kind, EntryKind::Function);
    assert_eq!(reset.normalized.value(), 0x0000_8000);
    assert_eq!(reset.canonical.value(), 0xC0_8000);
    assert_eq!(reset.runtime.value(), 0x00_8000);
    assert!(reset.decode_state.as_ref().unwrap().is_fully_known());
    assert!(map.decoder_seeds().all(|entry| {
        entry
            .decode_state
            .as_ref()
            .is_some_and(disasm::DecodeState::is_fully_known)
    }));
    assert!(map
        .entry_at_runtime(RuntimeRomAddress::new(0x80_8017).unwrap())
        .is_some_and(|entry| entry.id == "native_reset"));
}

#[test]
fn cop_dispatch_layout_and_declared_targets_resolve_from_image() {
    let map = RomMap::built_in_japan().unwrap();
    let dispatch = map.indirect_dispatch_by_id("cop_services").unwrap();
    let DispatchSource::RomTable {
        layout,
        pointer_encoding,
        ..
    } = &dispatch.source
    else {
        panic!("COP dispatch must be ROM-backed");
    };
    assert_eq!(layout.entry_count, 125);
    assert_eq!(layout.stride, 2);
    assert_eq!(layout.pointer_offset, 0);
    assert_eq!(
        *pointer_encoding,
        PointerEncoding::BankLocalU16 { runtime_bank: 0x80 }
    );

    let mut image = vec![0; Rom::IMAGE_SIZE];
    for target in &dispatch.targets {
        let runtime = map.entry_by_id(&target.entry_id).unwrap().runtime;
        let start = 0x83B2 + usize::try_from(target.index).unwrap() * 2;
        image[start..start + 2].copy_from_slice(&runtime.offset().to_le_bytes());
    }
    assert_eq!(
        rom::digests(&image[0x83B2..0x84AC]).sha256,
        [
            0xa2, 0x39, 0xb7, 0x04, 0x84, 0x81, 0x9b, 0x33, 0x4a, 0xea, 0xf4, 0xa9, 0xd8, 0x4d,
            0xc7, 0xaf, 0x03, 0x1b, 0x9d, 0x5a, 0x2f, 0xc6, 0x45, 0x20, 0x70, 0xab, 0x5a, 0xf0,
            0x0c, 0x4b, 0x23, 0xe0,
        ],
        "declared table bytes must match the authenticated Japanese ROM table digest"
    );
    let resolved = map
        .resolve_dispatch_from_image("cop_services", &image)
        .expect("declared table entries must resolve");
    assert_eq!(resolved.len(), 125);
    assert_eq!(resolved[0].index, 0);
    assert_eq!(resolved[0].normalized.value(), 0x0000_8592);
    assert_eq!(resolved[5].entry_id, "cop_service_05");
    assert_eq!(resolved[124].entry_id, "cop_service_7c");
}

#[test]
fn unknown_json_fields_are_rejected_at_every_level() {
    let mut value: serde_json::Value = serde_json::from_str(JAPAN_MAP).unwrap();
    value["regions"][0]["surprise"] = serde_json::json!(true);
    let error = RomMap::from_json(&value.to_string()).unwrap_err();
    assert!(error.to_string().contains("unknown field `surprise`"));
}

#[test]
fn unsupported_revisions_are_explicit() {
    let changed = JAPAN_MAP.replacen("\"japan\"", "\"europe-english\"", 1);
    let error = RomMap::from_json(&changed).unwrap_err();
    assert!(matches!(
        error.validation_error(),
        Some(RomMapValidationError::UnsupportedRevision { revision })
            if revision == "europe-english"
    ));
}

#[test]
fn inconsistent_region_addresses_are_rejected() {
    let changed = JAPAN_MAP.replacen("\"$C08000\"", "\"$C08001\"", 1);
    let error = RomMap::from_json(&changed).unwrap_err();
    assert!(matches!(
        error.validation_error(),
        Some(RomMapValidationError::RegionAddressMismatch { id }) if id == "boot_main"
    ));
}

#[test]
fn conflicting_claims_do_not_participate_in_canonical_lookup() {
    let mut value = map_value();
    value["conflicting_region_claims"] = serde_json::json!([{
        "region_id": "native_nmi",
        "normalized": "$05F990",
        "canonical": "$C5F990",
        "length": 376,
        "class": {"type": "code"},
        "source_id": "boot_assembly",
        "note": "A test-only overlapping alternative boundary."
    }]);
    let map = RomMap::from_json(&value.to_string()).unwrap();
    let claims = map.conflicting_claims_for("native_nmi").collect::<Vec<_>>();
    assert_eq!(claims.len(), 1);
    assert_eq!(claims[0].normalized.value(), 0x0005_F990);
    assert_eq!(
        map.region_at(NormalizedOffset::new(0x0005_F990).unwrap())
            .unwrap()
            .id,
        "native_nmi"
    );
}

#[test]
fn actual_rom_digest_is_checked_even_for_a_japan_tagged_rom() {
    let bytes = vec![0; Rom::IMAGE_SIZE];
    let digests = rom::digests(&bytes);
    let rom = Rom::load_with_known(
        &bytes,
        &[KnownRom {
            revision: Revision::Japan,
            sha256: digests.sha256,
            crc32: digests.crc32,
        }],
    )
    .unwrap();
    let error = RomMap::built_in_japan()
        .unwrap()
        .validate_rom(&rom)
        .unwrap_err();
    assert!(matches!(
        error,
        RomMapImageError::DigestMismatch { expected, actual }
            if expected == Revision::Japan.sha256() && actual == digests.sha256
    ));
}

#[test]
fn zero_length_overlap_and_entry_address_disagreement_are_rejected() {
    let mut value = map_value();
    value["regions"][0]["length"] = serde_json::json!(0);
    let error = RomMap::from_json(&value.to_string()).unwrap_err();
    assert!(matches!(
        error.validation_error(),
        Some(RomMapValidationError::InvalidRegionRange { id }) if id == "boot_main"
    ));

    let mut value = map_value();
    value["regions"][1]["normalized"] = serde_json::json!("$008050");
    value["regions"][1]["canonical"] = serde_json::json!("$C08050");
    let error = RomMap::from_json(&value.to_string()).unwrap_err();
    assert!(matches!(
        error.validation_error(),
        Some(RomMapValidationError::OverlappingRegions { .. })
    ));

    let mut value = map_value();
    let reset = entry_index(&value, "reset");
    value["entry_points"][reset]["runtime"] = serde_json::json!("$80:8001");
    let error = RomMap::from_json(&value.to_string()).unwrap_err();
    assert!(matches!(
        error.validation_error(),
        Some(RomMapValidationError::EntryAddressMismatch { id }) if id == "reset"
    ));
}

#[test]
fn evidence_confidence_and_source_kind_are_validated() {
    let mut value = map_value();
    value["regions"][0]["evidence"][0]["provenance"] = serde_json::json!("project_documentation");
    let error = RomMap::from_json(&value.to_string()).unwrap_err();
    assert!(matches!(
        error.validation_error(),
        Some(RomMapValidationError::ConfidenceEvidenceMismatch { owner_id })
            if owner_id == "boot_main"
    ));

    let mut value = map_value();
    value["regions"][0]["evidence"][0]["provenance"] = serde_json::json!("imported_claim");
    let error = RomMap::from_json(&value.to_string()).unwrap_err();
    assert!(matches!(
        error.validation_error(),
        Some(RomMapValidationError::ProvenanceSourceMismatch { owner_id, .. })
            if owner_id == "boot_main"
    ));
}

#[test]
fn table_pointer_fields_and_extent_are_validated_without_overflow() {
    let mut value = map_value();
    value["indirect_dispatches"][0]["source"]["layout"]["pointer_offset"] = serde_json::json!(1);
    let error = RomMap::from_json(&value.to_string()).unwrap_err();
    assert!(matches!(
        error.validation_error(),
        Some(RomMapValidationError::InvalidTableLayout { id }) if id == "cop_services"
    ));

    let mut value = map_value();
    value["indirect_dispatches"][0]["source"]["layout"]["entry_count"] =
        serde_json::json!(u32::MAX);
    let error = RomMap::from_json(&value.to_string()).unwrap_err();
    assert!(matches!(
        error.validation_error(),
        Some(RomMapValidationError::TableLayoutOutsideRegion { id })
            if id == "cop_services"
    ));
}

#[test]
fn runtime_and_normalized_u24_tables_resolve() {
    for (encoding, bank_byte) in [("runtime_u24", 0x80), ("normalized_u24", 0x00)] {
        let mut value = map_value();
        value["indirect_dispatches"][0]["source"]["layout"] =
            serde_json::json!({"entry_count": 6, "stride": 3, "pointer_offset": 0});
        value["indirect_dispatches"][0]["source"]["pointer_encoding"] =
            serde_json::json!({"type": encoding});
        value["indirect_dispatches"][0]["targets"]
            .as_array_mut()
            .unwrap()
            .truncate(6);
        let map = RomMap::from_json(&value.to_string()).unwrap();
        let mut image = vec![0; Rom::IMAGE_SIZE];
        let pointers = [0x8592_u16, 0x85B8, 0x85DF, 0x85F8, 0x8613, 0x862E];
        for (index, pointer) in pointers.into_iter().enumerate() {
            let start = 0x83B2 + index * 3;
            let [low, high] = pointer.to_le_bytes();
            image[start..start + 3].copy_from_slice(&[low, high, bank_byte]);
        }
        assert_eq!(
            map.resolve_dispatch_from_image("cop_services", &image)
                .unwrap()
                .len(),
            6
        );
    }
}

#[test]
fn truncated_and_disagreeing_table_images_are_rejected() {
    let map = RomMap::built_in_japan().unwrap();
    let truncated = vec![0; 0x83BE];
    let error = map
        .resolve_dispatch_from_image("cop_services", &truncated)
        .unwrap_err();
    assert!(matches!(error, DispatchResolveError::TruncatedImage { .. }));

    let image = vec![0; Rom::IMAGE_SIZE];
    let error = map
        .resolve_dispatch_from_image("cop_services", &image)
        .unwrap_err();
    assert!(matches!(
        error,
        DispatchResolveError::InvalidPointer { index: 0, .. }
    ));
}

#[test]
fn every_direct_boot_interrupt_and_main_loop_target_is_classified() {
    let map = RomMap::built_in_japan().unwrap();
    let targets = [
        0x80_8017, 0x80_818F, 0x80_820C, 0x80_8378, 0x80_E8AF, 0x85_F98F, 0x85_FB00, 0x85_FB01,
        0x85_FB08, 0x86_8000, 0x86_A505, 0x86_AA9C, 0x86_B8D6, 0x86_B9CB, 0x8D_86F8, 0x8D_8797,
        0x8D_9328,
    ];
    for address in targets {
        let runtime = RuntimeRomAddress::new(address).unwrap();
        let entry = map
            .entry_at_runtime(runtime)
            .unwrap_or_else(|| panic!("missing entry at {runtime}"));
        assert_eq!(
            map.region_at(entry.normalized).unwrap().class,
            RegionClass::Code,
            "{} must start in classified code",
            entry.id
        );
    }
}

#[test]
fn mutable_dispatch_targets_are_validated_but_not_image_resolved() {
    let map = RomMap::built_in_japan().unwrap();
    let dispatch = map.indirect_dispatch_by_id("top_level_state").unwrap();
    assert_eq!(dispatch.targets.len(), 1);
    let target = map.entry_by_id(&dispatch.targets[0].entry_id).unwrap();
    assert_eq!(target.id, "top_level_handler_805d");
    assert_eq!(target.runtime.value(), 0x80_805D);
    assert_eq!(
        map.region_at(target.normalized).unwrap().class,
        RegionClass::Code
    );

    let error = map
        .resolve_dispatch_from_image("top_level_state", &vec![0; Rom::IMAGE_SIZE])
        .unwrap_err();
    assert!(matches!(
        error,
        DispatchResolveError::MutableMemorySource { .. }
    ));

    let mut value = map_value();
    value["indirect_dispatches"][1]["targets"] =
        serde_json::json!([{"index": 0, "entry_id": "main_loop"}]);
    let map = RomMap::from_json(&value.to_string()).unwrap();
    let error = map
        .resolve_dispatch_from_image("top_level_state", &vec![0; Rom::IMAGE_SIZE])
        .unwrap_err();
    assert!(matches!(
        error,
        DispatchResolveError::MutableMemorySource { .. }
    ));

    value["indirect_dispatches"][1]["targets"][0]["entry_id"] = serde_json::json!("native_nmi");
    let error = RomMap::from_json(&value.to_string()).unwrap_err();
    assert!(matches!(
        error.validation_error(),
        Some(RomMapValidationError::DispatchTargetKindMismatch { .. })
    ));
}

#[test]
fn script_and_callback_dispatches_require_their_typed_data_regions() {
    let mut script = map_value();
    let service_region = region_index(&script, "cop_service_00_start");
    script["regions"][service_region]["class"] = serde_json::json!({"type": "data", "kind": "raw"});
    let service_entry = entry_index(&script, "cop_service_00");
    script["entry_points"][service_entry]["kind"] = serde_json::json!("script");
    script["indirect_dispatches"][0]["kind"] = serde_json::json!("script_table");
    script["indirect_dispatches"][0]["source"]["region_id"] =
        serde_json::json!("cop_dispatch_table");
    let table_region = region_index(&script, "cop_dispatch_table");
    script["regions"][table_region]["class"] =
        serde_json::json!({"type": "data", "kind": "script_entry_table"});
    script["indirect_dispatches"][0]["targets"] =
        serde_json::json!([{"index": 0, "entry_id": "cop_service_00"}]);
    RomMap::from_json(&script.to_string()).unwrap();

    let mut callbacks = map_value();
    callbacks["indirect_dispatches"][0]["kind"] = serde_json::json!("callback_records");
    let table_region = region_index(&callbacks, "cop_dispatch_table");
    callbacks["regions"][table_region]["class"] =
        serde_json::json!({"type": "data", "kind": "callback_records"});
    RomMap::from_json(&callbacks.to_string()).unwrap();
}

#[test]
fn impossible_emulation_decode_state_is_rejected() {
    let mut value = map_value();
    let reset = entry_index(&value, "reset");
    value["entry_points"][reset]["decode_state"]["m"] = serde_json::json!(false);
    let error = RomMap::from_json(&value.to_string()).unwrap_err();
    assert!(matches!(
        error.validation_error(),
        Some(RomMapValidationError::InvalidDecodeState { id }) if id == "reset"
    ));
}

#[test]
fn bank_local_dispatch_requires_the_declared_runtime_mirror() {
    let mut value = map_value();
    let service = entry_index(&value, "cop_service_00");
    value["entry_points"][service]["runtime"] = serde_json::json!("$C0:8592");
    let error = RomMap::from_json(&value.to_string()).unwrap_err();
    assert!(matches!(
        error.validation_error(),
        Some(RomMapValidationError::PointerCannotRepresentTarget { entry_id, .. })
            if entry_id == "cop_service_00"
    ));
}

#[test]
fn mutable_pointer_sources_must_not_be_rom_addresses() {
    let mut value = map_value();
    value["indirect_dispatches"][1]["source"]["address"] = serde_json::json!("$C08000");
    let error = RomMap::from_json(&value.to_string()).unwrap_err();
    assert!(error
        .to_string()
        .contains("mutable address lies in a ROM-backed window"));

    let mut value = map_value();
    value["indirect_dispatches"][1]["source"]["address"] = serde_json::json!("$3F7FFF");
    let error = RomMap::from_json(&value.to_string()).unwrap_err();
    assert!(matches!(
        error.validation_error(),
        Some(RomMapValidationError::InvalidMutablePointerSource { .. })
    ));
}

#[test]
fn unknown_pointer_encoding_fields_are_rejected() {
    let mut value = map_value();
    value["indirect_dispatches"][0]["source"]["pointer_encoding"]["surprise"] =
        serde_json::json!(true);
    let error = RomMap::from_json(&value.to_string()).unwrap_err();
    assert!(error.to_string().contains("unknown field `surprise`"));
}

#[test]
fn runtime_u24_resolution_preserves_and_checks_the_runtime_mirror() {
    let mut value = map_value();
    value["indirect_dispatches"][0]["source"]["layout"] =
        serde_json::json!({"entry_count": 1, "stride": 3, "pointer_offset": 0});
    value["indirect_dispatches"][0]["source"]["pointer_encoding"] =
        serde_json::json!({"type": "runtime_u24"});
    value["indirect_dispatches"][0]["targets"] =
        serde_json::json!([{"index": 0, "entry_id": "cop_service_00"}]);
    let map = RomMap::from_json(&value.to_string()).unwrap();
    let mut image = vec![0; Rom::IMAGE_SIZE];
    image[0x83B2..0x83B5].copy_from_slice(&[0x92, 0x85, 0xC0]);
    let error = map
        .resolve_dispatch_from_image("cop_services", &image)
        .unwrap_err();
    assert!(matches!(
        error,
        DispatchResolveError::RuntimeTargetMismatch { .. }
    ));
}

#[test]
fn ca65_include_has_exact_stable_sorted_output() {
    let map = RomMap::from_json(SMALL_MAP).unwrap();
    let expected = format!(
        "; Generated ROM map constants.\n\
; Schema version: 1\n\
; Revision: japan\n\
; ROM SHA-256: {JAPAN_SHA256}\n\
\n\
AlphaCanonical = $C08001\n\
AlphaRuntime = $808001\n\
ZedCanonical = $C08000\n\
ZedRuntime = $008000\n"
    );

    assert_eq!(map.generate_ca65_include(), expected);
    assert_eq!(map.generate_ca65_include(), map.generate_ca65_include());
}

#[test]
fn built_in_ca65_include_contains_distinct_canonical_and_runtime_constants() {
    let map = RomMap::built_in_japan().unwrap();
    let include = map.generate_ca65_include();

    assert!(include.starts_with(&format!(
        "; Generated ROM map constants.\n; Schema version: 1\n; Revision: japan\n; ROM SHA-256: {JAPAN_SHA256}\n\n"
    )));
    for constant in [
        "NativeNmiHandlerCanonical = $C5F98F\n",
        "NativeNmiHandlerRuntime = $85F98F\n",
        "NativeResetCanonical = $C08017\n",
        "NativeResetRuntime = $808017\n",
        "WaitForFrameAndPollInputCanonical = $C68000\n",
        "WaitForFrameAndPollInputRuntime = $868000\n",
    ] {
        assert!(include.contains(constant), "missing {constant:?}");
    }
    let body = include.split_once("\n\n").unwrap().1;
    let lines = body.lines().collect::<Vec<_>>();
    assert!(lines.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(!include.contains(" = $85F98F\nNativeNmiHandler ="));
}

#[test]
fn generated_ca65_names_must_not_collide_with_entries_or_each_other() {
    let mut value: serde_json::Value = serde_json::from_str(SMALL_MAP).unwrap();
    value["entry_points"][1]["asm_name"] = serde_json::json!("Zed");
    let error = RomMap::from_json(&value.to_string()).unwrap_err();
    assert!(matches!(
        error.validation_error(),
        Some(RomMapValidationError::GeneratedNameCollision { name, .. })
            if name == "ZedCanonical"
    ));

    let mut value: serde_json::Value = serde_json::from_str(SMALL_MAP).unwrap();
    value["entry_points"][1]["asm_name"] = serde_json::json!("ZedCanonical");
    let error = RomMap::from_json(&value.to_string()).unwrap_err();
    assert!(matches!(
        error.validation_error(),
        Some(RomMapValidationError::GeneratedNameCollision { name, .. })
            if name == "ZedCanonical"
    ));
}
