//! Local projection/export of loading instructions and independently decoded layers.
use crate::{invalid, sha256, Result};
use assets::maps::{
    scripts::{resolve_map, Command, Limits, MapProgram, ResourceKind},
    StaticLayer,
};
use rom::Rom;
use serde_json::{json, Value};

pub(super) fn layers(rom: &Rom, map_id: u16) -> Result<(MapProgram, Vec<StaticLayer>)> {
    let program = resolve_map(rom.image(), map_id, Limits::default())?;
    let mut layers = Vec::new();
    for instruction in &program.instructions {
        if let Command::Resource {
            kind: ResourceKind::Layer,
            source,
        } = instruction.command
        {
            if !matches!(instruction.bytes[1], 1..=3) {
                return Err(invalid(
                    "layer extraction supports only compressed first/second-layer flags 1..3",
                )
                .into());
            }
            layers.push(StaticLayer::from_rom(
                rom.image(),
                source.normalized().value() as usize,
            )?);
        }
    }
    Ok((program, layers))
}

pub(super) fn inspect(rom: &Rom, map_id: u16) -> Result<Value> {
    let (program, layers) = layers(rom, map_id)?;
    let instructions:Vec<_>=program.instructions.iter().map(|instruction| {
        let (operation,source)=match instruction.command {
            Command::Resource{kind,source}=>(format!("{kind:?} {source}"),Some(source.value())),
            _=>(format!("{:?}",instruction.command),None),
        };
        json!({"address":instruction.address.value(),"address_hex":instruction.address.to_string(),
            "bytes":instruction.bytes,"operation":operation,"resource_source":source})
    }).collect();
    let layer_instructions = program.instructions.iter().filter(|instruction| {
        matches!(
            instruction.command,
            Command::Resource {
                kind: ResourceKind::Layer,
                ..
            }
        )
    });
    let layers:Vec<_>=layers.iter().zip(layer_instructions).map(|(layer,instruction)| json!({
        "instruction_address":instruction.address.value(),"flags":instruction.bytes[1],
        "source_range":[layer.source_range().start,layer.source_range().end],
        "width":layer.width(),"height":layer.height(),"layer_sha256":sha256(&layer.layer_bytes()),
        "cells":layer.cells().iter().map(|cell|cell.raw()).collect::<Vec<_>>()
    })).collect();
    Ok(
        json!({"schema_version":1,"kind":"map-loading-resource-projection",
        "revision":rom.revision().id(),"rom_sha256":sha256(rom.image()),"map_id":map_id,
        "entry":program.entry.value(),"instructions":instructions,"layers":layers,
        "limits":"Audio/display not executed; conditional branches and alternate layer modes rejected. Layers are separate source loads, not final composition."}),
    )
}
