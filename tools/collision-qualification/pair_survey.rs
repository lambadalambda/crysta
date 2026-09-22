//! Bounded AllClear/static-BG1 source survey, not native execution or reachability.
//! Build via build.sh; pair_survey ROM > local/collision-qualification/pairs.jsonl.
use assets::{
    compression,
    maps::{
        scripts::{resolve_map, Command, Instruction, Limits, ResourceKind, MAP_COUNT},
        StaticLayer,
    },
};
use serde_json::{json, Value};
use std::io::{self, Write};

#[derive(Debug, Default, PartialEq, Eq)]
struct PairCounts {
    unflagged_by_first_raw_type: [usize; 32],
    first_flagged_by_raw_type: [usize; 32],
    second_flagged_by_first_raw_type: [usize; 32],
    top_row_raw_type8: usize,
}

fn count_pairs(cells: &[u16], width: usize) -> PairCounts {
    assert!(width > 0 && cells.len() % width == 0);
    let raw_type = |word: u16| usize::from((word >> 9) & 31);
    let mut counts = PairCounts::default();
    for (i, &second) in cells.iter().enumerate() {
        if raw_type(second) != 8 {
            continue;
        }
        if i < width {
            counts.top_row_raw_type8 += 1;
            continue;
        }
        let first = cells[i - width];
        // Bit 15 overrides dispatch. Do not treat a flagged stored type 8 as
        // second-type8, or a flagged first word as its stored raw type class.
        let bucket = if second & 0x8000 != 0 {
            &mut counts.second_flagged_by_first_raw_type
        } else if first & 0x8000 != 0 {
            &mut counts.first_flagged_by_raw_type
        } else {
            &mut counts.unflagged_by_first_raw_type
        };
        bucket[raw_type(first)] += 1;
    }
    counts
}

fn selected<'a>(
    instructions: &'a [Instruction],
    kind: ResourceKind,
    operands: &[u8],
) -> Vec<&'a Instruction> {
    instructions
        .iter()
        .filter(|i| {
            matches!(i.command, Command::Resource { kind: k, .. } if k == kind)
                && i.bytes.get(1..1 + operands.len()) == Some(operands)
        })
        .collect()
}

fn unique<'a>(candidates: &[&'a Instruction]) -> Result<&'a Instruction, &'static str> {
    match candidates {
        [] => Err("missing"),
        [one] => Ok(one),
        _ => Err("duplicate"),
    }
}

fn source_offset(instruction: &Instruction) -> usize {
    let Command::Resource { source, .. } = instruction.command else {
        unreachable!()
    };
    source.normalized().value() as usize
}

fn source_reference(instruction: &Instruction, image_len: usize) -> Value {
    let Command::Resource { source, .. } = instruction.command else {
        unreachable!()
    };
    let offset = source_offset(instruction);
    json!({
        "instruction_offset": instruction.address.normalized().value(),
        "instruction_size": instruction.bytes.len(),
        "instruction_cpu_address": instruction.address.value(),
        "source_cpu_address": source.value(),
        "offset": offset,
        "decode_limit_exclusive": ((offset | 0xFFFF) + 1).min(image_len),
    })
}

fn sha256(bytes: &[u8]) -> String {
    rom::digests(bytes)
        .sha256
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn extent(image: &[u8], range: std::ops::Range<usize>) -> Value {
    json!({"offset": range.start, "end_exclusive": range.end,
        "size": range.len(), "sha256": sha256(&image[range])})
}

fn decode_attributes(image: &[u8], offset: usize) -> Result<compression::DecodedPacket, String> {
    if offset >= rom::Rom::IMAGE_SIZE {
        return Err("attribute source outside supported ROM banks".into());
    }
    let end = ((offset | 0xFFFF) + 1).min(image.len());
    let input = image
        .get(offset..end)
        .ok_or("attribute source outside image")?;
    let packet = compression::decode(input, 512).map_err(|e| e.to_string())?;
    if packet.data.len() != 512 {
        return Err(format!(
            "attributes require 512 bytes, packet produced {}",
            packet.data.len()
        ));
    }
    Ok(packet)
}

fn survey_map(image: &[u8], map: u16) -> Value {
    let mut record = json!({"kind": "map", "map": map});
    // resolve_map explicitly uses EventFlags::AllClear. StaticBackground's
    // allowlist is intentionally NOT used for this broad source survey.
    let program = match resolve_map(image, map, Limits::default()) {
        Ok(program) => program,
        Err(error) => {
            record["status"] = json!("error");
            record["stage"] = json!("resolve_map");
            record["reason"] = json!(error.to_string());
            return record;
        }
    };
    record["script_entry_cpu_address"] = json!(program.entry.value());
    record["script_entry_offset"] = json!(program.entry.normalized().value());
    let layers = selected(&program.instructions, ResourceKind::Layer, &[1]);
    let attrs = selected(
        &program.instructions,
        ResourceKind::Metatiles,
        &[0, 8, 0, 0x81],
    );
    record["selected_resources"] = json!({
        "bg1": layers.iter().map(|i| source_reference(i, image.len())).collect::<Vec<_>>(),
        "attributes": attrs.iter().map(|i| source_reference(i, image.len())).collect::<Vec<_>>(),
    });
    let (layer_source, attr_source) = match (unique(&layers), unique(&attrs)) {
        (Ok(layer), Ok(attr)) => (source_offset(layer), source_offset(attr)),
        (layer, attr) => {
            record["status"] = json!("skipped");
            record["stage"] = json!("select_resources");
            record["reason"] = json!({"bg1": layer.err(), "attributes": attr.err()});
            return record;
        }
    };
    let layer = match StaticLayer::from_rom(image, layer_source) {
        Ok(layer) => layer,
        Err(error) => {
            record["status"] = json!("error");
            record["stage"] = json!("decode_bg1");
            record["reason"] = json!(error.to_string());
            return record;
        }
    };
    record["bg1_source"] = extent(image, layer.source_range());
    record["size"] = json!({"width_cells": layer.width(), "height_cells": layer.height(),
        "decoded_bytes": layer.cells().len() * 2});
    let packet = match decode_attributes(image, attr_source) {
        Ok(packet) => packet,
        Err(error) => {
            record["status"] = json!("error");
            record["stage"] = json!("decode_attributes");
            record["reason"] = json!(error);
            return record;
        }
    };
    record["attributes_source"] = extent(image, attr_source..attr_source + packet.consumed);
    record["attributes_decoded_bytes"] = json!(packet.data.len());
    let attributes: &[u8; 512] = packet.data.as_slice().try_into().expect("checked length");
    let cells: Vec<_> = layer
        .attributed_cells(attributes)
        .iter()
        .map(|c| c.raw())
        .collect();
    let counts = count_pairs(&cells, layer.width());
    record["above8"] = json!({
        "unflagged_by_first_raw_type": counts.unflagged_by_first_raw_type,
        "first_flagged_by_raw_type": counts.first_flagged_by_raw_type,
        "second_flagged_by_first_raw_type": counts.second_flagged_by_first_raw_type,
        "top_row_raw_type8": counts.top_row_raw_type8,
    });
    record["status"] = json!("resolved");
    record
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 2 {
        return Err("usage: pair_survey ROM > local/collision-qualification/pairs.jsonl".into());
    }
    let rom = rom::Rom::load(&std::fs::read(&args[1])?)?;
    if rom.revision() != rom::Revision::Japan {
        return Err("pair_survey requires the authenticated Japanese ROM".into());
    }
    let mut out = io::BufWriter::new(io::stdout().lock());
    writeln!(
        out,
        "{}",
        json!({
            "kind": "survey", "schema": 1,
            "revision": rom.revision().id(), "normalized_rom_sha256": sha256(rom.image()),
            "normalized_rom_size": rom.image().len(),
            "map_start": 0, "map_end_exclusive": MAP_COUNT,
            "scope": "AllClear/static BG1 projection; not native execution; no event patches or actors; not proof of unreachability",
            "selection": {"Layer": [1], "Metatiles": [0, 8, 0, 0x81]},
            "limits": {"instructions": Limits::default().instructions, "call_depth": Limits::default().call_depth,
                "source_bank_bytes": 65536, "attributes_decoded_bytes": 512},
            "offsets": "normalized headerless ROM; extents end-exclusive",
            "counts": "arrays indexed by first raw (word >> 9) & 31; first is cell directly above second raw type 8; unflagged requires both bit15 clear; first_flagged requires second bit15 clear; second_flagged excludes all such pairs from dispatch counts; top-row raw8 has no first cell; no handler classification",
        })
    )?;
    for map in 0..MAP_COUNT {
        writeln!(out, "{}", survey_map(rom.image(), map))?;
    }
    out.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use assets::maps::scripts::Command;
    use rom::RuntimeRomAddress;

    fn resource(kind: ResourceKind, operands: &[u8]) -> Instruction {
        let mut bytes = vec![if kind == ResourceKind::Layer {
            0x10
        } else {
            0x20
        }];
        bytes.extend_from_slice(operands);
        bytes.extend_from_slice(&[0, 0, 0]); // Synthetic pointer encoding only.
        Instruction {
            address: RuntimeRomAddress::new(0x86_9000).unwrap(),
            bytes,
            command: Command::Resource {
                kind,
                source: RuntimeRomAddress::new(0xC1_0000).unwrap(),
            },
        }
    }

    #[test]
    fn pairs_are_vertical_in_same_column_not_adjacent_or_wrapped() {
        let cells = [8 << 9, 5 << 9, 27 << 9, 8 << 9, 0, 8 << 9];
        let counts = count_pairs(&cells, 3);
        let mut expected = PairCounts::default();
        expected.top_row_raw_type8 = 1;
        expected.unflagged_by_first_raw_type[8] = 1;
        expected.unflagged_by_first_raw_type[27] = 1;
        assert_eq!(counts, expected);
        assert_eq!(count_pairs(&[], 1), PairCounts::default());
    }

    #[test]
    fn flags_are_not_genuine_raw_type_dispatch_and_bit14_is_not_a_flag() {
        let cells = [
            0x8000 | (5 << 9),
            9 << 9,
            0x8000 | (16 << 9),
            0x4000 | (31 << 9),
            8 << 9,
            0x8000 | (8 << 9),
            0x8000 | (8 << 9),
            0x4000 | (8 << 9),
        ];
        let mut expected = PairCounts::default();
        expected.first_flagged_by_raw_type[5] = 1;
        expected.second_flagged_by_first_raw_type[9] = 1;
        expected.second_flagged_by_first_raw_type[16] = 1;
        expected.unflagged_by_first_raw_type[31] = 1;
        assert_eq!(count_pairs(&cells, 4), expected);
    }

    #[test]
    fn exact_resource_operands_only_no_fallback_to_other_layers() {
        let instructions = vec![
            resource(ResourceKind::Layer, &[2]),
            resource(ResourceKind::Metatiles, &[0, 8, 0, 0x80]),
            resource(ResourceKind::Layer, &[1]),
            resource(ResourceKind::Metatiles, &[0, 8, 0, 0x81]),
        ];
        let layers = selected(&instructions, ResourceKind::Layer, &[1]);
        let attrs = selected(&instructions, ResourceKind::Metatiles, &[0, 8, 0, 0x81]);
        assert_eq!(unique(&layers), Ok(&instructions[2]));
        assert_eq!(unique(&attrs), Ok(&instructions[3]));
        assert_eq!(
            unique(&selected(&instructions[..2], ResourceKind::Layer, &[1])),
            Err("missing")
        );
    }

    #[test]
    fn repeated_selected_resource_is_ambiguous_even_at_same_source() {
        for (kind, operands) in [
            (ResourceKind::Layer, vec![1]),
            (ResourceKind::Metatiles, vec![0, 8, 0, 0x81]),
        ] {
            let instruction = resource(kind, &operands);
            let instructions = [instruction.clone(), instruction];
            assert_eq!(
                unique(&selected(&instructions, kind, &operands)),
                Err("duplicate")
            );
        }
    }
}
