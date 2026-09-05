//! Synthetic map-loading streams and tables, never extracted script bytes.
use assets::maps::scripts::{resolve_map, unpack_pointer, Command, Limits, ResourceKind};

const MAP_TABLE: usize = 0x06_959C;
const SUB_TABLE: usize = 0x06_A28C;
fn image(root: &[u8], sub: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0; 0x06_A505];
    bytes[MAP_TABLE..MAP_TABLE + 3].copy_from_slice(&[0, 1, 0xC0]);
    bytes[SUB_TABLE + 3..SUB_TABLE + 6].copy_from_slice(&[0, 2, 0xC0]);
    bytes[0x100..0x100 + root.len()].copy_from_slice(root);
    bytes[0x200..0x200 + sub.len()].copy_from_slice(sub);
    bytes
}

#[test]
fn resolves_calls_and_preserves_every_instruction() {
    let bytes = image(
        &[8, 0xF9, 1, 0, 0x10, 1, 0, 3, 0, 0],
        &[0x10, 2, 0, 4, 0, 8, 0xF8],
    );
    let program = resolve_map(&bytes, 0, Limits::default()).unwrap();
    assert_eq!(program.entry.value(), 0xC0_0100);
    assert_eq!(program.instructions.len(), 5);
    let resources: Vec<_> = program
        .instructions
        .iter()
        .filter_map(|instruction| match &instruction.command {
            Command::Resource {
                kind: ResourceKind::Layer,
                source,
            } => Some(source.value()),
            _ => None,
        })
        .collect();
    assert_eq!(resources, [0xC0_0400, 0xC0_0300]);
    for instruction in &program.instructions {
        let start = instruction.address.normalized().value() as usize;
        assert_eq!(
            instruction.bytes,
            bytes[start..start + instruction.bytes.len()]
        );
    }
}

#[test]
fn follows_deferred_streams_and_flagged_unwind_without_taking_jump() {
    // Root return does nothing; a deferred stream runs only after root END.
    let bytes = image(&[8, 0xF8, 8, 0xFA, 1, 0, 0], &[0]);
    assert_eq!(
        resolve_map(&bytes, 0, Limits::default())
            .unwrap()
            .instructions
            .len(),
        4
    );
    // Flagged call + FF returns before resolving the deliberately invalid operand.
    let bytes = image(&[8, 0xF9, 1, 0x80, 0], &[8, 0xFF, 0xFF, 0x7F]);
    assert_eq!(
        resolve_map(&bytes, 0, Limits::default())
            .unwrap()
            .instructions
            .len(),
        3
    );
    // FE likewise unwinds a flagged call; at root it merely consumes two operands.
    let bytes = image(&[8, 0xF9, 1, 0x80, 8, 0xFE, 2, 3, 0], &[8, 0xFE, 4, 5]);
    assert_eq!(
        resolve_map(&bytes, 0, Limits::default())
            .unwrap()
            .instructions
            .len(),
        4
    );
}

#[test]
fn handles_end_as_return_and_unflagged_jump_without_new_frame() {
    let bytes = image(&[8, 0xF9, 1, 0, 0], &[0]);
    assert_eq!(
        resolve_map(&bytes, 0, Limits::default())
            .unwrap()
            .instructions
            .len(),
        3
    );
    let bytes = image(&[8, 0xFF, 1, 0], &[0]);
    assert_eq!(
        resolve_map(&bytes, 0, Limits::default())
            .unwrap()
            .instructions
            .len(),
        2
    );
}

#[test]
fn bounds_ids_operands_control_flow_and_unknown_behavior() {
    let valid = image(&[0], &[0]);
    assert!(resolve_map(&valid, 0x450, Limits::default()).is_err());
    assert!(resolve_map(&[], 0, Limits::default()).is_err());
    for root in [
        &[0x80][..],
        &[8, 0xFD, 0, 0, 1, 0],
        &[8, 0xF9, 1, 0x40],
        &[8, 0xFF, 0xD3, 0],
        &[3],
    ] {
        let mut bytes = image(root, &[0]);
        // Move the stream to the end of input so truncation cannot read padding.
        bytes[MAP_TABLE..MAP_TABLE + 3].copy_from_slice(&[5, 0xA5, 0xC6]);
        bytes.extend(root);
        assert!(resolve_map(&bytes, 0, Limits::default()).is_err());
    }
    let looped = image(&[8, 0xFF, 1, 0], &[8, 0xFF, 1, 0]);
    assert!(resolve_map(
        &looped,
        0,
        Limits {
            instructions: 5,
            call_depth: 4
        }
    )
    .is_err());
    let recursive = image(&[8, 0xF9, 1, 0], &[8, 0xF9, 1, 0]);
    assert!(resolve_map(
        &recursive,
        0,
        Limits {
            instructions: 20,
            call_depth: 2
        }
    )
    .is_err());
    assert!(resolve_map(
        &valid,
        0,
        Limits {
            instructions: 0,
            call_depth: 4
        }
    )
    .is_err());
}

#[test]
fn packed_pointers_follow_loader_bank_arithmetic() {
    for (base, packed, expected) in [
        (0x86, 0, 0x86_8000),
        (0x86, 0x7FFF, 0x86_FFFF),
        (0x86, 0x8000, 0x87_8000),
        (0x86, 0x1C_8000, 0xBF_8000),
        (0x86, 0x1D_0000, 0xC0_0000),
        (0xC0, 0, 0xC0_0000),
        (0x86, 0x3D_0000, 0x00_8000),
        (0x86, 0x80_0000, 0x86_8000),
        (0xB3, 0x0B_0000, 0xC9_0000),
        (0xD9, 0x09_439E, 0xEB_439E),
    ] {
        let bytes = u32::to_le_bytes(packed);
        assert_eq!(
            unpack_pointer([bytes[0], bytes[1], bytes[2]], base)
                .unwrap()
                .value(),
            expected
        );
    }
    // The CPU arithmetic produces $00:0000; reject it because it is not ROM-backed.
    assert!(unpack_pointer([0, 0, 0x20], 0xC0).is_err());
}

#[test]
fn frames_every_resource_family_and_opaque_display_audio_controls() {
    for (bytes, kind) in [
        (vec![0x80, 1, 2, 3, 0, 3, 0, 4, 5], ResourceKind::Graphics),
        (vec![0x40, 1, 2, 3, 0, 3, 0], ResourceKind::Palette),
        (vec![0x20, 1, 2, 3, 4, 0, 3, 0], ResourceKind::Metatiles),
        (vec![0x10, 1, 0, 3, 0], ResourceKind::Layer),
        (vec![4, 0, 3, 0, 0], ResourceKind::Background),
        (vec![4, 0, 3, 0, 1, 2], ResourceKind::Background),
        (vec![2, 1, 2, 0, 3, 0], ResourceKind::Audio),
        (vec![1, 0, 2, 3, 0, 3, 0], ResourceKind::Sprite),
        (vec![1, 1, 2, 3, 0, 3, 0], ResourceKind::Sprite),
    ] {
        let mut root = bytes.clone();
        root.extend([8, 0, 0xAA, 0xBB, 8, 0xFC, 7, 0, 0]);
        let program = resolve_map(&image(&root, &[]), 0, Limits::default()).unwrap();
        assert_eq!(program.instructions.len(), 4);
        assert_eq!(program.instructions[0].bytes, bytes);
        assert_eq!(
            program.instructions[0].command,
            Command::Resource {
                kind,
                source: rom::RuntimeRomAddress::new(0xC0_0300).unwrap()
            }
        );
        assert_eq!(
            program.instructions[1].address.value(),
            0xC0_0100 + u32::try_from(bytes.len()).unwrap()
        );
        assert_eq!(program.instructions[1].command, Command::Display);
        assert_eq!(program.instructions[2].command, Command::AudioSelection);
        assert_eq!(program.instructions[3].command, Command::End);
    }
}

fn second_subscript(bytes: &mut [u8], script: &[u8]) {
    bytes[SUB_TABLE + 6..SUB_TABLE + 9].copy_from_slice(&[0, 3, 0xC0]);
    bytes[0x300..0x300 + script.len()].copy_from_slice(script);
}
fn addresses(bytes: &[u8]) -> Vec<u32> {
    resolve_map(bytes, 0, Limits::default())
        .unwrap()
        .instructions
        .iter()
        .map(|i| i.address.value())
        .collect()
}

#[test]
fn nested_calls_restore_flags_and_unflagged_jumps_keep_the_return_frame() {
    let mut bytes = image(
        &[8, 0xF9, 1, 0x80, 0],
        &[8, 0xF9, 2, 0, 8, 0xFF, 0xFF, 0x7F],
    );
    second_subscript(&mut bytes, &[8, 0xFE, 0, 0, 0]);
    assert_eq!(
        addresses(&bytes),
        [0xC0_0100, 0xC0_0200, 0xC0_0300, 0xC0_0304, 0xC0_0204, 0xC0_0104]
    );
    let mut bytes = image(&[8, 0xF9, 1, 0, 0], &[8, 0xFF, 2, 0]);
    second_subscript(&mut bytes, &[0]);
    assert_eq!(
        addresses(&bytes),
        [0xC0_0100, 0xC0_0200, 0xC0_0300, 0xC0_0104]
    );
}

#[test]
fn deferrals_replace_cancel_and_chain_only_at_root_end() {
    let mut bytes = image(&[8, 0xFA, 1, 0, 8, 0xF9, 2, 0, 8, 0xFA, 0, 0, 0], &[0]);
    second_subscript(&mut bytes, &[8, 0xFA, 3, 0, 0]);
    assert_eq!(
        addresses(&bytes),
        [0xC0_0100, 0xC0_0104, 0xC0_0300, 0xC0_0304, 0xC0_0108, 0xC0_010C]
    );
    let mut bytes = image(&[8, 0xFA, 1, 0, 0], &[8, 0xFA, 2, 0, 0]);
    second_subscript(&mut bytes, &[0]);
    assert_eq!(
        addresses(&bytes),
        [0xC0_0100, 0xC0_0104, 0xC0_0200, 0xC0_0204, 0xC0_0300]
    );
}

#[test]
fn bank_and_exact_budget_boundaries_have_specific_errors() {
    use assets::maps::scripts::ScriptError;
    let mut bytes = image(&[0], &[0]);
    bytes[MAP_TABLE..MAP_TABLE + 3].copy_from_slice(&[0xFF, 0xFF, 0xC0]);
    let limits = Limits {
        instructions: 1,
        call_depth: 0,
    };
    assert!(resolve_map(&bytes, 0, limits).is_ok());
    bytes[0xFFFF] = 0x10;
    assert_eq!(
        resolve_map(&bytes, 0, limits),
        Err(ScriptError::BankCrossing)
    );
    let bytes = image(&[8, 0xF8, 0], &[0]);
    assert_eq!(
        resolve_map(&bytes, 0, limits),
        Err(ScriptError::InstructionLimit)
    );
    assert!(resolve_map(
        &bytes,
        0,
        Limits {
            instructions: 2,
            ..limits
        }
    )
    .is_ok());
    let bytes = image(&[8, 0xF9, 1, 0, 0], &[0]);
    assert_eq!(resolve_map(&bytes, 0, limits), Err(ScriptError::CallDepth));
    assert_eq!(
        resolve_map(
            &bytes,
            0,
            Limits {
                instructions: 65537,
                ..limits
            }
        ),
        Err(ScriptError::InvalidLimits)
    );
    assert_eq!(
        resolve_map(
            &bytes,
            0,
            Limits {
                call_depth: 65,
                ..limits
            }
        ),
        Err(ScriptError::InvalidLimits)
    );
    assert_eq!(
        resolve_map(&bytes, 0x450, limits),
        Err(ScriptError::TableIndex {
            subscript: false,
            index: 0x450
        })
    );
}

#[test]
fn resource_pointers_use_active_stream_bank_and_restore_it_after_calls() {
    let root = [0x10, 1, 0, 0, 0, 8, 0xF9, 1, 0, 0x10, 1, 0, 0, 0, 0];
    let mut bytes = image(&[], &[0x10, 1, 0, 0, 0, 0]);
    bytes.resize(0x18_8100 + root.len(), 0);
    bytes[0x18_8100..].copy_from_slice(&root);
    bytes[MAP_TABLE..MAP_TABLE + 3].copy_from_slice(&[0, 0x81, 0x98]);
    let program = resolve_map(&bytes, 0, Limits::default()).unwrap();
    let sources: Vec<_> = program
        .instructions
        .iter()
        .filter_map(|instruction| match instruction.command {
            Command::Resource { source, .. } => Some(source.value()),
            _ => None,
        })
        .collect();
    assert_eq!(sources, [0x98_8000, 0xC0_0000, 0x98_8000]);
}

#[test]
fn last_table_entries_are_accepted_but_truncated_tables_are_not() {
    use assets::maps::scripts::ScriptError;
    let mut bytes = image(&[8, 0xFF, 0xD2, 0], &[0]);
    bytes[MAP_TABLE + 0x44F * 3..MAP_TABLE + 0x450 * 3].copy_from_slice(&[0, 1, 0xC0]);
    bytes[SUB_TABLE + 0xD2 * 3..SUB_TABLE + 0xD3 * 3].copy_from_slice(&[0, 2, 0xC0]);
    assert_eq!(
        resolve_map(&bytes, 0x44F, Limits::default())
            .unwrap()
            .instructions
            .len(),
        2
    );
    assert!(matches!(
        resolve_map(&bytes[..MAP_TABLE + 2], 0, Limits::default()),
        Err(ScriptError::Truncated { needed: 3, .. })
    ));
    assert!(matches!(
        resolve_map(&bytes[..SUB_TABLE + 0xD3 * 3 - 1], 0, Limits::default()),
        Err(ScriptError::Truncated { needed: 3, .. })
    ));
}
