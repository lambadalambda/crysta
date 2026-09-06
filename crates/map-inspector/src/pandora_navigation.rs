//! ROM-only bounded Pandora navigation transport; no runtime state or event VM.
use assets::maps::{
    exits::ExitList,
    visual::{pandora::PandoraBackground, StaticBackground},
};
use room_core::{Direction, Room};
use serde_json::{json, Value};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn ensure(ok: bool, reason: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(std::io::Error::new(std::io::ErrorKind::InvalidData, reason).into())
    }
}

/// Collision classification, never a replacement for the retained source word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Material {
    Open,
    Solid,
    Partial,
}

/// Immutable bounded profile. Actor order here is source phase order, not targeting order.
#[derive(Debug)]
pub struct Profile {
    name: &'static str,
    map: u16,
    room: Room,
    actors: Vec<(u32, [u16; 2])>,
    exits: ExitList,
}
impl Profile {
    pub fn name(&self) -> &str {
        self.name
    }
    pub fn map(&self) -> u16 {
        self.map
    }
    pub fn room(&self) -> &Room {
        &self.room
    }
    pub fn exits(&self) -> &ExitList {
        &self.exits
    }
    pub fn actors(&self) -> &[(u32, [u16; 2])] {
        &self.actors
    }
    /// Samples are cells, not anchors. Direction is the delayed direction governing
    /// this collision tick, NOT newly submitted input. Unknown modes/materials fail.
    pub fn sample(
        &self,
        cell: (u16, u16),
        direction: Direction,
        old: bool,
        control: u16,
    ) -> Result<Material> {
        ensure(
            matches!(control, 0 | 0xa0 | 0x20),
            "unqualified navigation control",
        )?;
        let [left, top, right, bottom] = self.room.sample_halo().ok_or("missing admission")?;
        ensure(
            cell.0 >= left && cell.0 < right && cell.1 >= top && cell.1 < bottom,
            "sample outside admission",
        )?;
        let raw = self.room.cells()
            [usize::from(cell.1) * usize::from(self.room.width()) + usize::from(cell.0)];
        let kind = (raw >> 9) & 31;
        ensure(!old || !matches!(kind, 6 | 7), "old edge slope")?;
        if raw & 0x8000 != 0 {
            return Ok(Material::Solid);
        }
        let lane = direction == Direction::Up
            && match self.map {
                12 => cell == (11, 21),
                14 => cell == (6, 53),
                32 => cell == (22, 53),
                _ => false,
            };
        match kind {
            0 | 2 | 22 => Ok(Material::Open),
            12 | 14 => Ok(Material::Solid),
            16 => Ok(Material::Partial),
            25 if self.map == 10 => Ok(Material::Solid),
            5 if lane && self.map == 12 && raw == 0x0b81 => Ok(Material::Partial),
            29 if lane && raw == 0x3acb => Ok(Material::Open),
            _ => Err("unqualified navigation material".into()),
        }
    }
    #[cfg(test)]
    fn test(map: u16, halo: [u16; 4], cells: Vec<u16>) -> Self {
        Self {
            name: "test",
            map,
            room: Room::new_passive(2, 2, cells)
                .unwrap()
                .with_sample_halo(halo)
                .unwrap(),
            actors: vec![],
            exits: ExitList::from_rom(&vec![0; 0x18000 + 2 * usize::from(map) + 2], map).unwrap(),
        }
    }
}

/// Source-derived first-contact and independent polling rectangles. Bounds inclusive.
#[derive(Debug)]
pub struct ContactSpec {
    first: [u16; 4],
    opening: [u16; 4],
}
impl ContactSpec {
    pub fn first_bounds(&self) -> [u16; 4] {
        self.first
    }
    pub fn opening_bounds(&self) -> [u16; 4] {
        self.opening
    }
    /// Raw anchor predicate, polled after local1/local2; not an input edge.
    pub fn opening_ready(&self, (x, y): (u16, u16), local1: bool, local2: bool) -> bool {
        let [l, t, r, b] = self.opening;
        local1 && local2 && (l..=r).contains(&x) && (t..=b).contains(&y)
    }
    /// Geometry only. Caller must enforce grounded ordinary Down participant policy;
    /// native recoil is qualified only for the northern centerline approach.
    pub fn first_geometry(&self, (x, y): (u16, u16)) -> bool {
        let [l, t, r, b] = self.first;
        (l..=r).contains(&x) && (t..=b).contains(&y)
    }
}
fn contact_spec(image: &[u8], sprites: &assets::sprites::PandoraSprites) -> Result<ContactSpec> {
    let frames = sprites
        .get(0x83f984)
        .ok_or("box art")?
        .list(3)
        .ok_or("box list")?
        .frames();
    let outer = &frames[0].source_composition().source_bytes()[8..12];
    for frame in frames {
        ensure(
            &frame.source_composition().source_bytes()[8..12] == outer,
            "changing box contact geometry",
        )?;
    }
    let inner = bytes(image, 0xa4a556, 4)?;
    for at in [0x9ad492, 0x9ad4cd, 0x9ad50f, 0x9ad551, 0x9ad58c, 0x9ad5ce] {
        ensure(
            bytes(image, at, 4)? == inner,
            "changing Down receiving geometry",
        )?;
    }
    let spawn = bytes(image, 0x83928f, 4)?;
    let (x, y) = (i32::from(spawn[1]) * 16 + 8, i32::from(spawn[2]) * 16);
    let signed = |v: u8| i32::from(v as i8);
    let first = [
        x + signed(outer[0]) - signed(inner[0]) - signed(inner[1]),
        y + signed(outer[2]) - signed(inner[2]) - signed(inner[3]),
        x + signed(outer[0]) + signed(outer[1]) - signed(inner[0]),
        y + signed(outer[2]) + signed(outer[3]) - signed(inner[2]),
    ];
    expect(image, 0x88ad35, &[2, 13, 255])?;
    let operands = bytes(image, 0x88ad38, 4)?;
    expect(image, 0x8791c2, &[0xe9])?;
    let projection = i32::from(word(image, 0x8791c3)?);
    ensure(projection == 8, "ordinary anchor projection")?;
    let opening = [
        x + signed(operands[0]) * 16,
        y - 8 + signed(operands[1]) * 16 + projection,
        x + signed(operands[2]) * 16,
        y - 8 + signed(operands[3]) * 16 + projection,
    ];
    let convert = |a: [i32; 4]| -> Result<[u16; 4]> {
        Ok([
            u16::try_from(a[0])?,
            u16::try_from(a[1])?,
            u16::try_from(a[2])?,
            u16::try_from(a[3])?,
        ])
    };
    Ok(ContactSpec {
        first: convert(first)?,
        opening: convert(opening)?,
    })
}

/// Source-owned transport; parent runtime owns event/phase selection and timing.
#[derive(Debug)]
pub struct Navigation {
    profiles: Vec<Profile>,
    contact: ContactSpec,
    metadata: Value,
}
impl Navigation {
    pub fn profiles(&self) -> &[Profile] {
        &self.profiles
    }
    pub fn profile(&self, name: &str) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.name == name)
    }
    pub fn metadata(&self) -> &Value {
        &self.metadata
    }
    pub fn contact(&self) -> &ContactSpec {
        &self.contact
    }
}
fn bytes(image: &[u8], address: usize, length: usize) -> Result<&[u8]> {
    let start = address & 0x3fffff;
    image
        .get(start..start.checked_add(length).ok_or("source overflow")?)
        .ok_or_else(|| "truncated navigation source".into())
}
fn word(image: &[u8], address: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(bytes(image, address, 2)?.try_into()?))
}
fn expect(image: &[u8], address: usize, want: &[u8]) -> Result<()> {
    ensure(
        bytes(image, address, want.len())? == want,
        "changed navigation source operands",
    )
}
fn hash(bytes: &[u8]) -> String {
    rom::digests(bytes)
        .sha256
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn validate_dispatch(image: &[u8]) -> Result<()> {
    for base in [0x80d542, 0x80d8e8, 0x80dc60, 0x80dfdc] {
        for offset in [0, 64, 128, 192] {
            let t = base + offset;
            ensure(
                word(image, t + 50)? == word(image, t + 24)?,
                "type25 solid dispatch",
            )?;
            ensure(
                word(image, t + 10)? == word(image, t + 32)?,
                "type5 partial dispatch",
            )?;
            ensure(
                (word(image, t + 58)? == word(image, t)?) == (t != 0x80e09c),
                "directional type29 dispatch",
            )?;
        }
    }
    Ok(())
}

/// Actual ordered source edge cells. Position overflow/underflow fails closed.
pub fn sample_cells((x, y): (u16, u16), d: Direction) -> Result<Vec<(u16, u16)>> {
    let (u, v) = match d {
        Direction::Left | Direction::Up => (x.checked_sub(8), y.checked_sub(16)),
        Direction::Right => (x.checked_add(7), y.checked_sub(16)),
        Direction::Down => (x.checked_sub(8), y.checked_sub(1)),
    };
    let (u, v) = (
        u.ok_or("sample X arithmetic")?,
        v.ok_or("sample Y arithmetic")?,
    );
    let horizontal = matches!(d, Direction::Left | Direction::Right);
    let mut result = vec![(u / 16, v / 16)];
    if (if horizontal { v } else { u }) & 15 != 0 {
        result.push(if horizontal {
            (u / 16, v / 16 + 1)
        } else {
            (u / 16 + 1, v / 16)
        });
    }
    Ok(result)
}
/// Raw Ark anchors project to (x,y-8) at $8791A2 before these far/near probes.
/// Actor-first traversal must try both points within each candidate.
pub fn interaction_samples((x, y): (u16, u16), d: Direction) -> Result<[(u16, u16); 2]> {
    let y = y.checked_sub(8).ok_or("interaction projection Y")?;
    let p = |distance: i16| -> Result<(u16, u16)> {
        let (dx, dy) = match d {
            Direction::Up => (0, -distance),
            Direction::Down => (0, distance),
            Direction::Left => (-distance, 0),
            Direction::Right => (distance, 0),
        };
        Ok((
            x.checked_add_signed(dx).ok_or("interaction X")?,
            y.checked_add_signed(dy).ok_or("interaction Y")?,
        ))
    };
    Ok([p(16)?, p(8)?])
}
/// Bounded fresh map13 witness, not generic callback/policy dispatch.
pub fn resident13_witness(p: (u16, u16), d: Direction) -> bool {
    d == Direction::Up && p == (360, 144)
}

fn stamp(cells: &mut [u16], width: u16, position: [u16; 2]) -> Result<()> {
    let col = position[0].checked_sub(8).ok_or("stamp X")? / 16;
    let row = position[1].checked_sub(16).ok_or("stamp Y")? / 16;
    ensure(col < width, "stamp column")?;
    let cell = cells
        .get_mut(usize::from(row) * usize::from(width) + usize::from(col))
        .ok_or("stamp extent")?;
    *cell |= 0x8000;
    Ok(())
}
fn patched_word(attributes: &[u8], tile: u16) -> Result<u16> {
    Ok(tile
        | ((u16::from(*attributes.get(usize::from(tile)).ok_or("patch attribute")?) & 127) << 9))
}

/// Decode only ROM. Capture/provenance metadata is never a production argument.
pub fn compile(image: &[u8]) -> Result<Navigation> {
    ensure(
        hash(image) == "f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548",
        "navigation ROM authentication",
    )?;
    validate_dispatch(image)?;
    expect(image, 0x8d89bd, &[0xf2, 0xff, 0xe9, 0xff])?;
    expect(image, 0x88ad35, &[2, 13, 255, 255, 255, 1, 1, 0x2a, 0xad])?;
    expect(image, 0x88ad3e, &[2, 9, 1, 0x20, 2, 0x80, 0x2a, 0xad])?;
    let sprites = assets::sprites::PandoraSprites::from_rom(image)?;
    let contact = contact_spec(image, &sprites)?;
    let recipes = [
        ("a", 10, [21, 16, 36, 53], Some("town-source")),
        ("a-north-open", 10, [21, 16, 36, 53], Some("town-source")),
        ("a-home-open", 10, [21, 16, 36, 53], Some("town-source")),
        ("13", 19, [22, 7, 25, 16], Some("resident-13")),
        ("d-return", 13, [7, 32, 9, 46], None),
        ("c-direct", 12, [1, 20, 14, 32], Some("c-direct")),
        ("c-after-miss", 12, [1, 20, 14, 32], Some("c-direct")),
        ("c-held-fa", 12, [1, 20, 14, 32], Some("c-direct")),
        ("c-first-hit", 12, [1, 20, 14, 32], Some("c-direct")),
        ("c-held-fb", 12, [1, 20, 14, 32], Some("c-direct")),
        ("c-departed", 12, [1, 20, 14, 32], None),
        ("e", 14, [6, 52, 10, 55], None),
        ("20", 32, [22, 52, 26, 55], None),
        ("21", 33, [8, 7, 9, 25], None),
        ("21-contact", 33, [8, 7, 9, 25], None),
        ("41-control", 65, [6, 10, 9, 14], None),
    ];
    let mut profiles = Vec::new();
    let mut entries = Vec::new();
    for (name, map, halo, phase) in recipes {
        let house;
        let pandora;
        let bg = if matches!(map, 12 | 13) {
            house = StaticBackground::from_rom(image, map)?;
            &house
        } else {
            pandora = PandoraBackground::from_rom(image, map)?;
            pandora.background()
        };
        let width = u16::try_from(bg.layer().width())?;
        let height = u16::try_from(bg.layer().height())?;
        let attributes = bg.resources()[3].decoded();
        let mut cells: Vec<_> = bg
            .layer()
            .attributed_cells(attributes.try_into()?)
            .iter()
            .map(|c| c.raw())
            .collect();
        let mut actors = Vec::new();
        let mut actor_sources = Vec::new();
        if let Some(phase) = phase {
            for actor in sprites
                .phase(phase)
                .ok_or("missing source actor phase")?
                .actors()
            {
                let art = sprites.get(actor.art_id).ok_or("actor art")?;
                let list = art.list(actor.selector).ok_or("actor source list")?;
                for frame in list.frames() {
                    ensure(
                        frame.source_composition().source_bytes()[4..8] == [248, 16, 240, 16],
                        "unqualified actor stamp geometry",
                    )?;
                }
                stamp(&mut cells, width, actor.position)?;
                actors.push((actor.source_id, actor.position));
                actor_sources.push(json!({"source":actor.source_id,"position":actor.position,"position_source":actor.position_source,
                    "art":actor.art_id,"selector":actor.selector,"geometry":[-8,16,-16,16]}));
            }
        }
        if map == 13 {
            let spawn = bytes(image, 0x838cb4, 4)?;
            let p = [u16::from(spawn[1]) * 16 + 8, u16::from(spawn[2]) * 16];
            stamp(&mut cells, width, p)?;
            actors.push((0x838cb4, p));
            actor_sources.push(json!({"source":0x838cb4,"position":p,"position_source":0x838cb4,
                "geometry":[-8,16,-16,16],"qualification":"existing house-backgrounds D source stamp; no AI"}));
        }
        if name == "21-contact" {
            let record = bytes(image, 0x83928f, 4)?;
            stamp(
                &mut cells,
                width,
                [u16::from(record[1]) * 16 + 8, u16::from(record[2]) * 16],
            )?;
        }
        if matches!(name, "a-north-open" | "a-home-open") {
            let (col, row) = if name == "a-north-open" {
                (29, 17)
            } else {
                (31, 46)
            };
            // Same wooden-door source COP44 final selectors; location is source exit.
            for (site, y) in [(0x879819, row), (0x879829, row - 1)] {
                expect(image, site, &[2, 0x44, 0, 0])?;
                cells[y * usize::from(width) + col] =
                    patched_word(attributes, word(image, site + 4)? & 511)?;
            }
        }
        let removed: &[usize] = match name {
            "c-after-miss" => &[5],
            "c-held-fa" | "c-first-hit" => &[3, 5],
            "c-held-fb" | "c-departed" | "e" | "20" => &[3, 4, 5],
            _ => &[],
        };
        if !removed.is_empty() {
            expect(image, 0x8796d7, &[0xa9])?;
            ensure(
                word(image, 0x96e1a6)? == 0 && word(image, 0x96e1ab)? == 0,
                "source pot fallback records",
            )?;
            for &col in removed {
                ensure(
                    matches!(cells[21 * 32 + col], 0x18fa | 0x18fb),
                    "source route pot cell",
                )?;
                cells[21 * 32 + col] = patched_word(attributes, word(image, 0x8796d8)?)?;
            }
        }
        if matches!(
            name,
            "c-first-hit" | "c-held-fb" | "c-departed" | "e" | "20"
        ) {
            let sites = if matches!(name, "c-first-hit" | "c-held-fb") {
                [0x88aba6, 0x88abac]
            } else {
                [0x88abd8, 0x88abde]
            };
            let origin = bytes(image, 0x838c32, 4)?;
            for site in sites {
                expect(image, site, &[2, 0x44])?;
                let op = bytes(image, site + 2, 4)?;
                let x = i16::from(origin[1]) * 16 + i16::from(op[0] as i8) * 16;
                let y = (i16::from(origin[2]) - 1) * 16 + i16::from(op[1] as i8) * 16;
                let idx = usize::try_from(y / 16)? * usize::from(width) + usize::try_from(x / 16)?;
                cells[idx] = patched_word(attributes, word(image, site + 4)? & 511)?;
            }
            if name == "c-departed" {
                let departure = sprites
                    .motions()
                    .iter()
                    .find(|m| m.actor == 0x838c1e && m.removes_actor)
                    .ok_or("source C final departure")?;
                stamp(&mut cells, width, departure.to)?;
            }
        }
        let exits = ExitList::from_rom(image, map)?;
        let raw: Vec<_> = cells.iter().flat_map(|w| w.to_le_bytes()).collect();
        entries.push(json!({"name":name,"map":map,"width":width,"height":height,"halo":halo,
            "grid_sha256":hash(&raw),"actors":actor_sources,
            "layer_source":bg.layer().source_range().start,
            "exits":exits.records().iter().map(|e|json!({"source":0x800000|e.source_range().start,"raw":e.bytes()})).collect::<Vec<_>>() }));
        profiles.push(Profile {
            name,
            map,
            room: Room::new_passive(width, height, cells)?.with_sample_halo(halo)?,
            actors,
            exits,
        });
    }
    // Native settled anchors are evidence for the separately tagged source selectors.
    let mut transfers = Vec::new();
    for (map, source, settled) in [
        (10, 0x818d6b, [120, 719]),
        (10, 0x818d8f, [392, 207]),
        (19, 0x818eac, [472, 305]),
        (13, 0x818dfe, [504, 769]),
        (13, 0x818e0a, [120, 447]),
        (12, 0x818df1, [152, 880]),
        (14, 0x818e2f, [408, 880]),
        (32, 0x818fc1, [136, 128]),
    ] {
        let list = ExitList::from_rom(image, map)?;
        let e = list
            .records()
            .iter()
            .find(|r| r.source_range().start == (source & 0x3fffff))
            .ok_or("required ordered exit absent")?;
        let raw = e.destination_position();
        let adjustment = 0x8d8985 + usize::from(e.selector()) * 4;
        let initial = [
            i32::from(raw.0) + i32::from(word(image, adjustment)? as i16) + 8,
            i32::from(raw.1) + i32::from(word(image, adjustment + 2)? as i16) + 16,
        ];
        let expected = match e.selector() {
            5 => [initial[0], initial[1] + 17],
            6 => [initial[0], initial[1] - 17],
            14 => [initial[0] + 14, initial[1] + 23],
            _ => return Err("unqualified transfer selector".into()),
        };
        ensure(expected == settled, "source-derived transfer endpoint")?;
        transfers.push(json!({"map":map,"source":source,"destination":e.direct_destination()?,"mode":e.transition_mode(),"selector":e.selector(),
            "raw_position":[raw.0,raw.1],"settled":settled,"policy":if e.selector()==14 {"stair-forced-no-native-pacing"}else{"ordinary-17-load-17"}}));
    }
    let mut forced = Vec::new();
    for source in [0x88ad53, 0x88aeab, 0x89d476, 0x89d4b4, 0x89d4d9, 0x89d4fe] {
        expect(image, source, &[2, 0x14])?;
        forced.push(json!({"source":source,"map":word(image,source+2)?,"mode":bytes(image,source+4,1)?[0],
            "selector":bytes(image,source+5,1)?[0],"raw_position":[word(image,source+6)?,word(image,source+8)?]}));
    }
    let mut ranges = Vec::new();
    for (start, end) in [
        (0x80d542, 0x80d642),
        (0x80d8e8, 0x80d9e8),
        (0x80dc60, 0x80dd60),
        (0x80dfdc, 0x80e0dc),
        (0x80e1df, 0x80e32e),
        (0x868a7d, 0x868ad8),
        (0x869145, 0x86916e),
        (0x8d8aed, 0x8d8b14),
        (0x8791a2, 0x8791ca),
        (0x87c783, 0x87c7f1),
        (0x87923f, 0x87941c),
        (0x88acfa, 0x88adcb),
        (0x8087c2, 0x808820),
        (0x85d30c, 0x85d399),
        (0x85d648, 0x85d757),
        (0x85f63e, 0x85f680),
        (0x85f78e, 0x85f856),
        (0x85f8d1, 0x85f925),
        (0x80c8d0, 0x80ca55),
        (0x80cad4, 0x80caf0),
        (0xa4a556, 0xa4a55a),
        (0x9ad48a, 0x9ad5d2),
        (0x8796bf, 0x87973d),
        (0x88aba6, 0x88abe4),
        (0x80bc2f, 0x80bc43),
        (0x80be8e, 0x80bf0e),
        (0x86baef, 0x86bb49),
        (0x8d8797, 0x8d89c9),
    ] {
        ranges.push(json!({"start":start,"end":end,"sha256":hash(bytes(image,start,end-start)?)}));
    }
    let metadata = json!({"schema":1,"rom_sha256":hash(image),"policy":"pandora-navigation-v1:source-frozen,passive,cell-scoped-up-stairs,no-vm,no-native-pacing",
        "profiles":entries,"transfers":transfers,"forced":forced,"source_ranges":ranges,
        "first_contact":{"bounds":contact.first,"inclusive":true,"callback":0x88ad69,"geometry_policy":"ordinary grounded Down; centerline north recoil endpoint only; queued callback before recoil"},
        "box_opening":{"bounds":contact.opening,"inclusive":true,"requires":[1,2],"input_edge":false},
        "lifecycle":"same first-layer pointer retains patches/pots; scene occupancy rebuilt; different pointer reconstructs; local0..31/counter reset"});
    Ok(Navigation {
        profiles,
        contact,
        metadata,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn samples_are_edges_not_radius_and_preserve_order() {
        assert_eq!(
            sample_cells((360, 144), Direction::Up).unwrap(),
            vec![(22, 8)]
        );
        assert_eq!(
            sample_cells((361, 145), Direction::Up).unwrap(),
            vec![(22, 8), (23, 8)]
        );
        assert_eq!(
            sample_cells((360, 144), Direction::Down).unwrap(),
            vec![(22, 8)]
        );
        assert_eq!(
            sample_cells((360, 145), Direction::Right).unwrap(),
            vec![(22, 8), (22, 9)]
        );
        assert!(sample_cells((7, 15), Direction::Up).is_err());
    }
    #[test]
    fn source_interaction_is_inclusive_and_far_first() {
        assert_eq!(
            interaction_samples((360, 144), Direction::Up).unwrap(),
            [(360, 120), (360, 128)]
        );
        assert!(resident13_witness((360, 144), Direction::Up));
        assert!(!resident13_witness((360, 145), Direction::Up));
        assert!(!resident13_witness((360, 144), Direction::Down));
        let contact = ContactSpec {
            first: [123, 370, 149, 400],
            opening: [120, 368, 152, 400],
        };
        // Y145 still hits the source rectangle via far121; witness restriction is policy.
        assert_eq!(
            interaction_samples((360, 145), Direction::Up).unwrap()[0],
            (360, 121)
        );
        assert!(contact.opening_ready((120, 368), true, true));
        assert!(contact.opening_ready((152, 400), true, true));
        for p in [(119, 368), (153, 400), (136, 367), (136, 401)] {
            assert!(!contact.opening_ready(p, true, true));
        }
        assert!(!contact.opening_ready((136, 368), true, false));
        assert!(!contact.opening_ready((136, 368), false, true));
    }
    #[test]
    fn classifications_fail_closed_and_do_not_normalize_raw_grid() {
        let p = Profile::test(10, [0, 0, 2, 2], vec![25 << 9, 29 << 9, 0x8000 | 6 << 9, 0]);
        assert_eq!(
            p.sample((0, 0), Direction::Up, false, 0).unwrap(),
            Material::Solid
        );
        assert!(p.sample((1, 0), Direction::Up, false, 0).is_err());
        assert!(p.sample((0, 1), Direction::Up, true, 0).is_err());
        assert_eq!(
            p.sample((0, 1), Direction::Up, false, 0).unwrap(),
            Material::Solid
        );
        assert!(p.sample((2, 0), Direction::Up, false, 0).is_err());
        assert!(p.sample((1, 1), Direction::Up, false, 0x50).is_err());
        assert_eq!(p.room.cells()[0], 25 << 9);
    }
    // Geometric witness only: no input cadence, nudge or native actor scheduler.
    fn connected(p: &Profile, from: (u16, u16), to: (u16, u16)) -> bool {
        use std::collections::{HashSet, VecDeque};
        let mut seen = HashSet::from([from]);
        let mut queue = VecDeque::from([from]);
        while let Some(at) = queue.pop_front() {
            if at == to {
                return true;
            }
            for (d, dx, dy) in [
                (Direction::Up, 0, -1),
                (Direction::Down, 0, 1),
                (Direction::Left, -1, 0),
                (Direction::Right, 1, 0),
            ] {
                let (Some(x), Some(y)) = (at.0.checked_add_signed(dx), at.1.checked_add_signed(dy))
                else {
                    continue;
                };
                if seen.contains(&(x, y)) {
                    continue;
                }
                let Ok(old) = sample_cells(at, d) else {
                    continue;
                };
                let Ok(new) = sample_cells((x, y), d) else {
                    continue;
                };
                if old.iter().all(|c| p.sample(*c, d, true, 0).is_ok())
                    && new
                        .iter()
                        .all(|c| matches!(p.sample(*c, d, false, 0), Ok(Material::Open)))
                {
                    seen.insert((x, y));
                    queue.push_back((x, y));
                }
            }
        }
        false
    }
    #[test]
    #[ignore = "requires PANDORA_ROM, run by the standalone qualification harness"]
    fn continuous_source_geometry_connects_required_anchors_without_unknown_samples() {
        let image = std::fs::read(std::env::var("PANDORA_ROM").unwrap()).unwrap();
        let data = compile(&image).unwrap();
        for (name, from, to) in [
            ("a", (538, 815), (472, 304)),
            ("a-north-open", (472, 304), (472, 288)),
            ("13", (392, 207), (360, 144)),
            ("13", (360, 144), (392, 208)),
            ("a", (472, 305), (504, 768)),
            ("a-home-open", (504, 768), (504, 752)),
            ("d-return", (120, 719), (120, 608)),
            ("c-direct", (120, 447), (104, 352)),
            ("c-direct", (104, 352), (136, 368)),
            ("c-direct", (136, 368), (40, 352)),
            ("c-direct", (40, 352), (184, 368)),
            ("c-first-hit", (184, 368), (88, 352)),
            ("c-departed", (184, 368), (184, 352)),
            ("e", (152, 880), (104, 864)),
            ("20", (408, 880), (360, 864)),
            ("21", (136, 128), (136, 360)),
            ("41-control", (136, 208), (120, 192)),
        ] {
            assert!(
                connected(data.profile(name).unwrap(), from, to),
                "{name} {from:?}->{to:?}"
            );
        }
    }
    #[test]
    #[ignore = "requires PANDORA_ROM, run by the standalone qualification harness"]
    fn owned_source_compile_and_mutation_controls() {
        let path = std::env::var("PANDORA_ROM").expect("test harness supplies owned ROM");
        let image = std::fs::read(path).unwrap();
        let data = compile(&image).unwrap();
        assert_eq!(data.profiles.len(), 16);
        assert_eq!(data.contact().first_bounds(), [123, 370, 149, 400]);
        assert_eq!(data.contact().opening_bounds(), [120, 368, 152, 400]);
        for point in [(123, 370), (149, 400), (136, 370)] {
            assert!(data.contact().first_geometry(point));
        }
        for point in [(122, 370), (150, 400), (136, 369), (136, 401)] {
            assert!(!data.contact().first_geometry(point));
        }
        let p = data.profile("13").unwrap();
        assert_eq!(p.room.cells()[7 * 64 + 22], 0x8007);
        assert_eq!(
            data.profile("c-direct").unwrap().room.cells()[22 * 32 + 9],
            0x8003
        );
        let e = data.profile("e").unwrap();
        assert_eq!(e.room.cells()[21 * 32 + 11], 0x3acb);
        assert_eq!(
            e.sample((6, 53), Direction::Up, false, 0).unwrap(),
            Material::Open
        );
        assert!(e.sample((6, 53), Direction::Left, false, 0).is_err());
        assert!(e.sample((11, 21), Direction::Up, false, 0).is_err());

        let grid = |name: &str| data.profile(name).unwrap().room.cells();
        let delta = |a: &[u16], b: &[u16]| {
            a.iter()
                .zip(b)
                .enumerate()
                .filter_map(|(i, (&a, &b))| (a != b).then_some((i, a, b)))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            delta(grid("a"), grid("a-north-open")),
            vec![(16 * 64 + 29, 0x1cf2, 0x1cf6), (17 * 64 + 29, 0x1cf3, 0xf7)]
        );
        assert_eq!(
            delta(grid("a"), grid("a-home-open")),
            vec![(45 * 64 + 31, 0x1cf2, 0x1cf6), (46 * 64 + 31, 0x1cf3, 0xf7)]
        );
        assert_eq!(
            delta(grid("c-direct"), grid("c-after-miss")),
            vec![(21 * 32 + 5, 0x18fa, 0xf8)]
        );
        assert_eq!(
            delta(grid("c-after-miss"), grid("c-held-fa")),
            vec![(21 * 32 + 3, 0x18fa, 0xf8)]
        );
        assert_eq!(
            delta(grid("c-held-fa"), grid("c-first-hit")),
            vec![(20 * 32 + 11, 0x1d80, 0x1da7)]
        );
        assert_eq!(
            delta(grid("c-first-hit"), grid("c-held-fb")),
            vec![(21 * 32 + 4, 0x18fb, 0xf8)]
        );
        assert_eq!(
            delta(grid("c-departed"), grid("e")),
            vec![(31 * 32 + 7, 0x9ce8, 0x1ce8)]
        );
        assert_eq!(grid("e"), grid("20"));
        assert_eq!(
            delta(grid("21"), grid("21-contact")),
            vec![(23 * 16 + 8, 0x12b, 0x812b)]
        );
        let base = StaticBackground::from_rom(&image, 12).unwrap();
        let raw: Vec<_> = base
            .layer()
            .attributed_cells(base.resources()[3].decoded().try_into().unwrap())
            .iter()
            .map(|c| c.raw())
            .collect();
        assert_eq!(
            delta(&raw, grid("e")),
            vec![
                (20 * 32 + 11, 0x1d80, 0x1cf6),
                (21 * 32 + 3, 0x18fa, 0xf8),
                (21 * 32 + 4, 0x18fb, 0xf8),
                (21 * 32 + 5, 0x18fa, 0xf8),
                (21 * 32 + 11, 0xb81, 0x3acb)
            ]
        );
        assert_eq!(
            delta(&raw, grid("d-return")),
            vec![(41 * 32 + 4, 0x2, 0x8002)]
        );
        let sprites = assets::sprites::PandoraSprites::from_rom(&image).unwrap();
        let mut operands = image.clone();
        operands[0x8_ad38] = 0xfe; // lower X: -2 source cells
        assert_eq!(
            contact_spec(&operands, &sprites).unwrap().opening_bounds(),
            [104, 368, 152, 400]
        );
        operands = image.clone();
        operands[0x1a_d492] ^= 1; // one receiving frame differs
        assert!(contact_spec(&operands, &sprites).is_err());
        assert!(bytes(&[0; 3], 0x800002, 2).is_err());
        assert_eq!(patched_word(&[0, 14, 128], 1).unwrap(), 0x1c01);
        assert_eq!(patched_word(&[0, 14, 128], 2).unwrap(), 2);

        let mut changed = image.clone();
        changed[0xd542 + 25 * 2] ^= 1;
        assert!(validate_dispatch(&changed).is_err());
        changed = image.clone();
        changed[0x3_8ebe + 1] ^= 1;
        assert!(compile(&changed).is_err());
        assert!(compile(&image[..100]).is_err());
    }
}
