//! Offline ROM-to-PandoraData adapter. Never called by the live host.
use crate::{invalid, pandora_navigation, sha256, Result};
use assets::{
    maps::visual::pandora::PandoraBackground,
    text::pandora::{PandoraDialogue, DIRECT_INVOCATIONS},
};
use room_core::{
    pots::SourceObject,
    slice::{
        Anchor, CollisionKey, Cue, Invocation, MotionFrame, MotionKey, PandoraText, ProfileRoom,
        RequestPages, ScenePhase,
    },
    Direction,
};
use serde_json::{json, Value};

fn require(ok: bool, message: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(invalid(message).into())
    }
}
fn source(image: &[u8], address: u32, length: usize) -> Result<&[u8]> {
    let at = (address & 0x3fffff) as usize;
    image
        .get(
            at..at
                .checked_add(length)
                .ok_or_else(|| invalid("source extent overflow"))?,
        )
        .ok_or_else(|| invalid("truncated Pandora operand").into())
}
fn word(image: &[u8], address: u32) -> Result<u16> {
    Ok(u16::from_le_bytes(source(image, address, 2)?.try_into()?))
}

fn compile_text(image: &[u8]) -> Result<(PandoraText, Value)> {
    let dialogue = PandoraDialogue::from_rom(image)?;
    let mut requests = Vec::new();
    let mut resources = Vec::new();
    for request in dialogue.requests() {
        let mut pages = Vec::new();
        let mut hashes = Vec::new();
        for (i, page) in request.pages().iter().enumerate() {
            let key = request
                .page_id(i)
                .ok_or_else(|| invalid("Pandora page key"))?;
            pages.push(key);
            hashes.push(json!({"key":key,"width":page.width(),"height":page.height(),
                "background":page.background_index(),"boundary":page.boundary_source(),
                "ack":format!("{:?}",page.acknowledgement()),"indexed_sha256":sha256(page.indexed())}));
        }
        requests.push(RequestPages {
            source: request.source(),
            pages,
            choice: request.choice_catalog().is_some(),
        });
        resources.push(
            json!({"source":request.source(),"choice":request.choice_catalog(),"pages":hashes}),
        );
    }
    for (invocation, (_, resource)) in room_core::slice::Invocation::ALL
        .into_iter()
        .zip(DIRECT_INVOCATIONS)
    {
        require(
            invocation.source() == resource,
            "core/source invocation correspondence",
        )?;
    }
    Ok((
        PandoraText::new(requests)?,
        json!({"resources":resources,"invocations":DIRECT_INVOCATIONS.as_slice()}),
    ))
}

fn source_objects(image: &[u8], nav: &pandora_navigation::Navigation) -> Result<Vec<SourceObject>> {
    require(
        source(image, 0x8796d7, 1)? == [0xa9]
            && word(image, 0x96e1a6)? == 0
            && word(image, 0x96e1ab)? == 0,
        "FA/FB source fallback",
    )?;
    let replacement = word(image, 0x8796d8)?;
    require(replacement == 0xf8, "qualified pot replacement")?;
    let room = nav
        .profile("c-direct")
        .ok_or_else(|| invalid("C profile"))?
        .room();
    let [l, t, r, b] = room.sample_halo().ok_or_else(|| invalid("C source halo"))?;
    let mut objects = Vec::new();
    for y in t..b {
        for x in l..r {
            let cell = y * room.width() + x;
            let raw = room.cells()[usize::from(cell)];
            if matches!(raw & 511, 0xfa | 0xfb) {
                objects.push(SourceObject {
                    cell,
                    raw,
                    replacement,
                });
            }
        }
    }
    require(!objects.is_empty(), "missing bounded source pots")?;
    Ok(objects)
}

fn compile_rooms(
    image: &[u8],
    nav: &pandora_navigation::Navigation,
    objects: &[SourceObject],
) -> Result<Vec<ProfileRoom>> {
    use CollisionKey::*;
    let mut rooms = Vec::new();
    for key in CollisionKey::ALL {
        let profile = match key {
            Town => Some("a"),
            Resident => Some("13"),
            CClosed | CReaction => Some("c-direct"),
            CDamaged => Some("c-first-hit"),
            COpen => Some("c-departed"),
            Box | BoxOpened => Some("21"),
            Tour41 => Some("41-control"),
            _ => None,
        };
        let (width, height, mut cells, halo) = if let Some(name) = profile {
            let r = nav
                .profile(name)
                .ok_or_else(|| invalid("navigation source profile"))?
                .room();
            (
                r.width(),
                r.height(),
                r.cells().to_vec(),
                r.sample_halo().ok_or_else(|| invalid("navigation halo"))?,
            )
        } else {
            let bg = PandoraBackground::from_rom(image, key.map())?;
            // E/20 begin as source cache-independent bases. The core-owned resident
            // sheet overlay supplies live door/pot patches, never pre-consumed data.
            let halo = match key {
                CellarE => [6, 52, 10, 55],
                Cellar20 => [22, 52, 26, 55],
                Tour44 => [24, 28, 25, 29],
                Tour42 => [8, 28, 9, 29],
                Tour43 => [24, 12, 25, 13],
                _ => return Err(invalid("unqualified profile").into()),
            };
            (
                u16::try_from(bg.background().layer().width())?,
                u16::try_from(bg.background().layer().height())?,
                bg.attributed_grid().iter().map(|c| c.raw()).collect(),
                halo,
            )
        };
        if matches!(key, CClosed | CDamaged | CReaction | COpen) {
            // PotState owns the ledger. Every supplied C variant retains source
            // pots, including reaction/open; overlays apply removals exactly once.
            for object in objects {
                cells[usize::from(object.cell)] = object.raw;
            }
        }
        if key == CReaction {
            let opened = nav
                .profile("c-departed")
                .ok_or_else(|| invalid("opened source cells"))?
                .room();
            for i in [651, 683] {
                cells[i] = opened.cells()[i] | 0x8000;
            }
        }
        // No type normalization here. The pending core-owned bounded classifier
        // seam must consume the same direction/cell rules as Profile::sample.
        rooms.push(ProfileRoom {
            key,
            room: room_core::Room::new_passive(width, height, cells)?.with_sample_halo(halo)?,
        });
    }
    Ok(rooms)
}

/// Only the explicit reconstruction samples, not the preceding cue completion.
/// COP14 uses queued+(8,16), NOT ordinary exit-selector adjustments. Both arrival
/// scripts restore standing without translation; timing belongs to semantic policy.
fn forced_arrivals(image: &[u8]) -> Result<Vec<(MotionKey, MotionFrame)>> {
    require(
        word(image, 0x848800)? == 0x88ac && word(image, 0x848802)? == 0x89d1,
        "forced arrival dispatch",
    )?;
    for (site, target, sequence) in [(0x8488e2, 0x84a2f3_u32, 0), (0x848a07, 0x84a308, 1)] {
        let target_bytes = target.to_le_bytes();
        require(
            source(image, site, 6)?
                == [
                    2,
                    0xcb,
                    0,
                    target_bytes[0],
                    target_bytes[1],
                    target_bytes[2],
                ],
            "forced arrival player target",
        )?;
        require(
            source(image, target, 21)?
                == [
                    0xbd, 4, 0, 0x29, 0xff, 0xef, 0x9d, 4, 0, 2, 0xb6, 2, 0x84, sequence, 0, 0, 2,
                    0x8e, 0x4c, 0x54, 0xa2,
                ],
            "stationary forced arrival facing",
        )?;
    }
    use Invocation::*;
    let mut arrivals = Vec::new();
    for (cue, site, map, mode, selector, scene) in [
        (Cue::BoxReload, 0x88ad53, 0x21, 7, 1, ScenePhase::BoxOpening),
        (
            Cue::Returned(OpeningFourth),
            0x88aeab,
            0x41,
            4,
            2,
            ScenePhase::Tour410,
        ),
        (
            Cue::Returned(TourLeave41),
            0x89d476,
            0x44,
            0,
            2,
            ScenePhase::Tour44,
        ),
        (
            Cue::Returned(TourLeave44),
            0x89d4b4,
            0x42,
            0,
            2,
            ScenePhase::Tour42,
        ),
        (
            Cue::Returned(TourLeave42),
            0x89d4d9,
            0x43,
            0,
            2,
            ScenePhase::Tour43,
        ),
        (
            Cue::Returned(TourLeave43),
            0x89d4fe,
            0x41,
            0,
            2,
            ScenePhase::Tour410,
        ),
    ] {
        require(
            source(image, site, 2)? == [2, 0x14]
                && word(image, site + 2)? == map
                && source(image, site + 4, 2)? == [mode, selector],
            "forced COP14 operands",
        )?;
        let add = |at, offset| -> Result<u16> {
            word(image, at)?
                .checked_add(offset)
                .ok_or_else(|| invalid("forced position overflow").into())
        };
        arrivals.push((
            MotionKey::Cue(cue),
            MotionFrame {
                map_id: map,
                anchor: Anchor {
                    position: (add(site + 6, 8)?, add(site + 8, 16)?),
                    facing: if selector == 1 {
                        Direction::Down
                    } else {
                        Direction::Up
                    },
                },
                reload: true,
                scene,
            },
        ));
    }
    Ok(arrivals)
}

#[cfg(test)]
mod tests {
    use super::*;
    use room_core::slice::Invocation;

    fn owned_rom() -> rom::Rom {
        rom::Rom::load(
            &std::fs::read(std::env::var("PANDORA_ROM").expect("owned ROM test harness")).unwrap(),
        )
        .unwrap()
    }
    #[test]
    #[ignore = "owned ROM; run tools/pandora-runtime-qualification/run.sh"]
    fn authentic_text_preserves_resources_invocations_and_choice_contexts() {
        let rom = owned_rom();
        let (text, manifest) = compile_text(rom.image()).unwrap();
        assert_eq!(manifest["resources"].as_array().unwrap().len(), 33);
        assert_eq!(manifest["invocations"].as_array().unwrap().len(), 34);
        let dialogue = PandoraDialogue::from_rom(rom.image()).unwrap();
        for invocation in Invocation::ALL
            .into_iter()
            .chain([Invocation::ResidentRetry, Invocation::ResidentRefusal])
        {
            let request = dialogue.request(invocation.source()).unwrap();
            assert_eq!(
                text.pages(invocation),
                (0..request.pages().len())
                    .map(|i| request.page_id(i).unwrap())
                    .collect::<Vec<_>>()
            );
        }
        assert_eq!(text.pages(Invocation::BoxWarning).len(), 2);
        assert_eq!(
            text.pages(Invocation::TourLeave41),
            text.pages(Invocation::TourLeave44)
        );
        assert!(compile_text(&[0; 64]).is_err());
    }
    #[test]
    #[ignore = "owned ROM; run tools/pandora-runtime-qualification/run.sh"]
    fn forced_arrivals_use_cop14_positions_and_source_standing_facing() {
        use room_core::{slice::MotionKey, Direction};
        let rom = owned_rom();
        let arrivals = forced_arrivals(rom.image()).unwrap();
        assert_eq!(arrivals.len(), 6);
        for ((key, frame), (map, position, facing)) in arrivals.iter().zip([
            (0x21, (136, 368), Direction::Down),
            (0x41, (136, 208), Direction::Up),
            (0x44, (392, 464), Direction::Up),
            (0x42, (136, 464), Direction::Up),
            (0x43, (392, 208), Direction::Up),
            (0x41, (136, 208), Direction::Up),
        ]) {
            assert!(matches!(key, MotionKey::Cue(_)));
            assert!(frame.reload);
            assert_eq!(frame.map_id, map);
            assert_eq!(frame.anchor.position, position);
            assert_eq!(frame.anchor.facing, facing);
        }
        let mut changed = rom.image().to_vec();
        changed[0x88ad53 & 0x3fffff] ^= 1;
        assert!(forced_arrivals(&changed).is_err());
        let mut changed = rom.image().to_vec();
        changed[0x848802 & 0x3fffff] ^= 1;
        assert!(forced_arrivals(&changed).is_err());
        let mut changed = rom.image().to_vec();
        changed[0x84a315 & 0x3fffff] ^= 1;
        assert!(forced_arrivals(&changed).is_err());
    }

    #[test]
    #[ignore = "owned ROM; run tools/pandora-runtime-qualification/run.sh"]
    fn room_variants_preserve_raw_pots_and_temporary_occupancy() {
        let rom = owned_rom();
        let nav = pandora_navigation::compile(rom.image()).unwrap();
        let objects = source_objects(rom.image(), &nav).unwrap();
        assert_eq!(
            objects
                .iter()
                .map(|o| (o.cell, o.raw, o.replacement))
                .collect::<Vec<_>>(),
            vec![
                (675, 0x18fa, 0xf8),
                (676, 0x18fb, 0xf8),
                (677, 0x18fa, 0xf8)
            ]
        );
        let rooms = compile_rooms(rom.image(), &nav, &objects).unwrap();
        assert_eq!(rooms.len(), 14);
        for (room, key) in rooms.iter().zip(CollisionKey::ALL) {
            assert_eq!(room.key, key);
            assert_eq!((room.room.width(), room.room.height()), key.dimensions());
            assert!(room.room.sample_halo().is_some());
        }
        for (key, door) in [
            (CollisionKey::CClosed, [0x1d80, 0xb81]),
            (CollisionKey::CDamaged, [0x1da7, 0xb81]),
            (CollisionKey::CReaction, [0x9cf6, 0xbacb]),
            (CollisionKey::COpen, [0x1cf6, 0x3acb]),
        ] {
            let r = &rooms[key as usize].room;
            assert_eq!([r.cells()[651], r.cells()[683]], door);
            // Core applies the authoritative wooden-door patch across shared loads.
            assert_eq!([r.cells()[616], r.cells()[648]], [0x1cf2, 0x1cf3]);
            for object in &objects {
                assert_eq!(r.cells()[usize::from(object.cell)], object.raw);
            }
        }
        assert_eq!(
            rooms[CollisionKey::Town as usize].room.cells(),
            nav.profile("a").unwrap().room().cells()
        );
        assert_eq!(
            rooms[CollisionKey::CellarE as usize].room.cells()[675],
            0x18fa
        ); // cache overlay, never pre-consumed initialization
    }
}
