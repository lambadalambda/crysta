//! The cellar pots in C, driven by the world's pad.
//!
//! Lifting, carrying, the throw and its door contact are
//! [`room_core::pots`]'s native-qualified component, unchanged: it admits only
//! the native lift poses and the two Up lanes from (136,368) and (184,368)
//! (`docs/pandora-pots.md`). The world owns ordinary walking between actions
//! and hands it to the component on an A press it admits.

use super::{Step, World, WorldError};
use crate::actors::Actor;
use crate::audio::Audio;
use assets::sprites::PandoraCarryMotion;
use room_core::pots::{Admission, Input, Output, Phase, PotState, SourceObject};
use room_core::{AnimationState, Direction, Room};

/// The map whose pots the component is qualified for.
const CELLAR: u16 = 0x000C;
/// A source pot's word.
const POT_WORDS: [u16; 2] = [0x18FA, 0x18FB];
/// The tile a lifted pot's cell takes.
const LIFTED_TILE: u16 = 0x00F8;
/// The carry record `$0988` names for an `$FA` pot; `$FB` has `$098F`.
const FA_SLOT: u16 = 0x098A;
/// Port 3's sounds: the lift (`$84:BE6D`), the break (`$84:C6E5`), and
/// the door's hit after it (`$84:C7A9`).
const LIFT_SOUND: u8 = 0x11;
const BREAK_SOUND: u8 = 0x12;
const HIT_SOUND: u8 = 0x13;

/// The pots of a room visit.
#[derive(Clone)]
pub(super) struct Pots {
    /// Source pots present at entry, sorted by cell.
    objects: Vec<SourceObject>,
    /// Created at the first admitted lift; keeps the visit's consumed cells.
    state: Option<PotState>,
    /// Whether the component moves the player this frame.
    owned: bool,
    /// The live cells and the collision built from them, with lifted pots
    /// back at their source words; rebuilt when the cells change.
    collision: Option<(Vec<u16>, Room)>,
    /// The collision a throw started with; the door patches may not change
    /// its lane (`docs/pandora-pots.md`).
    throw_room: Option<Room>,
    /// Whether the pot in flight has hit the door.
    hit: bool,
}

impl Pots {
    /// The source pots of a freshly built room, when it is the cellar.
    pub(super) fn at_entry(map: u16, cells: &[u16]) -> Option<Self> {
        let objects: Vec<SourceObject> = cells
            .iter()
            .enumerate()
            .filter(|(_, raw)| POT_WORDS.contains(raw))
            .filter_map(|(cell, &raw)| {
                Some(SourceObject {
                    cell: u16::try_from(cell).ok()?,
                    raw,
                    replacement: LIFTED_TILE,
                })
            })
            .collect();
        (map == CELLAR && !objects.is_empty()).then_some(Self {
            objects,
            state: None,
            owned: false,
            collision: None,
            throw_room: None,
            hit: false,
        })
    }
}

/// A pot the player holds or has thrown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CarriedPot {
    /// Its tile, `$FA` or `$FB`.
    pub tile: u16,
    /// Its sprite's art (`$96:E1A6` for `$FA`, `$96:E1AB` for `$FB`), for
    /// [`assets::sprites::PandoraSprites::get`].
    pub art: u32,
    /// Where it flies, once released; `None` while in hand.
    pub flight: Option<(u16, u16)>,
}

/// Ark's part in a pot action: the carry pose to draw and how far into
/// its phase he is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Carry {
    /// Lifting, standing or walking with the pot, or throwing it.
    pub motion: PandoraCarryMotion,
    /// Facing, 0 Down, 1 Up, 2 Left, 3 Right, as
    /// [`assets::sprites::PandoraSprites::carry_pose`] takes it.
    pub facing: u8,
    /// Frames into the lift or the throw; 0 while held.
    pub tick: u8,
}

impl World<'_> {
    /// The pot the player holds or has thrown, until it breaks.
    #[must_use]
    pub fn pot(&self) -> Option<CarriedPot> {
        let pots = self.pots.as_ref()?;
        let state = pots.state?;
        let admission = self.admission(&pots.objects, &self.room.room);
        let slot = state.reserved_slot_in(&admission)?;
        let (tile, art) = if slot == FA_SLOT {
            (0xFA, 0x96_E1A6)
        } else {
            (0xFB, 0x96_E1AB)
        };
        Some(CarriedPot {
            tile,
            art,
            flight: state.flight().map(|flight| (flight.x, flight.y)),
        })
    }

    /// Ark's carry pose, from the lift until the throw's recovery ends.
    #[must_use]
    pub fn carry(&self) -> Option<Carry> {
        let state = self.pots.as_ref()?.state?;
        let motion = match state.phase() {
            Phase::Empty => return None,
            Phase::Lifting => PandoraCarryMotion::Lifting,
            Phase::Throwing => PandoraCarryMotion::Throwing,
            // The queued step, not the walker's retained direction: Ark
            // stands the frame the pad is released.
            Phase::Held if state.walking().delayed_direction().is_some() => {
                PandoraCarryMotion::Walking
            }
            Phase::Held => PandoraCarryMotion::Standing,
        };
        Some(Carry {
            motion,
            facing: match state.facing() {
                Direction::Down => 0,
                Direction::Up => 1,
                Direction::Left => 2,
                Direction::Right => 3,
            },
            tick: state.phase_tick(),
        })
    }

    /// One frame of a pot action, when the component owns the player or `lift`
    /// (an A press nothing else takes) starts one. Returns the walking step
    /// when it did.
    pub(super) fn pot_frame(
        &mut self,
        direction: Option<Direction>,
        lift: bool,
    ) -> Result<Option<Step>, WorldError> {
        let Some(mut pots) = self.pots.take() else {
            return Ok(None);
        };
        let step = self.pot_step(&mut pots, direction, lift);
        self.pots = Some(pots);
        let Some(step) = step? else {
            return Ok(None);
        };
        self.run_actors()?;
        Ok(Some(step))
    }

    fn pot_step(
        &mut self,
        pots: &mut Pots,
        direction: Option<Direction>,
        lift: bool,
    ) -> Result<Option<Step>, WorldError> {
        if !pots.owned && (!lift || self.arrival.is_some()) {
            return Ok(None);
        }
        self.refresh_collision(pots)?;
        let Some(room) = pots
            .throw_room
            .as_ref()
            .or(pots.collision.as_ref().map(|(_, room)| room))
        else {
            return Ok(None);
        };
        let admission = self.admission(&pots.objects, room);
        let mut state = match pots.state {
            Some(state) => state,
            None => match PotState::new(&admission, self.walking, self.facing) {
                Ok(state) => state,
                Err(_) => return Ok(None),
            },
        };
        let tries: &[Input] = if pots.owned {
            &[
                Input {
                    direction,
                    action: lift,
                },
                Input {
                    direction,
                    action: false,
                },
                Input::default(),
            ]
        } else if state.rebase(self.walking, self.facing).is_ok() {
            &[Input {
                direction,
                action: true,
            }]
        } else {
            &[]
        };
        let mut refusal = None;
        let mut done = None;
        for &input in tries {
            match state.step(&admission, input) {
                Ok(output) => {
                    done = Some((input, output));
                    break;
                }
                Err(error) => refusal = Some(error),
            }
        }
        let Some((input, output)) = done else {
            // An A press that is not a lift goes on to talking and doorways.
            return match refusal {
                Some(error) if pots.owned => Err(WorldError::Pot(error)),
                _ => Ok(None),
            };
        };
        let flying = pots.state.and_then(|state| state.flight()).is_some();
        let phase = state.phase();
        pots.throw_room = match (phase, pots.throw_room.take()) {
            (Phase::Throwing, Some(room)) => Some(room),
            (Phase::Throwing, None) => pots.collision.as_ref().map(|(_, room)| room.clone()),
            _ => None,
        };
        pots.owned = phase != Phase::Empty || input.action;
        pots.state = Some(state);
        let before = self.position();
        self.walking = *state.walking();
        self.facing = state.facing();
        if !pots.owned {
            self.animation = AnimationState::standing(self.facing);
        } else if output.movement.is_some() {
            self.animation.advance(self.walking.active_direction());
        }
        let broke = flying && state.flight().is_none();
        sounds(&mut self.globals.audio, &mut pots.hit, &output, broke);
        if let Some(object) = output.consumed_cell {
            let width = self.base.width;
            self.globals.patches.push((
                object.cell % width,
                object.cell / width,
                object.replacement,
            ));
        }
        if let Some(flight) = output.flight.filter(|_| output.door_hit) {
            let column = flight.x.saturating_sub(8) / 16;
            for (resident, actor) in self.residents.iter().zip(&mut self.actors) {
                if actor.hittable() && resident.collision_cell().0 == column {
                    actor.strike();
                }
            }
        }
        Ok(Some(if self.position() == before {
            Step::Stayed
        } else {
            Step::Walked
        }))
    }

    fn admission<'r>(&self, objects: &'r [SourceObject], room: &'r Room) -> Admission<'r> {
        Admission {
            room,
            objects,
            cellar_up_lanes: true,
            door_hit_enabled: self.actors.iter().any(Actor::hittable),
        }
    }

    /// Rebuilds the cached collision when the live cells changed: the live
    /// room with lifted pots back at their source words, as the component
    /// overlays its consumed cells itself.
    fn refresh_collision(&self, pots: &mut Pots) -> Result<(), WorldError> {
        let live = self.room.room.cells();
        if pots
            .collision
            .as_ref()
            .is_some_and(|(cells, _)| cells == live)
        {
            return Ok(());
        }
        let mut cells = live.to_vec();
        if let Some(state) = pots.state {
            let admission = self.admission(&pots.objects, &self.room.room);
            for object in &pots.objects {
                if state.consumed_in(&admission, object.cell) {
                    cells[usize::from(object.cell)] = object.raw;
                }
            }
        }
        let room = self.room.with_cells(cells)?.room;
        pots.collision = Some((live.to_vec(), room));
        Ok(())
    }
}

/// A step's sounds: the lift, and at the break the break and, when the pot
/// hit the door, the door's hit -- natively four frames later, here next in
/// line.
fn sounds(audio: &mut Audio, hit: &mut bool, output: &Output, broke: bool) {
    if output.consumed_cell.is_some() {
        audio.sound_port3(LIFT_SOUND);
    }
    *hit |= output.door_hit;
    if broke {
        audio.sound_port3(BREAK_SOUND);
        if std::mem::take(hit) {
            audio.flush();
            audio.sound_port3(HIT_SOUND);
        }
    }
}
