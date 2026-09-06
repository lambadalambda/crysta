//! Opt-in source scene and carrying atlas; progression selects phases, never this adapter.
use super::{invalid, json, raster, BTreeMap, Result, Value};
use assets::sprites::{PandoraActorPhase, PandoraSprites};

pub(super) struct Presentation {
    scenes: BTreeMap<&'static str, (u16, Vec<PandoraActorPhase>)>,
}

pub(super) fn frame_key(art: u32, selector: u8, record: usize, mirror: bool) -> String {
    format!("pandora:{art:06x}:{selector}:{record}:{}", u8::from(mirror))
}

impl Presentation {
    pub(super) fn scene(
        &self,
        map: u16,
        phase: &str,
        ark_key: &str,
        position: (u16, u16),
    ) -> Result<Value> {
        let (source_map, actors) = self
            .scenes
            .get(phase)
            .ok_or_else(|| invalid("unsupported Pandora scene phase"))?;
        if map != *source_map {
            return Err(invalid("Pandora scene phase/map mismatch").into());
        }
        let mut entries: Vec<_> = actors
            .iter()
            .map(|actor| {
                (
                    actor.position[1],
                    usize::from(actor.tie_rank),
                    json!({"id":format!("pandora:{:06x}",actor.source_id),
                    "key":frame_key(actor.art_id,actor.selector,0,actor.hflip),
                    "position":actor.position,"priority":actor.priority_override.unwrap_or(2)}),
                )
            })
            .collect();
        entries.push((
            position.1,
            actors.len(),
            json!({"id":"ark","key":ark_key,"position":[position.0,position.1],"priority":2}),
        ));
        // Priority is a BG-comparison property, NOT an alternate OBJ painter order.
        entries.sort_by_key(|(y, tie, _)| (*y, *tie));
        Ok(Value::Array(
            entries.into_iter().map(|(_, _, entry)| entry).collect(),
        ))
    }
}

pub(super) fn compile(
    rom: &rom::Rom,
    frames: &mut serde_json::Map<String, Value>,
) -> Result<(Presentation, Value, Value)> {
    let sprites = PandoraSprites::from_rom(rom.image())?;
    for art in sprites.art() {
        for list in art.lists() {
            for (record, frame) in list.frames().iter().enumerate() {
                for mirror in [false, true] {
                    let key = frame_key(art.source_id(), list.selector(), record, mirror);
                    let pixels = raster(
                        frame.composition(),
                        art.graphics(),
                        art.palette(),
                        art.palette_base(),
                        mirror,
                    )?;
                    if frames.insert(key, pixels).is_some() {
                        return Err(invalid("duplicate Pandora frame key").into());
                    }
                }
            }
        }
    }
    let mut scenes = BTreeMap::new();
    let mut manifest = serde_json::Map::new();
    for phase in sprites.phases() {
        for actor in phase.actors() {
            if !frames.contains_key(&frame_key(actor.art_id, actor.selector, 0, actor.hflip))
                || !matches!(actor.priority_override, None | Some(3))
            {
                return Err(invalid("unsupported Pandora scene art/priority").into());
            }
        }
        scenes.insert(phase.id(), (phase.map_id(), phase.actors().to_vec()));
        manifest.insert(phase.id().into(),json!({"map_id":phase.map_id(),
            "ark_tie_rank":phase.ark_tie_rank(),
            "policy":"source-endpoints-not-script-timing",
            "limits":phase.limits().iter().map(|limit| format!("{limit:?}")).collect::<Vec<_>>(),
            "actors":phase.actors().iter().map(|actor| json!({
                "id":format!("pandora:{:06x}",actor.source_id),
                "art_id":actor.art_id,"position":actor.position,"position_source":actor.position_source,
                "selector":actor.selector,"hflip":actor.hflip,"tie_rank":actor.tie_rank,
                "priority":actor.priority_override.unwrap_or(2),
                "key":frame_key(actor.art_id,actor.selector,0,actor.hflip)
            })).collect::<Vec<_>>()
        }));
    }
    Ok((
        Presentation { scenes },
        Value::Object(manifest),
        super::carry::compile(frames)?,
    ))
}

#[cfg(test)]
#[path = "pandora_tests.rs"]
mod tests;
