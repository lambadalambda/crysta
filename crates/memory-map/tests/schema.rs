//! Integration coverage for the canonical map and its public APIs.

use std::fmt::Write as _;

use memory_map::{
    Address, AddressSpace, Alias, AliasStatus, Confidence, LoadError, MemoryMap, ReadError,
    ValidationError, JAPAN_ROM_SHA256, SCHEMA_VERSION,
};

fn builtin() -> MemoryMap {
    MemoryMap::built_in_japan().expect("built-in map must load")
}

#[test]
fn map_hash_matches_the_rom_crate_japanese_revision() {
    let expected = rom::Revision::Japan
        .sha256()
        .iter()
        .fold(String::new(), |mut hex, byte| {
            write!(hex, "{byte:02x}").expect("writing to String cannot fail");
            hex
        });
    let map = builtin();
    assert_eq!(map.rom_sha256, expected);
    assert!(map.sources.iter().any(|source| {
        source.id == "project-rom-library"
            && source.project_path.as_deref() == Some("crates/rom/src/lib.rs")
    }));
}

#[test]
fn built_in_map_has_expected_identity_and_gameplay_symbols() {
    let map = builtin();
    assert_eq!(map.schema_version, SCHEMA_VERSION);
    assert_eq!(map.revision, "japan");
    assert_eq!(map.rom_sha256, JAPAN_ROM_SHA256);

    for id in [
        "program_state",
        "current_map",
        "pending_map",
        "room_change_timer",
        "state_handler_pointer",
        "name_entry_cursor_row",
        "menu_cursor",
        "text_speed",
        "player_x",
        "player_y",
        "event_flags",
        "cgram_staging_buffer",
        "inventory_items",
        "inventory_weapons",
        "inventory_armor",
        "inventory_magic",
    ] {
        assert!(map.lookup_id(id).is_some(), "missing {id}");
    }

    assert!(map
        .sources
        .iter()
        .any(|source| source.id == "datacrystal-53316"));
    assert!(map
        .sources
        .iter()
        .any(|source| source.id == "gamefaqs-78151"));
}

fn assert_invalid(mutate: impl FnOnce(&mut MemoryMap), predicate: fn(&ValidationError) -> bool) {
    let mut map = builtin();
    mutate(&mut map);
    let error = map.validate().expect_err("invalid map accepted");
    assert!(predicate(&error), "unexpected validation error: {error:?}");
}

#[test]
fn validation_rejects_identity_and_duplicate_ids() {
    assert_invalid(
        |map| map.schema_version = 2,
        |error| matches!(error, ValidationError::UnsupportedSchema { .. }),
    );
    assert_invalid(
        |map| map.revision = "europe".into(),
        |error| matches!(error, ValidationError::UnsupportedRevision { .. }),
    );
    assert_invalid(
        |map| map.rom_sha256 = "00".repeat(32),
        |error| matches!(error, ValidationError::UnexpectedRomHash { .. }),
    );
    assert_invalid(
        |map| map.symbols[1].id = map.symbols[0].id.clone(),
        |error| matches!(error, ValidationError::DuplicateSymbolId { .. }),
    );
    assert_invalid(
        |map| map.symbols[1].asm_name = map.symbols[0].asm_name.clone(),
        |error| matches!(error, ValidationError::DuplicateAsmName { .. }),
    );
    assert_invalid(
        |map| map.sources[1].id = map.sources[0].id.clone(),
        |error| matches!(error, ValidationError::DuplicateSourceId { .. }),
    );
}

#[test]
fn source_records_are_validated() {
    assert_invalid(
        |map| map.sources[0].retrieved_on = Some("2026-9-04".into()),
        |error| matches!(error, ValidationError::InvalidSource { .. }),
    );
    assert_invalid(
        |map| map.sources[0].id.clear(),
        |error| matches!(error, ValidationError::InvalidSource { .. }),
    );
    assert_invalid(
        |map| map.sources[0].title.clear(),
        |error| matches!(error, ValidationError::InvalidSource { .. }),
    );
    assert_invalid(
        |map| map.sources[0].url = Some(String::new()),
        |error| matches!(error, ValidationError::InvalidSource { .. }),
    );
    assert_invalid(
        |map| map.sources[0].url = Some("not a URL".into()),
        |error| matches!(error, ValidationError::InvalidSource { .. }),
    );
    assert_invalid(
        |map| map.sources[0].retrieved_on = Some("2026-99-99".into()),
        |error| matches!(error, ValidationError::InvalidSource { .. }),
    );
    assert_invalid(
        |map| {
            map.sources
                .iter_mut()
                .find(|source| source.id == "project-rom-library")
                .unwrap()
                .project_path = Some(String::new());
        },
        |error| matches!(error, ValidationError::InvalidSource { .. }),
    );
    assert_invalid(
        |map| {
            map.sources
                .iter_mut()
                .find(|source| source.id == "project-rom-library")
                .unwrap()
                .project_path = Some("../../outside".into());
        },
        |error| matches!(error, ValidationError::InvalidSource { .. }),
    );
}

#[test]
fn evidence_alias_text_and_provenance_are_validated() {
    assert_invalid(
        |map| map.symbols[0].confidence = Confidence::ImportedClaim,
        |error| matches!(error, ValidationError::ConfidenceEvidenceMismatch { .. }),
    );
    assert_invalid(
        |map| map.symbols[0].id.clear(),
        |error| matches!(error, ValidationError::InvalidSymbolId),
    );
    assert_invalid(
        |map| map.symbols[0].evidence[0].locator.clear(),
        |error| matches!(error, ValidationError::InvalidEvidence { .. }),
    );
    assert_invalid(
        |map| map.symbols[0].evidence[0].source_id.clear(),
        |error| matches!(error, ValidationError::InvalidEvidence { .. }),
    );
    assert_invalid(
        |map| map.symbols[0].evidence[0].note.clear(),
        |error| matches!(error, ValidationError::InvalidEvidence { .. }),
    );
    assert_invalid(
        |map| map.symbols[0].evidence[0].provenance = memory_map::Provenance::ImportedClaim,
        |error| matches!(error, ValidationError::ProvenanceSourceMismatch { .. }),
    );
    assert_invalid(
        |map| {
            let symbol = map
                .symbols
                .iter_mut()
                .find(|symbol| symbol.id == "event_flags")
                .unwrap();
            symbol.evidence[0].provenance = memory_map::Provenance::ProjectTrace;
        },
        |error| matches!(error, ValidationError::ProvenanceSourceMismatch { .. }),
    );
    assert_invalid(
        |map| {
            map.symbols
                .iter_mut()
                .find(|symbol| symbol.id == "map_x_scroll")
                .unwrap()
                .aliases[0]
                .name
                .clear();
        },
        |error| matches!(error, ValidationError::InvalidAlias { .. }),
    );
    assert_invalid(
        |map| {
            map.symbols
                .iter_mut()
                .find(|symbol| symbol.id == "map_x_scroll")
                .unwrap()
                .aliases[0]
                .note
                .clear();
        },
        |error| matches!(error, ValidationError::InvalidAlias { .. }),
    );
}

#[test]
fn validation_rejects_bad_regions_evidence_operands_and_overlaps() {
    assert_invalid(
        |map| map.symbols[0].asm_name = "BAD NAME".into(),
        |error| matches!(error, ValidationError::InvalidAsmName { .. }),
    );
    assert_invalid(
        |map| map.symbols[0].evidence.clear(),
        |error| matches!(error, ValidationError::MissingEvidence { .. }),
    );
    assert_invalid(
        |map| map.symbols[0].width = 0,
        |error| matches!(error, ValidationError::ZeroWidth { .. }),
    );
    assert_invalid(
        |map| map.symbols[0].address = Address::new(0x01_0000),
        |error| matches!(error, ValidationError::AddressOutOfRange { .. }),
    );
    assert_invalid(
        |map| map.symbols[0].evidence[0].source_id = "missing".into(),
        |error| matches!(error, ValidationError::UnknownEvidenceSource { .. }),
    );
    assert_invalid(
        |map| map.symbols[0].assembly_operand = Some(Address::new(0x100_0000)),
        |error| matches!(error, ValidationError::AssemblyOperandOutOfRange { .. }),
    );
    assert_invalid(
        |map| map.symbols[0].assembly_operand = Some(Address::new(0x12_3456)),
        |error| matches!(error, ValidationError::AssemblyOperandMismatch { .. }),
    );
    assert_invalid(
        |map| map.symbols[0].confidence = Confidence::TraceCorroborated,
        |error| matches!(error, ValidationError::ConfidenceEvidenceMismatch { .. }),
    );
    assert_invalid(
        |map| {
            map.symbols[1].space = map.symbols[0].space;
            map.symbols[1].address = map.symbols[0].address;
            map.symbols[1].width = map.symbols[0].width;
            map.symbols[1].assembly_operand = map.symbols[0].assembly_operand;
        },
        |error| matches!(error, ValidationError::UndocumentedOverlap { .. }),
    );
}

#[test]
fn short_low_wram_operand_requires_the_whole_region_in_the_mirror() {
    let mut map = builtin();
    let symbol = map
        .symbols
        .iter_mut()
        .find(|symbol| symbol.id == "menu_cursor")
        .unwrap();
    symbol.address = Address::new(0x7E_1FFE);
    symbol.width = 2;
    symbol.assembly_operand = Some(Address::new(0x1FFE));
    map.validate()
        .expect("region ending at $7E1FFF is mirrored");

    let symbol = map
        .symbols
        .iter_mut()
        .find(|symbol| symbol.id == "menu_cursor")
        .unwrap();
    symbol.address = Address::new(0x7E_1FFF);
    symbol.assembly_operand = Some(Address::new(0x1FFF));
    assert!(matches!(
        map.validate(),
        Err(ValidationError::AssemblyOperandMismatch { .. })
    ));
}

fn relate(map: &mut MemoryMap, from: &str, to: &str, status: AliasStatus) {
    map.symbols
        .iter_mut()
        .find(|symbol| symbol.id == from)
        .unwrap()
        .aliases
        .push(Alias {
            name: format!("claim related to {to}"),
            status,
            related_symbol_id: Some(to.into()),
            note: "test relationship".into(),
        });
}

fn direct_page_overlap() -> MemoryMap {
    let mut map = builtin();
    let symbol = map
        .symbols
        .iter_mut()
        .find(|symbol| symbol.id == "program_state")
        .unwrap();
    symbol.address = Address::new(0x7E_0036);
    symbol.width = 2;
    symbol.assembly_operand = Some(Address::new(0x0036));
    map
}

#[test]
fn direct_page_and_low_wram_are_the_same_physical_memory() {
    assert!(matches!(
        direct_page_overlap().validate(),
        Err(ValidationError::UndocumentedOverlap { .. })
    ));

    let mut map = direct_page_overlap();
    relate(
        &mut map,
        "cop_return_address",
        "program_state",
        AliasStatus::Conflicting,
    );
    relate(
        &mut map,
        "program_state",
        "cop_return_address",
        AliasStatus::Conflicting,
    );
    map.validate()
        .expect("reciprocal conflicting aliases permit exact overlap");
}

#[test]
fn overlap_requires_reciprocal_conflicting_aliases_and_exact_regions() {
    for status in [
        AliasStatus::Confirmed,
        AliasStatus::Imported,
        AliasStatus::Rejected,
    ] {
        let mut map = direct_page_overlap();
        relate(&mut map, "cop_return_address", "program_state", status);
        relate(&mut map, "program_state", "cop_return_address", status);
        assert!(matches!(
            map.validate(),
            Err(ValidationError::UndocumentedOverlap { .. })
        ));
    }

    let mut unilateral = direct_page_overlap();
    relate(
        &mut unilateral,
        "cop_return_address",
        "program_state",
        AliasStatus::Conflicting,
    );
    assert!(matches!(
        unilateral.validate(),
        Err(ValidationError::UndocumentedOverlap { .. })
    ));

    let mut partial = direct_page_overlap();
    let symbol = partial
        .symbols
        .iter_mut()
        .find(|symbol| symbol.id == "program_state")
        .unwrap();
    symbol.address = Address::new(0x7E_0037);
    symbol.width = 1;
    symbol.assembly_operand = Some(Address::new(0x0037));
    relate(
        &mut partial,
        "cop_return_address",
        "program_state",
        AliasStatus::Conflicting,
    );
    relate(
        &mut partial,
        "program_state",
        "cop_return_address",
        AliasStatus::Conflicting,
    );
    assert!(matches!(
        partial.validate(),
        Err(ValidationError::UndocumentedOverlap { .. })
    ));
}

#[test]
fn typed_conflicting_boundary_claims_are_queryable() {
    let map = builtin();
    let armor = map.lookup_id("inventory_armor").unwrap();
    assert_eq!(armor.address, Address::new(0x7F_8068));
    assert_eq!(armor.confidence, Confidence::ImportedClaim);

    let claims = map.lookup_conflicting_claims("inventory_armor");
    assert_eq!(claims.len(), 1);
    assert_eq!(claims[0].claimed_space, AddressSpace::Wram);
    assert_eq!(claims[0].claimed_address, Address::new(0x7F_8066));
    assert_eq!(claims[0].claimed_width, 0x1A);
    assert_eq!(claims[0].source_id, "datacrystal-53316");
}

#[test]
fn conflicting_claims_validate_targets_sources_text_and_ranges() {
    assert_invalid(
        |map| map.conflicting_claims[0].symbol_id = "missing".into(),
        |error| matches!(error, ValidationError::UnknownClaimSymbol { .. }),
    );
    assert_invalid(
        |map| map.conflicting_claims[0].source_id = "missing".into(),
        |error| matches!(error, ValidationError::UnknownClaimSource { .. }),
    );
    assert_invalid(
        |map| map.conflicting_claims[0].note.clear(),
        |error| matches!(error, ValidationError::InvalidConflictingClaim { .. }),
    );
    assert_invalid(
        |map| map.conflicting_claims[0].claimed_width = 0,
        |error| matches!(error, ValidationError::InvalidConflictingClaim { .. }),
    );
    assert_invalid(
        |map| {
            map.conflicting_claims[0].symbol_id = "cop_return_address".into();
            map.conflicting_claims[0].claimed_space = AddressSpace::Wram;
            map.conflicting_claims[0].claimed_address = Address::new(0x7E_0036);
            map.conflicting_claims[0].claimed_width = 2;
        },
        |error| matches!(error, ValidationError::NonConflictingClaim { .. }),
    );
    assert_invalid(
        |map| {
            let symbol = map.lookup_id("inventory_armor").unwrap();
            let (space, address, width) = (symbol.space, symbol.address, symbol.width);
            map.conflicting_claims[0].claimed_space = space;
            map.conflicting_claims[0].claimed_address = address;
            map.conflicting_claims[0].claimed_width = width;
        },
        |error| matches!(error, ValidationError::NonConflictingClaim { .. }),
    );
}

#[test]
fn conflicting_aliases_are_retained_and_allow_documented_exact_overlap() {
    let map = builtin();
    let matches = map.lookup_address(Address::new(0x7E_081E));
    assert_eq!(matches.len(), 2);
    assert_ne!(matches[0].id, matches[1].id);
    assert!(matches
        .iter()
        .any(|symbol| symbol.aliases.iter().any(|alias| {
            alias.status == AliasStatus::Conflicting && alias.related_symbol_id.is_some()
        })));
}

#[test]
fn reads_direct_page_and_wram_using_canonical_offsets() {
    let map = builtin();
    let mut wram = vec![0_u8; 0x20_000];
    wram[0x36..0x38].copy_from_slice(&[0x34, 0x12]);
    wram[0x1000..0x1002].copy_from_slice(&[0x30, 0x01]);

    assert_eq!(
        map.read_symbol(map.lookup_id("cop_return_address").unwrap(), &wram)
            .unwrap(),
        &[0x34, 0x12]
    );
    assert_eq!(
        map.read_symbol(map.lookup_id("player_x").unwrap(), &wram)
            .unwrap(),
        &[0x30, 0x01]
    );
}

fn with_sram_region(address: u32, width: u32) -> MemoryMap {
    let mut map = builtin();
    let symbol = map
        .symbols
        .iter_mut()
        .find(|symbol| symbol.id == "menu_cursor")
        .unwrap();
    symbol.space = AddressSpace::Sram;
    symbol.address = Address::new(address);
    symbol.width = width;
    symbol.assembly_operand = Some(Address::new(address));
    map
}

#[test]
fn sram_regions_are_bank_local_hirom_windows() {
    with_sram_region(0x20_6000, 1)
        .validate()
        .expect("first canonical SRAM byte");
    with_sram_region(0x3F_7FFF, 1)
        .validate()
        .expect("last canonical SRAM byte");

    for (address, width) in [
        (0x1F_6000, 1),
        (0x40_6000, 1),
        (0x20_5FFF, 1),
        (0x20_8000, 1),
        (0x20_7FFF, 2),
    ] {
        assert!(matches!(
            with_sram_region(address, width).validate(),
            Err(ValidationError::AddressOutOfRange { .. })
        ));
    }
}

#[test]
fn reads_reject_unsupported_spaces_and_short_wram() {
    let map = builtin();
    let hardware = map.lookup_id("cgram_address").unwrap();
    let wram = vec![0; 0x20_000];
    assert!(matches!(
        map.read_symbol(hardware, &wram),
        Err(ReadError::UnsupportedAddressSpace(AddressSpace::Hardware))
    ));

    let mut sram = hardware.clone();
    sram.space = AddressSpace::Sram;
    sram.address = Address::new(0x30_6000);
    assert!(matches!(
        map.read_symbol(&sram, &wram),
        Err(ReadError::UnsupportedAddressSpace(AddressSpace::Sram))
    ));

    let mut invalid_wram = hardware.clone();
    invalid_wram.space = AddressSpace::Wram;
    invalid_wram.address = Address::new(0x7D_FFFF);
    assert!(matches!(
        map.read_symbol(&invalid_wram, &wram),
        Err(ReadError::InvalidSymbolRegion)
    ));
    invalid_wram.address = Address::new(0x7E_0000);
    invalid_wram.width = 0;
    assert!(matches!(
        map.read_symbol(&invalid_wram, &wram),
        Err(ReadError::InvalidSymbolRegion)
    ));

    let player = map.lookup_id("player_x").unwrap();
    assert!(matches!(
        map.read_symbol(player, &[0; 0x1001]),
        Err(ReadError::WramTooShort { .. })
    ));
}

#[test]
fn ca65_include_is_exact_sorted_and_formats_short_and_long_operands() {
    let mut map = builtin();
    map.symbols.retain(|symbol| {
        [
            "cop_return_address",
            "cgram_staging_buffer",
            "program_state",
        ]
        .contains(&symbol.id.as_str())
    });
    // Deliberately disorder input: output order is by assembler name.
    map.symbols.reverse();

    assert_eq!(
        map.generate_ca65_include(),
        "; Generated by memory-map schema v1 for japan. Do not edit.\n\
; ROM SHA-256: f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548\n\
\n\
CGRAM_STAGING_BUFFER = $7F0600\n\
COP_RETURN_ADDRESS = $0036\n\
PROGRAM_STATE = $0450\n"
    );
}

#[test]
fn malformed_and_unknown_json_errors_are_typed() {
    assert!(matches!(MemoryMap::from_json("{"), Err(LoadError::Json(_))));

    let json = include_str!("../data/japan-v1.json").replacen(
        "\"schema_version\": 1,",
        "\"schema_version\": 1, \"schema_typo\": 1,",
        1,
    );
    assert!(matches!(
        MemoryMap::from_json(&json),
        Err(LoadError::Json(_))
    ));
}

#[test]
fn broad_gameplay_regions_remain_imported_with_exact_byte_widths() {
    let map = builtin();
    for id in [
        "event_flags",
        "inventory_items",
        "inventory_key_items",
        "inventory_weapons",
        "inventory_armor",
        "inventory_magic",
    ] {
        assert_eq!(
            map.lookup_id(id).unwrap().confidence,
            Confidence::ImportedClaim,
            "{id} is still an imported name/range lead"
        );
    }
    for id in ["current_map", "player_x", "player_y"] {
        assert_eq!(
            map.lookup_id(id).unwrap().confidence,
            Confidence::TraceCorroborated,
            "{id} has controlled ROM-backed trace evidence"
        );
    }
    assert_eq!(map.lookup_id("program_state").unwrap().width, 1);
    assert_eq!(map.lookup_id("current_map_times_two").unwrap().width, 1);
    assert_eq!(map.lookup_id("pending_map").unwrap().width, 1);
    assert_eq!(map.lookup_id("controller_1_normalized").unwrap().width, 1);
    assert_eq!(map.lookup_id("text_speed").unwrap().width, 1);
    assert_eq!(map.lookup_id("name_entry_cursor_row").unwrap().width, 1);
    assert_eq!(map.lookup_id("menu_cursor").unwrap().width, 1);
    assert_eq!(
        map.lookup_id("name_entry_cursor_row").unwrap().address,
        Address::new(0x7E_04C8)
    );
    assert_eq!(map.lookup_id("inventory_items").unwrap().width, 0x36);
    assert_eq!(
        map.lookup_id("inventory_key_items").unwrap().address,
        Address::new(0x7F_8036)
    );
    let armor = map.lookup_id("inventory_armor").unwrap();
    assert_eq!(armor.address, Address::new(0x7F_8068));
    assert!(armor.aliases.is_empty());
    assert_eq!(
        map.lookup_id("state_handler_pointer").unwrap().confidence,
        Confidence::StaticCorroborated
    );
}

#[test]
fn project_evidence_uses_current_block_and_callable_names() {
    let map = builtin();
    for symbol in &map.symbols {
        for evidence in &symbol.evidence {
            if evidence.source_id == "project-boot-assembly" {
                assert!(!evidence.locator.contains("lines "), "{}", evidence.locator);
                assert!(!evidence.locator.contains('/'), "{}", evidence.locator);
            }
            if evidence.source_id == "project-oracle-tests" {
                assert!(
                    evidence.locator.contains("scenario_suite_runs_on_one_boot")
                        || evidence
                            .locator
                            .contains("local_sram_trace_verifies_gameplay_symbols")
                        || evidence.locator.starts_with("run_scenario_child:"),
                    "{}",
                    evidence.locator
                );
            }
        }
    }
}
