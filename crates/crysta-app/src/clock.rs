//! Fixed-rate native presentation clock, independent of window redraw frequency.

use std::time::Duration;

/// NTSC non-interlaced mean: (1364 * 262 - 2) master clocks per frame at
/// (315 MHz / 88) * 6. Rounded to the nearest nanosecond: ~60.098814 Hz.
/// The four-clock short scanline occurs on every other frame. See vendored
/// ares/sfc/ppu/counter/inline.hpp and ares/ares/ares.hpp. Host pacing only;
/// this does not reproduce game logic skipped during a native lag frame.
pub const FRAME_PERIOD: Duration = Duration::from_nanos(16_639_263);
/// Frames one poll may run to catch up.
pub const MAX_CATCH_UP: usize = 4;

/// What a poll asks the host to run.
pub struct Batch {
    /// Frames due.
    pub steps: usize,
    /// Whether more were due and were dropped.
    pub dropped_backlog: bool,
}

/// Deadlines are relative to a host-owned monotonic origin, not wall-clock time.
pub struct Clock {
    next: Duration,
}

impl Clock {
    /// A clock whose first frame is due one period after `now`.
    pub fn new(now: Duration) -> Self {
        Self {
            next: now + FRAME_PERIOD,
        }
    }

    /// Starts again from `now`.
    pub fn reset(&mut self, now: Duration) {
        *self = Self::new(now);
    }

    /// When the next frame is due.
    pub const fn deadline(&self) -> Duration {
        self.next
    }

    /// The frames due at `now`.
    pub fn poll(&mut self, now: Duration) -> Batch {
        let mut steps = 0;
        while now >= self.next && steps < MAX_CATCH_UP {
            steps += 1;
            self.next += FRAME_PERIOD;
        }
        let dropped_backlog = now >= self.next;
        if dropped_backlog {
            // Long host stalls are not a request to replay minutes of held input.
            self.reset(now);
        }
        Batch {
            steps,
            dropped_backlog,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn extra_event_loop_wakeups_do_not_advance_simulation() {
        let mut clock = Clock::new(Duration::ZERO);
        for _ in 0..100 {
            assert_eq!(
                clock
                    .poll(FRAME_PERIOD.checked_sub(Duration::from_nanos(1)).unwrap())
                    .steps,
                0
            );
        }
        assert_eq!(clock.poll(FRAME_PERIOD).steps, 1);
        assert_eq!(clock.poll(FRAME_PERIOD).steps, 0);
    }

    #[test]
    fn display_refresh_does_not_change_simulation_rate() {
        for hz in [60, 120, 144, 240, 1000] {
            let mut clock = Clock::new(Duration::ZERO);
            let mut ticks = 0;
            for wake in 0..=hz * 10 {
                let elapsed = Duration::from_secs_f64(f64::from(wake) / f64::from(hz));
                let batch = clock.poll(elapsed);
                assert!(!batch.dropped_backlog);
                ticks += batch.steps;
            }
            assert_eq!(ticks, 600, "display refresh {hz}");
        }
    }

    #[test]
    fn stalls_have_bounded_catchup_not_an_unbounded_fast_forward() {
        let mut clock = Clock::new(Duration::ZERO);
        let now = Duration::from_secs(10);
        let batch = clock.poll(now);
        assert_eq!(batch.steps, MAX_CATCH_UP);
        assert!(batch.dropped_backlog);
        assert_eq!(clock.poll(now).steps, 0);
        assert_eq!(clock.deadline(), now + FRAME_PERIOD);
        assert_eq!(clock.poll(clock.deadline()).steps, 1);
    }

    #[test]
    fn exactly_four_due_ticks_are_not_reported_as_dropped() {
        let mut clock = Clock::new(Duration::ZERO);
        let batch = clock.poll(FRAME_PERIOD * 4);
        assert_eq!(batch.steps, 4);
        assert!(!batch.dropped_backlog);
        let mut clock = Clock::new(Duration::ZERO);
        assert!(clock.poll(FRAME_PERIOD * 5).dropped_backlog);
    }

    #[test]
    fn resuming_rebases_without_advancing_suspended_time() {
        let mut clock = Clock::new(Duration::ZERO);
        clock.reset(Duration::from_secs(30));
        assert_eq!(clock.poll(Duration::from_secs(30)).steps, 0);
        assert_eq!(clock.poll(Duration::from_secs(30) + FRAME_PERIOD).steps, 1);
    }
}
