//! The pots of a room, driven by the world's pad.
//!
//! Lifting, carrying, the throw and its flight are [`room_core::pots`]'s
//! component, by the native rules (`docs/pots.md`). The world owns ordinary
//! walking between actions and hands it to the component on an A press it
//! admits, until the pot has broken; it strikes the hittable actors a
//! flight's box overlaps.

use super::{Step, World, WorldError};
use crate::actors::Actor;
use crate::audio::Audio;
use assets::sprites::PandoraCarryMotion;
use room_core::pots::{Admission, Input, Output, Phase, PotState, Sound, SourceObject};
use room_core::{AnimationState, Direction, Room};

/// A source pot's word.
const POT_WORDS: [u16; 2] = [0x18FA, 0x18FB];
/// The tile a lifted pot's cell takes.
const LIFTED_TILE: u16 = 0x00F8;
/// The carry record `$0988` names for an `$FA` pot; `$FB` has `$098F`.
const FA_SLOT: u16 = 0x098A;
/// Port 3's sounds: the lift (`$84:BE6D`), the release (`$84:C6E5`) and
/// the break (`$84:C7A9`).
const LIFT_SOUND: u8 = 0x11;
const RELEASE_SOUND: u8 = 0x12;
const BREAK_SOUND: u8 = 0x13;
/// A flight strikes an actor whose box (x±8, y±8) overlaps its own,
/// edges included (`$85:D281`, `$85:F835`).
const REACH: u16 = 16;

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
    /// The actors the flight has struck, once each, and those whose
    /// callback runs next frame (`$85:D281` strikes; the target's
    /// `COP 65` callback runs a frame later).
    struck: Vec<usize>,
    pending: Vec<usize>,
}

impl Pots {
    /// The source pots of a freshly built room, if it has any.
    pub(super) fn at_entry(cells: &[u16]) -> Option<Self> {
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
        (!objects.is_empty()).then_some(Self {
            objects,
            state: None,
            owned: false,
            collision: None,
            throw_room: None,
            struck: Vec::new(),
            pending: Vec::new(),
        })
    }
}

/// A pot the player holds or has thrown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CarriedPot {
    /// Its tile, `$FA` or `$FB`.
    pub tile: u16,
    /// Its sprite's art (`$96:E1A6` for `$FA`, `$96:E1AB` for `$FB`), for
    /// [`assets::sprites::PandoraSprites::get`]: the Japanese addresses,
    /// the art's keys in either revision.
    pub art: u32,
    /// Where it flies, once released: its ground point and its height
    /// above it (`$0999`, negative up); `None` while in hand.
    pub flight: Option<(u16, u16, i16)>,
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
            flight: state
                .flight()
                .zip(state.flight_height())
                .map(|(flight, height)| (flight.x, flight.y, height)),
        })
    }

    /// Ark's carry pose, from the lift until the throw's recovery ends.
    #[must_use]
    pub fn carry(&self) -> Option<Carry> {
        // An item held up: the lift, then its stand (`$84:BEA2`).
        if let Some(presentation) = self.globals.presentation.filter(|p| p.age > 0) {
            let lifting = presentation.age <= super::PRESENTATION_LIFT;
            return Some(Carry {
                motion: if lifting {
                    PandoraCarryMotion::Lifting
                } else {
                    PandoraCarryMotion::Standing
                },
                facing: self.facing as u8,
                tick: u8::try_from(presentation.age - 1).unwrap_or(u8::MAX),
            });
        }
        // A Magirock lifted (`$84:DDFA`).
        if let Some(pickup) = self.pickup {
            let tick = pickup.frame().saturating_sub(1);
            return Some(Carry {
                motion: if tick < super::PRESENTATION_LIFT {
                    PandoraCarryMotion::Lifting
                } else {
                    PandoraCarryMotion::Standing
                },
                facing: self.facing as u8,
                tick: u8::try_from(tick).unwrap_or(u8::MAX),
            });
        }
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
            facing: state.facing() as u8,
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
        // A script that takes the pad stops Ark at once, even while the pot
        // he threw still flies (the door's callback does).
        if self.globals.input_mask & super::PAD_DIRECTIONS != 0 {
            if let Some(state) = &mut pots.state {
                let (x, y) = state.position();
                let _ = state.rebase(room_core::WalkingState::new(x, y), state.facing());
            }
        }
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
        // Talking wins over a lift (`$87:C783` before `$87:C7F1`).
        if !pots.owned && (!lift || self.arrival.is_some() || self.faces_resident()) {
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
            match state.step_over(&admission, &self.base.room, input) {
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
        let phase = state.phase();
        let busy = phase == Phase::Throwing || state.flight().is_some();
        pots.throw_room = match (busy, pots.throw_room.take()) {
            (true, Some(room)) => Some(room),
            (true, None) => pots.collision.as_ref().map(|(_, room)| room.clone()),
            (false, _) => None,
        };
        // A flight goes on after Ark's control returns; the component walks
        // him until it breaks.
        pots.owned = phase != Phase::Empty || input.action || state.flight().is_some();
        pots.state = Some(state);
        let before = self.position();
        self.walking = *state.walking();
        self.facing = state.facing();
        if !pots.owned {
            self.animation = AnimationState::standing(self.facing);
        } else if output.movement.is_some() {
            self.animation.advance(self.walking.active_direction());
        }
        sounds(&mut self.globals.audio, &output);
        if let Some(object) = output.consumed_cell {
            let width = self.base.width;
            self.globals.patches.push((
                object.cell % width,
                object.cell / width,
                object.replacement,
            ));
        }
        self.strike_under_flight(pots, &output);
        if self.position() == before {
            return Ok(Some(Step::Stayed));
        }
        self.leave_carrying(pots)?;
        Ok(Some(Step::Walked))
    }

    /// An exit under a carry step: Ark drops the pot (`$84:C5CB`, sound
    /// `$12`, unbroken) and leaves.
    fn leave_carrying(&mut self, pots: &mut Pots) -> Result<(), WorldError> {
        self.take_exit()?;
        if !self.in_transition() {
            return Ok(());
        }
        if let Some(state) = &mut pots.state {
            if state.drop_held() {
                self.globals.audio.sound_port3(RELEASE_SOUND);
                pots.owned = false;
            }
        }
        Ok(())
    }

    /// Strikes the hittable actors a flight's box overlaps (x±8, y±8 each,
    /// edges included), once a flight each; their callback runs a frame
    /// later, as the native `COP 65` does.
    fn strike_under_flight(&mut self, pots: &mut Pots, output: &Output) {
        for index in std::mem::take(&mut pots.pending) {
            if let Some(actor) = self.actors.get_mut(index) {
                actor.strike();
            }
        }
        if output.held_changed == Some(None) {
            pots.struck.clear();
        }
        let Some(flight) = output.flight else {
            return;
        };
        for (index, actor) in self.actors.iter().enumerate() {
            let (x, y) = actor.position;
            let overlaps = flight.x.abs_diff(x) <= REACH && flight.y.abs_diff(y) <= REACH;
            if overlaps && actor.hittable() && !pots.struck.contains(&index) {
                pots.struck.push(index);
                pots.pending.push(index);
            }
        }
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

/// A step's sounds: the lift, the release and the break.
fn sounds(audio: &mut Audio, output: &Output) {
    if output.consumed_cell.is_some() {
        audio.sound_port3(LIFT_SOUND);
    }
    match output.sound {
        Some(Sound::Release) => audio.sound_port3(RELEASE_SOUND),
        Some(Sound::Break) => audio.sound_port3(BREAK_SOUND),
        None => {}
    }
}
