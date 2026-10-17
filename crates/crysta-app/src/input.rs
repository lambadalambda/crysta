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

/// The held directions as the walk takes them: the last one pressed, and
/// the last one pressed across it, a diagonal.
pub fn held_axes(
    keys: &[room_core::Direction],
) -> (Option<room_core::Direction>, Option<room_core::Direction>) {
    let Some(&last) = keys.last() else {
        return (None, None);
    };
    let cross = keys
        .iter()
        .rev()
        .find(|&&key| key.horizontal() != last.horizontal())
        .copied();
    (Some(last), cross)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_last_key_leads_and_the_last_key_across_it_makes_a_diagonal() {
        use room_core::Direction::{Down, Left, Right, Up};
        assert_eq!(held_axes(&[]), (None, None));
        assert_eq!(held_axes(&[Right]), (Some(Right), None));
        assert_eq!(held_axes(&[Right, Up]), (Some(Up), Some(Right)));
        assert_eq!(held_axes(&[Up, Down]), (Some(Down), None));
        assert_eq!(held_axes(&[Left, Up, Down]), (Some(Down), Some(Left)));
    }

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
