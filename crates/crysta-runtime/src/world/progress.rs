//! What a fight leaves (`docs/combat.md` §4, §6): gems taken, EXP and the
//! levels it brings, and the damage digits; with hooks for tests and hosts.

use super::World;
use crate::combat;
use crate::scene::Digits;

/// The sound of a gem taken.
const GEM_SOUND: u8 = 0x47;
/// The level table (`$8D:BA61`): 11 bytes a level.
const LEVELS: usize = 0x0D_BA61;

impl World<'_> {
    /// Takes the EXP of enemies that died this frame (`$85:E218`), and a
    /// gem Ark stands on (sound `$47`).
    pub(super) fn collect_deaths(&mut self) {
        let player = self.position();
        let gems: u32 = self
            .actors
            .iter_mut()
            .filter_map(|actor| actor.take_gem(player))
            .map(u32::from)
            .sum();
        if gems > 0 {
            self.globals.inventory.add_money(gems);
            self.globals.audio.sound_port3(GEM_SOUND);
        }
        let mut exp = 0;
        for actor in &mut self.actors {
            if std::mem::take(&mut actor.died) {
                exp += actor
                    .foe
                    .as_ref()
                    .map_or(0, |foe| u32::from(foe.profile.exp));
            }
        }
        if exp > 0 {
            let total = self.globals.slot.exp() + exp;
            self.globals.slot.set_exp(total);
            self.level_up();
        }
    }

    /// Raises Ark while his EXP reaches the next level's (`$85:EA99`, the
    /// table `$8D:BA61`, European `$8D:B92A`). The native world stops for
    /// the victory pose and a message per stat (516 frames); not modelled.
    fn level_up(&mut self) {
        let table = assets::layout::per_revision(self.image, LEVELS, LEVELS - 0x137);
        let Some(table) = self.image.get(table..) else {
            return;
        };
        loop {
            let stats = self.globals.slot.stats();
            let (Some(now), Some(next)) = (
                combat::level(table, stats.level),
                combat::level(table, stats.level.saturating_add(1)),
            ) else {
                return;
            };
            if stats.exp < next.exp {
                return;
            }
            self.globals.slot.raise_level(&now, &next);
        }
    }

    /// The damage digits floating now.
    #[must_use]
    pub fn digits(&self) -> &[Digits] {
        &self.globals.digits
    }

    /// Ages the damage digits a frame and drops the finished ones.
    pub(super) fn age_digits(&mut self) {
        for digits in &mut self.globals.digits {
            digits.age += 1;
        }
        self.globals
            .digits
            .retain(|digits| digits.age < Digits::FRAMES);
    }

    /// Ark's combat stats.
    #[must_use]
    pub fn stats(&self) -> combat::Stats {
        self.globals.slot.stats()
    }

    /// The EXP (`$0690`).
    #[must_use]
    pub fn exp(&self) -> u32 {
        self.globals.slot.exp()
    }

    /// The index of `record`'s actor here.
    fn actor_of(&self, record: usize) -> Option<usize> {
        self.residents
            .iter()
            .position(|resident| resident.record == record)
    }

    /// An enemy's life, by its spawn record; `None` once gone.
    #[doc(hidden)]
    #[must_use]
    pub fn foe_life(&self, record: usize) -> Option<u16> {
        let foe = self.actors.get(self.actor_of(record)?)?.foe.as_ref()?;
        (!foe.dead).then_some(foe.life)
    }

    /// Kills an enemy at once, for tests: its death check runs next
    /// frame. Returns whether the record is an enemy here.
    #[doc(hidden)]
    pub fn kill_foe(&mut self, record: usize) -> bool {
        let Some(index) = self.actor_of(record) else {
            return false;
        };
        let Some(foe) = self.actors[index].foe.as_mut() else {
            return false;
        };
        foe.life = 0;
        foe.knocked = true;
        true
    }

    /// Places an enemy and sets its life, for tests. Returns whether the
    /// record is an enemy here.
    #[doc(hidden)]
    pub fn poke_foe(&mut self, record: usize, at: (u16, u16), life: u16) -> bool {
        let Some(index) = self.actor_of(record) else {
            return false;
        };
        let actor = &mut self.actors[index];
        let Some(foe) = &mut actor.foe else {
            return false;
        };
        foe.life = life;
        actor.position = at;
        true
    }

    /// Sets Ark's life, for tests.
    #[doc(hidden)]
    pub fn set_life(&mut self, life: u16) {
        self.globals.slot.set_life(life);
    }

    /// Sets the EXP, for tests.
    #[doc(hidden)]
    pub fn set_exp(&mut self, exp: u32) {
        self.globals.slot.set_exp(exp);
    }

    /// The first layer's cell word at (column, row), as enemies probe it,
    /// for tests.
    #[doc(hidden)]
    #[must_use]
    pub fn base_cell(&self, column: u16, row: u16) -> Option<u16> {
        if column >= self.base.width || row >= self.base.height {
            return None;
        }
        let at = usize::from(row) * usize::from(self.base.width) + usize::from(column);
        self.base.room.cells().get(at).copied()
    }
}
