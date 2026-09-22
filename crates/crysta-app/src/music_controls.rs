//! Device-independent music controls; never part of simulation state.

/// User pause is independent of the temporary focus/suspension pause.
#[derive(Clone, Copy, Debug)]
pub struct Controls {
    paused: bool,
    focused: bool,
    percent: u8,
}

impl Default for Controls {
    fn default() -> Self {
        Self {
            paused: false,
            focused: true,
            percent: 50,
        }
    }
}

impl Controls {
    pub fn playing(self) -> bool {
        !self.paused && self.focused
    }

    pub fn volume(self) -> f32 {
        f32::from(self.percent) / 100.0
    }

    pub fn toggle(&mut self) {
        self.paused = !self.paused;
    }

    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    pub fn adjust_volume(&mut self, delta: i16) {
        self.percent = u8::try_from((i16::from(self.percent).saturating_add(delta)).clamp(0, 100))
            .expect("clamped percentage");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_audible_at_moderate_volume() {
        let controls = Controls::default();
        assert!(controls.playing());
        assert_eq!(controls.volume().to_bits(), 0.5_f32.to_bits());
    }

    #[test]
    fn focus_does_not_erase_the_users_pause_choice() {
        let mut controls = Controls::default();
        controls.toggle();
        controls.set_focused(false);
        controls.set_focused(true);
        assert!(!controls.playing());
        controls.toggle();
        assert!(controls.playing());
        controls.set_focused(false);
        assert!(!controls.playing());
        controls.set_focused(true);
        assert!(controls.playing());
    }

    #[test]
    fn volume_is_bounded_and_independent_of_pause() {
        let mut controls = Controls::default();
        for _ in 0..30 {
            controls.adjust_volume(10);
        }
        assert_eq!(controls.volume().to_bits(), 1.0_f32.to_bits());
        controls.toggle();
        for _ in 0..30 {
            controls.adjust_volume(-10);
        }
        assert_eq!(controls.volume().to_bits(), 0.0_f32.to_bits());
        assert!(!controls.playing());
        controls.adjust_volume(10);
        assert_eq!(controls.volume().to_bits(), 0.1_f32.to_bits());
    }
}
