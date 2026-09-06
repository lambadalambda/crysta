//! Offline ROM-to-PandoraData adapter. Never called by the live host.
use crate::{invalid, pandora_navigation, pandora_navigation::source_objects, sha256, Result};
use assets::{
    maps::visual::pandora::PandoraBackground,
    text::pandora::{PandoraDialogue, DIRECT_INVOCATIONS},
};
use room_core::{
    pots::SourceObject,
    slice::{
        Anchor, CollisionKey, Cue, Invocation, MotionFrame, MotionKey, MotionPose, MotionSpec,
        PandoraText, ProfileRoom, RequestPages, ScenePhase,
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
        use room_core::{MaterialAlias::*, MaterialRule};
        let lane = |alias, x, y| MaterialRule {
            bounds: [x, y, x + 1, y + 1],
            direction: Some(Direction::Up),
            alias,
        };
        // Navigation::compile authenticated the ROM and all sixteen dispatch tables.
        // Policies admit only those source map/cell aliases; raw words stay intact.
        let policy = match key {
            Town => vec![MaterialRule {
                bounds: halo,
                direction: None,
                alias: TownSolid25,
            }],
            CClosed | CDamaged | CReaction | COpen => {
                vec![lane(ClosedDoorPartial5, 11, 21), lane(StairOpen29, 11, 21)]
            }
            CellarE => vec![lane(StairOpen29, 6, 53)],
            Cellar20 => vec![lane(StairOpen29, 22, 53)],
            _ => vec![],
        };
        rooms.push(ProfileRoom {
            key,
            room: room_core::Room::new_passive(width, height, cells)?
                .with_sample_halo(halo)?
                .with_material_policy(policy)?,
        });
    }
    Ok(rooms)
}

fn compile_contacts(
    nav: &pandora_navigation::Navigation,
) -> Result<[room_core::slice::ContactSpec; 2]> {
    use room_core::slice::{ContactKind, ContactSpec};
    let resident = Anchor {
        position: (360, 144),
        facing: Direction::Up,
    };
    let first = Anchor {
        position: (136, 370),
        facing: Direction::Down,
    };
    require(
        pandora_navigation::resident13_witness(resident.position, resident.facing),
        "qualified resident interaction witness",
    )?;
    require(
        nav.contact().first_geometry(first.position)
            && nav.contact().first_bounds() == [123, 370, 149, 400]
            && nav.contact().opening_bounds() == [120, 368, 152, 400],
        "qualified box geometry",
    )?;
    Ok([
        ContactSpec {
            kind: ContactKind::Resident,
            trigger: resident,
            result: resident,
        },
        ContactSpec {
            kind: ContactKind::BoxWarning,
            trigger: first,
            // This witness represents COMPLETED northern recoil/rest, including
            // $8488CD STZ $097C, not merely visiting Y359 during a recoil.
            result: Anchor {
                position: (136, 359),
                facing: Direction::Down,
            },
        },
    ])
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
                pose: MotionPose::Absolute(Anchor {
                    position: (add(site + 6, 8)?, add(site + 8, 16)?),
                    facing: if selector == 1 {
                        Direction::Down
                    } else {
                        Direction::Up
                    },
                }),
                reload: true,
                scene,
            },
        ));
    }
    Ok(arrivals)
}

// Compiler-only fixed recipes. These are expanded into immutable samples here;
// no source address, recipe instruction or branch program enters room-core.
enum CueSegment {
    Boundary(ScenePhase, u32),
    Delay(ScenePhase, u32, u16),
    Reload,
}
fn cue_recipes() -> Vec<(Cue, u16, Vec<CueSegment>)> {
    use Cue::Returned as R;
    use CueSegment::{Boundary as B, Delay as D, Reload as L};
    use Invocation::*;
    use ScenePhase as S;
    // B certifies the named finite cooperative completion, not one native frame.
    // D reads that exact COPC1 operand as logical actor units; never a fallback.
    #[rustfmt::skip]
    let recipes = vec![
        (R(CEntry), 0xc, vec![B(S::CEntry, 0x889afb)]),
        (R(CApproach), 0xc, vec![B(S::CChoice, 0x889b11)]),
        (R(FirstHit), 0xc, vec![B(S::CFirstHit, 0x88abc6)]),
        (Cue::SecondHitPatched, 0xc, vec![B(S::CSecondHit, 0x88abee), D(S::CSecondHit, 0x889b9b, 60)]),
        (R(SecondHit), 0xc, vec![D(S::CSecondHit, 0x889ba5, 16), B(S::CSecondHit, 0x88a583)]),
        (Cue::ReactionColorMath, 0xc, vec![B(S::CColorMath, 0x889d1f)]),
        (R(ReactionSpeaker), 0xc, vec![D(S::CReactionSpeaker, 0x889bc8, 60)]),
        (Cue::ReactionColorReturn, 0xc, vec![B(S::CColorMath, 0x889d4f), D(S::CReactionSpeaker, 0x889bd9, 60), B(S::CReactionSpeaker, 0x889bef)]),
        (R(ReactionRequest), 0xc, vec![B(S::CReactionSpeaker, 0x88a241)]),
        (R(ReactionRight), 0xc, vec![B(S::CReactionRight, 0x88a256)]),
        (R(ReactionLeft), 0xc, vec![B(S::CReactionLeft, 0x889c10)]),
        (R(ReactionFinal), 0xc, vec![B(S::CReactionFinal, 0x889c30)]),
        (R(BoxWarning), 0x21, vec![D(S::BoxContact, 0x88adbd, 32)]),
        (Cue::BoxAcquireControl, 0x21, vec![B(S::BoxContact, 0x888ea6)]),
        (R(OpeningFirst), 0x21, vec![D(S::BoxOpeningCue, 0x88ae81, 60)]),
        (R(OpeningSecond), 0x21, vec![D(S::BoxOpening, 0x88ae8f, 30)]),
        (R(OpeningThird), 0x21, vec![D(S::BoxOpeningCue, 0x88ae9d, 30)]),
        (R(TourIntro), 0x41, vec![B(S::Tour410, 0x89d3ee)]),
        (R(TourOne), 0x41, vec![B(S::Tour411, 0x89d402)]),
        (R(TourTwo), 0x41, vec![B(S::Tour412, 0x89d416)]),
        (R(TourThree), 0x41, vec![B(S::Tour413, 0x89d42a)]),
        (R(TourFour), 0x41, vec![B(S::Tour414, 0x89d43e)]),
        (R(TourFive), 0x41, vec![B(S::Tour415, 0x89d452)]),
        (R(TourSix), 0x41, vec![B(S::Tour416, 0x89d466), D(S::Tour417, 0x89d46c, 32)]),
        (R(Tour44), 0x44, vec![D(S::Tour44, 0x89d4aa, 32)]),
        (R(Tour42), 0x42, vec![D(S::Tour42, 0x89d4cf, 32)]),
        (R(Tour43), 0x43, vec![D(S::Tour43, 0x89d4f4, 32)]),
        (Cue::BoxReload, 0x21, vec![L, D(S::BoxOpening, 0x88ae6c, 120), D(S::BoxOpening, 0x88ae73, 240)]),
        (R(OpeningFourth), 0x21, vec![D(S::BoxOpeningCue, 0x88aea7, 120), L, D(S::Tour410, 0x89d3dc, 60)]),
        (R(TourLeave41), 0x41, vec![L, D(S::Tour44, 0x89d4a0, 60)]),
        (R(TourLeave44), 0x44, vec![L, D(S::Tour42, 0x89d4c5, 60)]),
        (R(TourLeave42), 0x42, vec![L, D(S::Tour43, 0x89d4ea, 60)]),
        (R(TourLeave43), 0x43, vec![L, D(S::Tour410, 0x89d487, 60)]),
    ];
    recipes
}

fn compile_cues(image: &[u8]) -> Result<(Vec<MotionSpec>, Value)> {
    require(
        sha256(image) == "f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548",
        "cue ROM authentication",
    )?;
    // Readiness comes from completed northern recoil/rest and the closed ordinary
    // control subset, NOT this downstream handoff pose or gate proximity.
    require(
        source(image, 0x8488cd, 3)? == [0x9c, 0x7c, 9]
            && source(image, 0x80b828, 8)? == [0xad, 0x7c, 9, 0x89, 0x10, 8, 0xd0, 0x4b]
            && source(image, 0x888ea6, 16)?
                == [
                    2, 0x84, 0, 0, 0, 2, 0x8e, 2, 0xcb, 1, 0xc1, 0x87, 0x84, 2, 0xbc, 0x6b,
                ],
        "completed-rest COPDF and stationary Down handoff",
    )?;
    let arrivals = forced_arrivals(image)?;
    let mut motions = Vec::new();
    let mut evidence = Vec::new();
    for (cue, mut map, segments) in cue_recipes() {
        let key = MotionKey::Cue(cue);
        let mut frames = Vec::new();
        let mut steps = Vec::new();
        for segment in segments {
            let (scene, site, count, kind) = match segment {
                CueSegment::Reload => {
                    let (_, frame) = arrivals
                        .iter()
                        .find(|(k, _)| *k == key)
                        .ok_or_else(|| invalid("missing source forced arrival"))?;
                    frames.push(*frame);
                    map = frame.map_id;
                    steps.push(json!({"kind":"reload","map":map}));
                    continue;
                }
                CueSegment::Boundary(scene, site) => (scene, site, 1, "semantic-completion"),
                CueSegment::Delay(scene, site, expected) => {
                    require(
                        source(image, site, 2)? == [2, 0xc1] && word(image, site + 2)? == expected,
                        "source COPC1 cue delay",
                    )?;
                    (scene, site, expected, "logical-actor-delay")
                }
            };
            steps.push(json!({"kind":kind,"site":site,"units":count,"scene":format!("{scene:?}")}));
            frames.extend(std::iter::repeat_n(
                MotionFrame {
                    map_id: map,
                    pose: MotionPose::Preserve {
                        facing: (cue == Cue::BoxAcquireControl).then_some(Direction::Down),
                    },
                    reload: false,
                    scene,
                },
                usize::from(count),
            ));
        }
        evidence.push(json!({"cue":format!("{cue:?}"),"segments":steps}));
        motions.push(MotionSpec {
            key,
            trigger: None,
            frames,
        });
    }
    let mut ranges = Vec::new();
    for (start, end) in [
        (0x889ae9, 0x889c34),
        (0x889cd8, 0x889d53),
        (0x88a241, 0x88a266),
        (0x88a3d9, 0x88a400),
        (0x88a56d, 0x88a58b),
        (0x88ab96, 0x88ac12),
        (0x88ad2f, 0x88adcb),
        (0x88ae6c, 0x88aeb5),
        (0x89d2f4, 0x89d510),
        (0x808a23, 0x808a58),
        (0x80f7f3, 0x80f815),
        (0x8487ce, 0x848804),
        (0x8488ac, 0x8488e8),
        (0x8489d1, 0x848a0d),
        (0x84a2e9, 0x84a389),
        (0x85d735, 0x85d755),
        (0x85f8d1, 0x85f925),
        (0x848000, 0x84803b),
        (0x84809b, 0x8480c8),
        (0x80b827, 0x80b885),
        (0x888ea6, 0x888eb6),
    ] {
        ranges.push(json!({"start":start,"end":end,"sha256":sha256(source(image, start, (end-start) as usize)?)}));
    }
    Ok((
        motions,
        json!({"schema":1,"recipes":evidence,"source_ranges":ranges,
        "policy":"cue-v1:preserve-player,source-copc1-logical-units,finite-cooperative-boundaries,graph-atomic-locals-and-grants,completed-rest-copdf,no-native-timing,no-default-delays"}),
    ))
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
    fn cue_catalog_is_complete_preserving_and_has_no_default_delays() {
        use Invocation::*;
        let rom = owned_rom();
        let (motions, evidence) = compile_cues(rom.image()).unwrap();
        let expected = [
            (Cue::Returned(CEntry), 1),
            (Cue::Returned(CApproach), 1),
            (Cue::Returned(FirstHit), 1),
            (Cue::SecondHitPatched, 61),
            (Cue::Returned(SecondHit), 17),
            (Cue::ReactionColorMath, 1),
            (Cue::Returned(ReactionSpeaker), 60),
            (Cue::ReactionColorReturn, 62),
            (Cue::Returned(ReactionRequest), 1),
            (Cue::Returned(ReactionRight), 1),
            (Cue::Returned(ReactionLeft), 1),
            (Cue::Returned(ReactionFinal), 1),
            (Cue::Returned(BoxWarning), 32),
            (Cue::BoxAcquireControl, 1),
            (Cue::Returned(OpeningFirst), 60),
            (Cue::Returned(OpeningSecond), 30),
            (Cue::Returned(OpeningThird), 30),
            (Cue::Returned(TourIntro), 1),
            (Cue::Returned(TourOne), 1),
            (Cue::Returned(TourTwo), 1),
            (Cue::Returned(TourThree), 1),
            (Cue::Returned(TourFour), 1),
            (Cue::Returned(TourFive), 1),
            (Cue::Returned(TourSix), 33),
            (Cue::Returned(Tour44), 32),
            (Cue::Returned(Tour42), 32),
            (Cue::Returned(Tour43), 32),
            (Cue::BoxReload, 361),
            (Cue::Returned(OpeningFourth), 181),
            (Cue::Returned(TourLeave41), 61),
            (Cue::Returned(TourLeave44), 61),
            (Cue::Returned(TourLeave42), 61),
            (Cue::Returned(TourLeave43), 61),
        ];
        assert_eq!(motions.len(), expected.len());
        assert_eq!(evidence["recipes"].as_array().unwrap().len(), 33);
        let arrivals = forced_arrivals(rom.image()).unwrap();
        for (motion, (cue, length)) in motions.iter().zip(expected) {
            assert_eq!(motion.key, MotionKey::Cue(cue));
            assert_eq!(motion.frames.len(), length);
            assert_eq!(motion.trigger, None);
            let reloads: Vec<_> = motion.frames.iter().filter(|f| f.reload).collect();
            let arrival = arrivals.iter().find(|(key, _)| *key == motion.key);
            assert_eq!(reloads, arrival.map(|(_, f)| vec![f]).unwrap_or_default());
            for frame in motion.frames.iter().filter(|f| !f.reload) {
                assert_eq!(
                    frame.pose,
                    MotionPose::Preserve {
                        facing: (cue == Cue::BoxAcquireControl).then_some(Direction::Down)
                    }
                );
            }
        }
        let nav = pandora_navigation::compile(rom.image()).unwrap();
        let objects = source_objects(rom.image(), &nav).unwrap();
        let rooms = compile_rooms(rom.image(), &nav, &objects).unwrap();
        room_core::slice::PandoraData::new(
            compile_text(rom.image()).unwrap().0,
            rooms,
            motions,
            compile_contacts(&nav).unwrap(),
            room_core::slice::BoxOpeningGate {
                raw_bounds: nav.contact().opening_bounds(),
            },
            objects,
            true,
        )
        .unwrap();
        let mut changed = rom.image().to_vec();
        changed[0x88adbd & 0x3fffff] ^= 1;
        assert!(compile_cues(&changed).is_err());
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
            assert_eq!(
                frame.pose,
                MotionPose::Absolute(Anchor { position, facing })
            );
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
    fn source_contacts_and_raw_rooms_satisfy_delivered_constructor() {
        use room_core::slice::{BoxOpeningGate, ContactKind, PandoraData};
        let rom = owned_rom();
        let nav = pandora_navigation::compile(rom.image()).unwrap();
        let contacts = compile_contacts(&nav).unwrap();
        assert_eq!(contacts[0].kind, ContactKind::Resident);
        assert_eq!(
            contacts[0].trigger,
            Anchor {
                position: (360, 144),
                facing: Direction::Up
            }
        );
        assert_eq!(contacts[0].result, contacts[0].trigger);
        assert_eq!(contacts[1].kind, ContactKind::BoxWarning);
        assert_eq!(
            contacts[1].trigger,
            Anchor {
                position: (136, 370),
                facing: Direction::Down
            }
        );
        assert_eq!(
            contacts[1].result,
            Anchor {
                position: (136, 359),
                facing: Direction::Down
            }
        );
        let objects = source_objects(rom.image(), &nav).unwrap();
        let rooms = compile_rooms(rom.image(), &nav, &objects).unwrap();
        // Constructor compatibility only: no incomplete motion set is executed.
        PandoraData::new(
            compile_text(rom.image()).unwrap().0,
            rooms,
            vec![],
            contacts,
            BoxOpeningGate {
                raw_bounds: nav.contact().opening_bounds(),
            },
            objects,
            true,
        )
        .unwrap();
    }

    #[test]
    #[ignore = "owned ROM; run tools/pandora-runtime-qualification/run.sh"]
    fn room_policies_are_scoped_to_source_map_halo_and_delayed_direction() {
        use room_core::{MaterialAlias::*, MaterialRule};
        let rom = owned_rom();
        let nav = pandora_navigation::compile(rom.image()).unwrap();
        let objects = source_objects(rom.image(), &nav).unwrap();
        for profile in compile_rooms(rom.image(), &nav, &objects).unwrap() {
            let cell = |alias, x, y| MaterialRule {
                bounds: [x, y, x + 1, y + 1],
                direction: Some(Direction::Up),
                alias,
            };
            let expected = match profile.key {
                CollisionKey::Town => vec![MaterialRule {
                    bounds: nav.profile("a").unwrap().room().sample_halo().unwrap(),
                    direction: None,
                    alias: TownSolid25,
                }],
                CollisionKey::CClosed
                | CollisionKey::CDamaged
                | CollisionKey::CReaction
                | CollisionKey::COpen => {
                    vec![cell(ClosedDoorPartial5, 11, 21), cell(StairOpen29, 11, 21)]
                }
                CollisionKey::CellarE => vec![cell(StairOpen29, 6, 53)],
                CollisionKey::Cellar20 => vec![cell(StairOpen29, 22, 53)],
                _ => vec![],
            };
            assert_eq!(profile.room.material_policy(), expected);
        }
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
