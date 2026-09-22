//! Retain interaction edges between fixed simulation updates.
#[derive(Default)]
pub struct Interaction {
    pending: bool,
    pad_down: bool,
}

impl Interaction {
    pub fn keyboard(&mut self, pressed: bool) {
        self.pending |= pressed; // OS key repeats are filtered by the host.
    }

    pub fn gamepad(&mut self, down: bool) {
        self.pending |= down && !self.pad_down;
        self.pad_down = down;
    }

    pub fn take(&mut self) -> bool {
        std::mem::take(&mut self.pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quick_keyboard_tap_survives_release_and_pad_poll_before_deadline() {
        let mut input = Interaction::default();
        input.keyboard(true);
        input.keyboard(false);
        for _ in 0..10 {
            input.gamepad(false);
        }
        assert!(input.take());
        assert!(!input.take());
    }

    #[test]
    fn held_pad_does_not_retrigger_during_catchup_and_keyboard_still_works() {
        let mut input = Interaction::default();
        input.gamepad(true);
        assert!(input.take());
        for _ in 0..4 {
            input.gamepad(true);
            assert!(!input.take());
        }
        input.keyboard(true);
        assert!(input.take());
        input.gamepad(false);
        input.gamepad(true);
        assert!(input.take());
    }
}
