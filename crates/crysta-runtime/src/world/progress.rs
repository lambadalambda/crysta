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
        }
    }

    /// Starts gaining a level, on a free frame, when Ark's EXP reaches the next one's
    /// (`$85:EA99`, the table `$8D:BA61`, European `$8D:B92A`): one level a
    /// pass, its presentation in [`super::levelup`].
    pub(super) fn level_up(&mut self) {
        // Not while Ark is busy or down (`$85:EA99` checks each free frame).
        let busy = self.hurt.is_some() || self.down.is_some() || self.thrust.is_some();
        if self.level_up.is_some() || busy || self.globals.slot.stats().life == 0 {
            return;
        }
        let table = assets::layout::per_revision(self.image, LEVELS, LEVELS - 0x137);
        let Some(table) = self.image.get(table..) else {
            return;
        };
        let stats = self.globals.slot.stats();
        let (Some(now), Some(next)) = (
            combat::level(table, stats.level),
            combat::level(table, stats.level.saturating_add(1)),
        ) else {
            return;
        };
        if stats.exp >= next.exp {
            let level = stats.level.saturating_add(1);
            self.level_up = Some(super::levelup::LevelUp::new(now, next, level));
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

    /// Hits the first spawned enemy a hit may land on now, as Ark's thrust
    /// would with `damage`, for tests. Returns its script, if one was hit.
    #[doc(hidden)]
    pub fn hit_spawned(&mut self, damage: u16) -> Option<u32> {
        let (resident, actor) =
            self.residents
                .iter()
                .zip(&mut self.actors)
                .find(|(resident, actor)| {
                    resident.record == 0 && actor.body_box().is_some() && !actor.unharmed()
                })?;
        actor.take_hit(damage, room_core::Direction::Down, self.image);
        resident.script
    }

    /// Hits a record's enemy as Ark's thrust would with `damage`, if a hit
    /// may land on it now, for tests. Returns whether it did.
    #[doc(hidden)]
    pub fn hit_foe(&mut self, record: usize, damage: u16) -> bool {
        let Some(index) = self.actor_of(record) else {
            return false;
        };
        let actor = &mut self.actors[index];
        if actor.body_box().is_none() || actor.unharmed() {
            return false;
        }
        actor.take_hit(damage, room_core::Direction::Down, self.image);
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

    /// Puts an item in the inventory, for tests.
    #[doc(hidden)]
    pub fn give_item(&mut self, item: u8) {
        self.globals.inventory.add(item);
    }

    /// Whether the inventory holds an item.
    #[must_use]
    pub fn has_item(&self, item: u8) -> bool {
        self.globals.inventory.count(item) > 0
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
