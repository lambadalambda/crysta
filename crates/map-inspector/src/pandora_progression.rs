//! Offline ROM-to-PandoraData adapter. Never called by the live host.
use crate::{invalid, pandora_navigation, pandora_navigation::source_objects, sha256, Result};
use assets::{
    maps::{exits::ExitList, visual::pandora::PandoraBackground},
    text::pandora::{PandoraDialogue, DIRECT_INVOCATIONS},
};
use room_core::{
    pots::SourceObject,
    slice::{
        Anchor, BoxOpeningGate, CellPatch, CollisionKey, Cue, DataIdentity, Exit, ExitKey,
        GameData, GameState, Invocation, MapExits, MotionFrame, MotionKey, MotionPose, MotionSpec,
        NavigationSpec, PandoraData, PandoraText, Policy, ProfileRoom, RequestPages, ScenePhase,
        TownDoor, TownDoorSpec, Travel, TravelExit,
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
    let at = (address & 0x003f_ffff) as usize;
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

/// Compile the opt-in aggregate from the owned ROM, never from captured state.
///
/// # Errors
/// Rejects unauthenticated source data or any incompatible bounded core contract.
pub fn compile(rom: &rom::Rom) -> Result<GameData> {
    Ok(compile_profile(rom)?.0)
}

fn aggregate_identity(rom: &rom::Rom, base: &[u8], manifest: &Value) -> Result<DataIdentity> {
    let mut content = b"pandora-aggregate-v1:canonical-json,base-new-game-identity".to_vec();
    content.push(room_core::slice::PROFILE_VERSION);
    content.extend(rom::digests(base).sha256);
    content.extend(rom::digests(&serde_json::to_vec(manifest)?).sha256);
    Ok(DataIdentity {
        rom_sha256: rom.digests().sha256,
        content_sha256: rom::digests(&content).sha256,
    })
}

fn compile_profile(rom: &rom::Rom) -> Result<(GameData, Value)> {
    let nav = pandora_navigation::compile(rom.image())?;
    let base = crate::house_progression::compile(rom)?;
    // This freshly ROM-derived serialization binds the existing house identity.
    // It is hashed only, NEVER restored/used to initialize the enabled runtime.
    let base_identity = GameState::new_game(&base, Policy::SemanticPreview).snapshot();
    let (text, text_manifest) = compile_text(rom.image())?;
    let objects = source_objects(rom.image(), &nav)?;
    let rooms = compile_rooms(rom.image(), &nav, &objects)?;
    let contacts = compile_contacts(&nav)?;
    let opening_gate = BoxOpeningGate {
        raw_bounds: nav.contact().opening_bounds(),
    };
    let (mut motions, cue_manifest) = compile_cues(rom.image())?;
    let (navigation, travel_motions) = compile_navigation(rom.image(), &nav)?;
    motions.extend(travel_motions);
    let manifest = json!({
        "schema":1,
        "policy":"pandora-aggregate-v1:passive-frozen-source,completed-rest-contact,cell-scoped-delayed-materials,ordinary17-load17,stair-load-completion,graph-atomic-cue-v1,no-native-timing,no-vm",
        "source":{"navigation":nav.metadata(),"cues":cue_manifest},
        "text":text_manifest,
        "rooms":rooms.iter().map(|p| json!({"key":format!("{:?}",p.key),
            "width":p.room.width(),"height":p.room.height(),"halo":p.room.sample_halo(),"passive":true,
            "raw_sha256":sha256(&p.room.cells().iter().flat_map(|w| w.to_le_bytes()).collect::<Vec<_>>()),
            "materials":p.room.material_policy().iter().map(|r|json!({"bounds":r.bounds,"direction":r.direction.map(|d|format!("{d:?}")),"alias":format!("{:?}",r.alias)})).collect::<Vec<_>>()
        })).collect::<Vec<_>>(),
        "objects":objects.iter().map(|o|json!([o.cell,o.raw,o.replacement])).collect::<Vec<_>>(),
        "contacts":contacts.iter().map(|c|json!({"kind":format!("{:?}",c.kind),"trigger":anchor_manifest(c.trigger),"result":anchor_manifest(c.result)})).collect::<Vec<_>>(),
        "opening_gate":opening_gate.raw_bounds,"cellar_up_lanes":true,
        "motions":motions.iter().map(|m|json!({"key":format!("{:?}",m.key),"trigger":m.trigger.map(anchor_manifest),
            "frames":m.frames.iter().map(|f|json!({"map":f.map_id,"reload":f.reload,"scene":format!("{:?}",f.scene),"pose":match f.pose {
                MotionPose::Absolute(a)=>json!({"absolute":anchor_manifest(a)}),
                MotionPose::Preserve{facing}=>json!({"preserve":facing.map(|d|format!("{d:?}"))}),
            }})).collect::<Vec<_>>()
        })).collect::<Vec<_>>(),
        "navigation":{"maps":navigation.maps.iter().map(|m|json!({"map":m.map_id,"records":m.records.iter().map(|e|e.0).collect::<Vec<_>>()})).collect::<Vec<_>>(),
            "travels":navigation.travels.iter().map(|t|json!({"travel":format!("{:?}",t.travel),"map":t.exit.map_id,"index":t.exit.index})).collect::<Vec<_>>(),
            "doors":navigation.doors.iter().map(|d|json!({"door":format!("{:?}",d.door),"map":d.exit.map_id,"index":d.exit.index,
                "interaction":anchor_manifest(d.interaction),"patches":d.patches.map(|p|[p.cell,p.closed,p.open])})).collect::<Vec<_>>()}
    });
    let identity = aggregate_identity(rom, &base_identity, &manifest)?;
    let pandora = PandoraData::new(text, rooms, motions, contacts, opening_gate, objects, true)
        .map_err(|error| invalid(&format!("Pandora profile: {error}")))?
        .with_navigation(navigation)
        .map_err(|error| invalid(&format!("Pandora navigation: {error}")))?;
    Ok((
        base.with_pandora(pandora, identity)
            .map_err(|error| invalid(&format!("Pandora aggregate: {error}")))?,
        manifest,
    ))
}
fn anchor_manifest(anchor: Anchor) -> Value {
    json!({"position":anchor.position,"facing":format!("{:?}",anchor.facing)})
}

fn compile_navigation(
    image: &[u8],
    nav: &pandora_navigation::Navigation,
) -> Result<(NavigationSpec, Vec<MotionSpec>)> {
    use Travel::{CToCellar, ETo20, ResidentToTown, TownToHouse, TownToResident, TwentyToBox};
    let lists = [0x000a, 0x0013, 0x000c, 0x000d, 0x000e, 0x0020]
        .into_iter()
        .map(|map| Ok((map, ExitList::from_rom(image, map)?)))
        .collect::<Result<Vec<_>>>()?;
    let maps = lists
        .iter()
        .map(|(map, list)| MapExits {
            map_id: *map,
            records: list.records().iter().map(|r| Exit(*r.bytes())).collect(),
        })
        .collect();
    let mut travels = Vec::new();
    let mut motions = Vec::new();
    let mut doors = Vec::new();
    for (travel, map, site) in [
        (TownToResident, 0x000a, 0x0081_8d8f),
        (ResidentToTown, 0x0013, 0x0081_8eac),
        (TownToHouse, 0x000a, 0x0081_8d6b),
        (CToCellar, 0x000c, 0x0081_8df1),
        (ETo20, 0x000e, 0x0081_8e2f),
        (TwentyToBox, 0x0020, 0x0081_8fc1),
    ] {
        let (_, list) = lists
            .iter()
            .find(|(id, _)| *id == map)
            .ok_or_else(|| invalid("travel source map"))?;
        let (index, record) = list
            .records()
            .iter()
            .enumerate()
            .find(|(_, r)| r.source_range().start == site & 0x003f_ffff)
            .ok_or_else(|| invalid("travel source ordinal"))?;
        let selector = record.selector();
        let facing = match selector {
            5 => Direction::Down,
            6 | 14 => Direction::Up,
            _ => return Err(invalid("travel selector").into()),
        };
        let trigger = Anchor {
            position: (
                u16::from(record.x()) * 16 + 8,
                u16::from(record.y()) * 16 + 16,
            ),
            facing,
        };
        require(
            list.select(trigger.position.0 - 8, trigger.position.1 - 16) == Some(record),
            "ordered source travel witness",
        )?;
        let exit = ExitKey {
            map_id: map,
            index: u16::try_from(index)?,
        };
        travels.push(TravelExit { travel, exit });
        let frames = travel_frames(image, map, record, trigger)?;
        if matches!(travel, TownToResident | TownToHouse) {
            let (door, opened_name) = if travel == TownToResident {
                (TownDoor::North, "a-north-open")
            } else {
                (TownDoor::Home, "a-home-open")
            };
            let closed = nav
                .profile("a")
                .ok_or_else(|| invalid("Town source base"))?
                .room();
            let open = nav
                .profile(opened_name)
                .ok_or_else(|| invalid("Town source patch"))?
                .room();
            let lower = u16::from(record.y()) * closed.width() + u16::from(record.x());
            doors.push(TownDoorSpec {
                door,
                exit,
                interaction: Anchor {
                    position: (trigger.position.0, trigger.position.1 + 16),
                    facing,
                },
                patches: [lower - closed.width(), lower].map(|cell| CellPatch {
                    cell,
                    closed: closed.cells()[usize::from(cell)] & 0x7fff,
                    open: open.cells()[usize::from(cell)] & 0x7fff,
                }),
            });
        }
        motions.push(MotionSpec {
            key: MotionKey::Travel(travel),
            trigger: Some(trigger),
            frames,
        });
    }
    Ok((
        NavigationSpec {
            maps,
            travels,
            doors: doors.try_into().map_err(|_| invalid("Town door count"))?,
        },
        motions,
    ))
}

fn travel_frames(
    image: &[u8],
    map: u16,
    record: &assets::maps::exits::ExitRecord,
    trigger: Anchor,
) -> Result<Vec<MotionFrame>> {
    let selector = record.selector();
    let facing = trigger.facing;
    let raw = record.destination_position();
    let adjustment = 0x008d_8985 + u32::from(selector) * 4;
    let adjust = |value: u16, at, anchor_offset| -> Result<u16> {
        value
            .checked_add_signed(word(image, at)?.cast_signed())
            .and_then(|v| v.checked_add(anchor_offset))
            .ok_or_else(|| invalid("travel coordinate adjustment").into())
    };
    let loaded = (
        adjust(raw.0, adjustment, 8)?,
        adjust(raw.1, adjustment + 2, 16)?,
    );
    let destination = record.direct_destination()?;
    let scene = |id| match id {
        0x000a => ScenePhase::TownSource,
        0x0013 => ScenePhase::Resident13,
        0x000c => ScenePhase::CDeparted,
        0x000d => ScenePhase::House,
        0x000e => ScenePhase::CellarE,
        0x0020 => ScenePhase::Cellar20,
        _ => ScenePhase::BoxContact,
    };
    let mut frames = Vec::new();
    if selector == 14 {
        // Two explicit semantic boundaries: source-adjusted load, then completed
        // stair arrival (+14,+23). No ordinary walking or native duration claim.
        for (position, reload) in [(loaded, true), ((loaded.0 + 14, loaded.1 + 23), false)] {
            frames.push(MotionFrame {
                map_id: destination,
                pose: MotionPose::Absolute(Anchor { position, facing }),
                reload,
                scene: scene(destination),
            });
        }
    } else {
        for elapsed in 1..=35u16 {
            let (origin, distance) = if elapsed <= 17 {
                (trigger.position, elapsed)
            } else {
                (loaded, elapsed - 18)
            };
            let position = (
                origin.0,
                match facing {
                    Direction::Up => origin.1.checked_sub(distance),
                    _ => origin.1.checked_add(distance),
                }
                .ok_or_else(|| invalid("ordinary doorway position"))?,
            );
            let current_map = if elapsed <= 17 { map } else { destination };
            frames.push(MotionFrame {
                map_id: current_map,
                pose: MotionPose::Absolute(Anchor { position, facing }),
                reload: elapsed == 18,
                scene: scene(current_map),
            });
        }
    }
    Ok(frames)
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
    use room_core::{
        MaterialAlias::{ClosedDoorPartial5, StairOpen29, TownSolid25},
        MaterialRule,
    };
    use CollisionKey::{
        Box, BoxOpened, CClosed, CDamaged, COpen, CReaction, Cellar20, CellarE, Resident, Tour41,
        Tour42, Tour43, Tour44, Town,
    };
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
#[allow(clippy::too_many_lines)] // Keep six fixed source sites with their arrival checks.
fn forced_arrivals(image: &[u8]) -> Result<Vec<(MotionKey, MotionFrame)>> {
    use Invocation::{OpeningFourth, TourLeave41, TourLeave42, TourLeave43, TourLeave44};
    require(
        word(image, 0x0084_8800)? == 0x88ac && word(image, 0x0084_8802)? == 0x89d1,
        "forced arrival dispatch",
    )?;
    for (site, target, sequence) in [
        (0x0084_88e2, 0x0084_a2f3_u32, 0),
        (0x0084_8a07, 0x0084_a308, 1),
    ] {
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
    let mut arrivals = Vec::new();
    for (cue, site, map, mode, selector, scene) in [
        (
            Cue::BoxReload,
            0x0088_ad53,
            0x21,
            7,
            1,
            ScenePhase::BoxOpening,
        ),
        (
            Cue::Returned(OpeningFourth),
            0x0088_aeab,
            0x41,
            4,
            2,
            ScenePhase::Tour410,
        ),
        (
            Cue::Returned(TourLeave41),
            0x0089_d476,
            0x44,
            0,
            2,
            ScenePhase::Tour44,
        ),
        (
            Cue::Returned(TourLeave44),
            0x0089_d4b4,
            0x42,
            0,
            2,
            ScenePhase::Tour42,
        ),
        (
            Cue::Returned(TourLeave42),
            0x0089_d4d9,
            0x43,
            0,
            2,
            ScenePhase::Tour43,
        ),
        (
            Cue::Returned(TourLeave43),
            0x0089_d4fe,
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
    use Invocation::{
        BoxWarning, CApproach, CEntry, FirstHit, OpeningFirst, OpeningFourth, OpeningSecond,
        OpeningThird, ReactionFinal, ReactionLeft, ReactionRequest, ReactionRight, ReactionSpeaker,
        SecondHit, Tour42, Tour43, Tour44, TourFive, TourFour, TourIntro, TourLeave41, TourLeave42,
        TourLeave43, TourLeave44, TourOne, TourSix, TourThree, TourTwo,
    };
    use ScenePhase as S;
    // B certifies the named finite cooperative completion, not one native frame.
    // D reads that exact COPC1 operand as logical actor units; never a fallback.
    #[rustfmt::skip]
    let recipes = vec![
        (R(CEntry), 0xc, vec![B(S::CEntry, 0x0088_9afb)]),
        (R(CApproach), 0xc, vec![B(S::CChoice, 0x0088_9b11)]),
        (R(FirstHit), 0xc, vec![B(S::CFirstHit, 0x0088_abc6)]),
        (Cue::SecondHitPatched, 0xc, vec![B(S::CSecondHit, 0x0088_abee), D(S::CSecondHit, 0x0088_9b9b, 60)]),
        (R(SecondHit), 0xc, vec![D(S::CSecondHit, 0x0088_9ba5, 16), B(S::CSecondHit, 0x0088_a583)]),
        (Cue::ReactionColorMath, 0xc, vec![B(S::CColorMath, 0x0088_9d1f)]),
        (R(ReactionSpeaker), 0xc, vec![D(S::CReactionSpeaker, 0x0088_9bc8, 60)]),
        (Cue::ReactionColorReturn, 0xc, vec![B(S::CColorMath, 0x0088_9d4f), D(S::CReactionSpeaker, 0x0088_9bd9, 60), B(S::CReactionSpeaker, 0x0088_9bef)]),
        (R(ReactionRequest), 0xc, vec![B(S::CReactionSpeaker, 0x0088_a241)]),
        (R(ReactionRight), 0xc, vec![B(S::CReactionRight, 0x0088_a256)]),
        (R(ReactionLeft), 0xc, vec![B(S::CReactionLeft, 0x0088_9c10)]),
        (R(ReactionFinal), 0xc, vec![B(S::CReactionFinal, 0x0088_9c30)]),
        (R(BoxWarning), 0x21, vec![D(S::BoxContact, 0x0088_adbd, 32)]),
        (Cue::BoxAcquireControl, 0x21, vec![B(S::BoxContact, 0x0088_8ea6)]),
        (R(OpeningFirst), 0x21, vec![D(S::BoxOpeningCue, 0x0088_ae81, 60)]),
        (R(OpeningSecond), 0x21, vec![D(S::BoxOpening, 0x0088_ae8f, 30)]),
        (R(OpeningThird), 0x21, vec![D(S::BoxOpeningCue, 0x0088_ae9d, 30)]),
        (R(TourIntro), 0x41, vec![B(S::Tour410, 0x0089_d3ee)]),
        (R(TourOne), 0x41, vec![B(S::Tour411, 0x0089_d402)]),
        (R(TourTwo), 0x41, vec![B(S::Tour412, 0x0089_d416)]),
        (R(TourThree), 0x41, vec![B(S::Tour413, 0x0089_d42a)]),
        (R(TourFour), 0x41, vec![B(S::Tour414, 0x0089_d43e)]),
        (R(TourFive), 0x41, vec![B(S::Tour415, 0x0089_d452)]),
        (R(TourSix), 0x41, vec![B(S::Tour416, 0x0089_d466), D(S::Tour417, 0x0089_d46c, 32)]),
        (R(Tour44), 0x44, vec![D(S::Tour44, 0x0089_d4aa, 32)]),
        (R(Tour42), 0x42, vec![D(S::Tour42, 0x0089_d4cf, 32)]),
        (R(Tour43), 0x43, vec![D(S::Tour43, 0x0089_d4f4, 32)]),
        (Cue::BoxReload, 0x21, vec![L, D(S::BoxOpening, 0x0088_ae6c, 120), D(S::BoxOpening, 0x0088_ae73, 240)]),
        (R(OpeningFourth), 0x21, vec![D(S::BoxOpeningCue, 0x0088_aea7, 120), L, D(S::Tour410, 0x0089_d3dc, 60)]),
        (R(TourLeave41), 0x41, vec![L, D(S::Tour44, 0x0089_d4a0, 60)]),
        (R(TourLeave44), 0x44, vec![L, D(S::Tour42, 0x0089_d4c5, 60)]),
        (R(TourLeave42), 0x42, vec![L, D(S::Tour43, 0x0089_d4ea, 60)]),
        (R(TourLeave43), 0x43, vec![L, D(S::Tour410, 0x0089_d487, 60)]),
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
        source(image, 0x0084_88cd, 3)? == [0x9c, 0x7c, 9]
            && source(image, 0x0080_b828, 8)? == [0xad, 0x7c, 9, 0x89, 0x10, 8, 0xd0, 0x4b]
            && source(image, 0x0088_8ea6, 16)?
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
        (0x0088_9ae9, 0x0088_9c34),
        (0x0088_9cd8, 0x0088_9d53),
        (0x0088_a241, 0x0088_a266),
        (0x0088_a3d9, 0x0088_a400),
        (0x0088_a56d, 0x0088_a58b),
        (0x0088_ab96, 0x0088_ac12),
        (0x0088_ad2f, 0x0088_adcb),
        (0x0088_ae6c, 0x0088_aeb5),
        (0x0089_d2f4, 0x0089_d510),
        (0x0080_8a23, 0x0080_8a58),
        (0x0080_f7f3, 0x0080_f815),
        (0x0084_87ce, 0x0084_8804),
        (0x0084_88ac, 0x0084_88e8),
        (0x0084_89d1, 0x0084_8a0d),
        (0x0084_a2e9, 0x0084_a389),
        (0x0085_d735, 0x0085_d755),
        (0x0085_f8d1, 0x0085_f925),
        (0x0084_8000, 0x0084_803b),
        (0x0084_809b, 0x0084_80c8),
        (0x0080_b827, 0x0080_b885),
        (0x0088_8ea6, 0x0088_8eb6),
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
    fn navigation_retains_complete_lists_and_source_stair_boundaries() {
        let rom = owned_rom();
        let nav = pandora_navigation::compile(rom.image()).unwrap();
        let (spec, motions) = compile_navigation(rom.image(), &nav).unwrap();
        for (map, expected_id) in spec
            .maps
            .iter()
            .zip([0x000a, 0x0013, 0x000c, 0x000d, 0x000e, 0x0020])
        {
            assert_eq!(map.map_id, expected_id);
            let source = ExitList::from_rom(rom.image(), expected_id).unwrap();
            assert_eq!(
                map.records.iter().map(|e| e.0).collect::<Vec<_>>(),
                source
                    .records()
                    .iter()
                    .map(|e| *e.bytes())
                    .collect::<Vec<_>>()
            );
        }
        // Remote unsupported Town record extends beyond grid width; keep it verbatim.
        assert_eq!(
            spec.maps[0].records[8].0,
            [0, 0x3e, 0x50, 2, 3, 0, 0, 0x55, 0x10, 2, 0x10, 2]
        );
        for (motion, (travel, map, loaded, settled)) in motions[3..].iter().zip([
            (Travel::CToCellar, 0x000e, (138, 857), (152, 880)),
            (Travel::ETo20, 0x0020, (394, 857), (408, 880)),
            (Travel::TwentyToBox, 0x0021, (122, 105), (136, 128)),
        ]) {
            assert_eq!(motion.key, MotionKey::Travel(travel));
            assert_eq!(motion.frames.len(), 2);
            for (frame, (position, reload)) in
                motion.frames.iter().zip([(loaded, true), (settled, false)])
            {
                assert_eq!(frame.map_id, map);
                assert_eq!(frame.reload, reload);
                assert_eq!(
                    frame.pose,
                    MotionPose::Absolute(Anchor {
                        position,
                        facing: Direction::Up
                    })
                );
            }
        }
    }

    #[test]
    #[ignore = "owned ROM; run tools/pandora-runtime-qualification/run.sh"]
    fn aggregate_compiles_complete_source_navigation_and_repeatable_identity() {
        use room_core::slice::{GameState, Policy};
        let rom = owned_rom();
        let (data, manifest) = compile_profile(&rom).unwrap();
        assert!(data.pandora_enabled());
        assert_eq!(manifest["motions"].as_array().unwrap().len(), 39);
        assert_eq!(manifest["navigation"]["maps"].as_array().unwrap().len(), 6);
        assert_eq!(manifest["rooms"].as_array().unwrap().len(), 14);
        let snapshot = GameState::new_game(&data, Policy::SemanticPreview).snapshot();
        assert_eq!(snapshot.len(), 320);
        assert_eq!(
            GameState::restore(&data, &snapshot).unwrap().snapshot(),
            snapshot
        );
        let repeated = compile(&rom).unwrap();
        assert_eq!(
            GameState::new_game(&repeated, Policy::SemanticPreview).snapshot(),
            snapshot
        );
        let base = crate::house_progression::compile(&rom).unwrap();
        let base_snapshot = GameState::new_game(&base, Policy::SemanticPreview).snapshot();
        assert!(GameState::restore(&base, &snapshot).is_err());
        let identity = aggregate_identity(&rom, &base_snapshot, &manifest).unwrap();
        for section in [
            "rooms",
            "motions",
            "navigation",
            "contacts",
            "text",
            "source",
            "objects",
            "policy",
            "opening_gate",
            "cellar_up_lanes",
        ] {
            let mut changed = manifest.clone();
            changed[section] = Value::Null;
            assert_ne!(
                aggregate_identity(&rom, &base_snapshot, &changed).unwrap(),
                identity,
                "{section}"
            );
        }
        let mut changed_base = base_snapshot;
        changed_base[10] ^= 1;
        assert_ne!(
            aggregate_identity(&rom, &changed_base, &manifest).unwrap(),
            identity
        );
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
        changed[0x0088_adbd & 0x003f_ffff] ^= 1;
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
        changed[0x0088_ad53 & 0x003f_ffff] ^= 1;
        assert!(forced_arrivals(&changed).is_err());
        let mut changed = rom.image().to_vec();
        changed[0x0084_8802 & 0x003f_ffff] ^= 1;
        assert!(forced_arrivals(&changed).is_err());
        let mut changed = rom.image().to_vec();
        changed[0x0084_a315 & 0x003f_ffff] ^= 1;
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
