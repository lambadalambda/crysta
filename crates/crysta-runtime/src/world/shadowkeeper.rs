//! Shadowkeeper, tower 5's guardian on `$123` (`docs/tower-five.md` §1,
//! §5.5), as a reduced fight in Rust: the body's script (`$93:D876`) is
//! replaced. The intro (`$8F:8005`, its native code modelled in
//! `actors/routines.rs`) and the end controller (`$90:A3AE`) run as they
//! are.
//!
//! - At Ark's y < 272 (`$8F:804A`) the intro holds him, the camera climbs
//!   to the body, the torches light up; then flag `$001`, music 5: the
//!   fight begins.
//! - The body is an enemy of its descriptor's profile (`$3C`): it takes no
//!   hits before the fight. At its first life's end it takes a second of
//!   100 (`docs/tower-five.md` §5.5), then explodes; the enemies' count
//!   falls to 0 and the end controller goes on.
//!
//! Not modelled: the darkness, the head's window and wisps, the claws, the
//! tail and the shots (the body hurts by its own attack box only).
//! Tracked: `meta/issues/shadowkeeper-scripts.md`.

use super::World;

/// The body's script, Japanese and European.
const BODY: [u32; 2] = [0x93_D876, 0x99_9B5E];
/// The map, and the flag the intro sets as the fight begins.
const MAP: u16 = 0x0123;
const FIGHT_FLAG: u16 = 0x001;
/// The second life.
const SECOND_LIFE: u16 = 100;

/// The fight under way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Fight {
    /// The body's actor's id.
    body: u16,
    began: bool,
    second: bool,
}

impl World<'_> {
    /// At `$123`'s load: the body held for the fight.
    pub(super) fn start_shadowkeeper(&mut self) {
        if self.map != MAP {
            return;
        }
        let script = |resident: &crate::residents::Resident| resident.script;
        let Some(body) = self
            .residents
            .iter()
            .position(|resident| script(resident).is_some_and(|at| BODY.contains(&at)))
        else {
            return;
        };
        let descriptor = self.residents[body].descriptor;
        self.actors[body].hold_as_boss(self.image, descriptor);
        self.shadowkeeper = Some(Fight {
            body: self.actors[body].id,
            began: false,
            second: false,
        });
    }

    /// A frame of the fight, after the actors ran.
    pub(super) fn shadowkeeper_frame(&mut self) {
        let Some(mut fight) = self.shadowkeeper else {
            return;
        };
        fight.began |=
            self.globals.events[usize::from(FIGHT_FLAG / 8)] & 1 << (FIGHT_FLAG % 8) != 0;
        let Some(body) = self.actors.iter_mut().find(|actor| actor.id == fight.body) else {
            self.shadowkeeper = None;
            return;
        };
        body.set_target(fight.began);
        if !fight.second && body.foe_life() == Some(0) {
            fight.second = true;
            body.revive(SECOND_LIFE);
        }
        self.shadowkeeper = Some(fight);
    }
}
