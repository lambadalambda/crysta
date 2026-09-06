//! Finite record-zero carry presentation; no native animation clock or Y offset.
use super::{invalid, json, pandora::frame_key, Result, Value};
use assets::sprites::{PandoraCarryMotion, PandoraSprites};
use room_core::{
    pots::{Flight, Phase, PotState},
    Direction,
};

const KINDS: [(u16, u32); 2] = [(0x098a, 0x96_e1a6), (0x098f, 0x96_e1ab)];
const MOTIONS: [&str; 4] = ["lifting", "standing", "walking", "throwing"];

/// Read-only core projection, not a second action controller. Motion is the
/// explicitly queued cardinal input, NOT the walker's retained active direction.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CarryInput {
    pub phase: Phase,
    pub phase_tick: u8,
    pub facing: Direction,
    pub motion: Option<Direction>,
    pub position: (u16, u16),
    pub held_slot: Option<u16>,
    pub reserved_slot: Option<u16>,
    pub flight: Option<Flight>,
}
impl CarryInput {
    // Parent live wiring is deliberately outside this component.
    #[allow(dead_code)]
    pub(crate) fn from_state(p: &PotState, slots: (Option<u16>, Option<u16>)) -> Self {
        Self {
            phase: p.phase(),
            phase_tick: p.phase_tick(),
            facing: p.facing(),
            motion: p.walking().delayed_direction(),
            position: p.walking().position(),
            held_slot: slots.0,
            reserved_slot: slots.1,
            flight: p.flight(),
        }
    }
}

// Parent consumes these fields when live wiring is accepted.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct CarryPresentation {
    pub actor_key: String,
    /// Send as `state.carry`; `scene_phase` still emits only Ark and source residents.
    pub overlay: Value,
}

fn facing(direction: Direction) -> u8 {
    match direction {
        Direction::Down => 0,
        Direction::Up => 1,
        Direction::Left => 2,
        Direction::Right => 3,
    }
}
fn pose(key: &str) -> Option<Value> {
    let (motion, direction) = key.split_once(':')?;
    let direction = match direction {
        "0" => 0,
        "1" => 1,
        "2" => 2,
        "3" => 3,
        _ => return None,
    };
    let motion = match motion {
        "lifting" => PandoraCarryMotion::Lifting,
        "standing" => PandoraCarryMotion::Standing,
        "walking" => PandoraCarryMotion::Walking,
        "throwing" => PandoraCarryMotion::Throwing,
        _ => return None,
    };
    let p = PandoraSprites::carry_pose(motion, direction)?;
    Some(
        json!({"ark":frame_key(p.ark_art,p.ark_selector,0,p.ark_hflip),
        "pots":KINDS.map(|(slot,art)| (slot.to_string(),frame_key(art,p.pot_selector,0,p.pot_hflip)))
            .into_iter().collect::<std::collections::BTreeMap<_,_>>()}),
    )
}

/// Compile only after the ROM-authenticated Pandora atlas. Every allowed pair
/// refers to an existing precomposed source raster; no extra graphics are made.
pub(super) fn compile(frames: &serde_json::Map<String, Value>) -> Result<Value> {
    let mut poses = serde_json::Map::new();
    for motion in MOTIONS {
        for direction in 0..4 {
            let key = format!("{motion}:{direction}");
            let p = pose(&key).expect("finite source carry pose");
            for value in
                std::iter::once(&p["ark"]).chain(p["pots"].as_object().expect("pot kinds").values())
            {
                if !frames.contains_key(value.as_str().expect("source key")) {
                    return Err(invalid("missing source carry frame").into());
                }
            }
            poses.insert(key, p);
        }
    }
    let flight: std::collections::BTreeMap<_, _> = KINDS
        .map(|(slot, art)| (slot.to_string(), frame_key(art, 60, 0, false)))
        .into_iter()
        .collect();
    if flight.values().any(|key| !frames.contains_key(key)) {
        return Err(invalid("missing source flight frame").into());
    }
    Ok(
        json!({"map_id":12,"policy":"core-semantic-record0-not-native-timing",
        "priority":2,"tie_policy":"world-y-pot-after-ark","poses":poses,"flight":flight}),
    )
}

pub(super) fn select(map: u16, input: CarryInput) -> Result<Option<CarryPresentation>> {
    let CarryInput {
        phase,
        phase_tick: tick,
        facing: direction,
        motion,
        position,
        held_slot: hand,
        reserved_slot: reserved,
        flight,
    } = input;
    let valid_slot =
        |slot: Option<u16>| slot.is_none() || KINDS.iter().any(|(kind, _)| Some(*kind) == slot);
    let held = hand.is_some() && hand == reserved && flight.is_none();
    let valid = match phase {
        Phase::Empty => tick == 0 && hand.is_none() && reserved.is_none() && flight.is_none(),
        Phase::Lifting => tick < 23 && held && motion.is_none(),
        Phase::Held => tick == 0 && held,
        Phase::Throwing if tick < 18 => held && motion.is_none(),
        Phase::Throwing => {
            // Validate the core's admitted point; never manufacture a trajectory
            // when the authoritative sample is absent. Break clears reservation
            // before Ark's finite throw recovery finishes at tick32.
            let end = if position.0 == 184 { 22 } else { 27 };
            tick < 32
                && direction == Direction::Up
                && matches!(position, (136 | 184, 368))
                && motion.is_none()
                && hand.is_none()
                && if tick < end {
                    reserved.is_some()
                        && flight
                            == Some(Flight {
                                x: position.0,
                                y: 357 - 3 * u16::from(tick - 18),
                            })
                } else {
                    reserved.is_none() && flight.is_none()
                }
        }
    };
    if !valid || !valid_slot(hand) || !valid_slot(reserved) {
        return Err(invalid("invalid core carry phase/slots/flight").into());
    }
    if phase == Phase::Empty {
        return Ok(None);
    }
    if map != 12 {
        return Err(invalid("unsupported carry map").into());
    }
    let motion = match phase {
        Phase::Empty => unreachable!(),
        Phase::Lifting => "lifting",
        Phase::Throwing => "throwing",
        Phase::Held => {
            if motion.is_some() {
                "walking"
            } else {
                "standing"
            }
        }
    };
    let key = format!("{motion}:{}", facing(direction));
    let p = pose(&key).expect("typed source pose");
    Ok(Some(CarryPresentation {
        actor_key: p["ark"].as_str().expect("source key").into(),
        overlay: json!({"pose":key,"phase_tick":tick,"held_slot":hand,"reserved_slot":reserved,
            "flight":flight.map(|p|[p.x,p.y])}),
    }))
}

#[cfg(test)]
#[path = "carry_tests.rs"]
mod tests;
