use rand::Rng;
use std::collections::VecDeque;
use std::ops::RangeInclusive;
use std::time::{Duration, Instant};

/// Minimum idle time (as reported by `IdleWatcher`) before a jiggle is allowed
/// through when `idle_only` is on.
pub const IDLE_THRESHOLD_SECS: u64 = 8;

pub fn should_jiggle(idle_only: bool, idle_seconds: u64) -> bool {
    !idle_only || idle_seconds >= IDLE_THRESHOLD_SECS
}

/// Max breaks that may *start* within any rolling `QUOTA_WINDOW` — counted by
/// start time, not end time.
const BREAK_QUOTA: usize = 6;
const QUOTA_WINDOW: Duration = Duration::from_secs(7200);
const BREAK_DURATION_SECS: RangeInclusive<u64> = 60..=300;
const ACTIVE_GAP_SECS: RangeInclusive<u64> = 900..=1800;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakState {
    /// Jiggling normally; waiting for `deadline` to start the next break.
    Active,
    /// On a break; waiting for `deadline` to resume jiggling.
    Break,
    /// Quota full, waiting for the oldest recorded start to age out of the
    /// rolling window (at `deadline`) before a break can start.
    QuotaWait,
}

/// Lives for the lifetime of the jiggle-loop thread, created once the
/// `random_breaks` toggle first turns on and kept around afterward (per
/// `docs/goals.md`: state is in-memory only, persists across the toggle
/// within the same process, resets only on process restart).
pub struct RandomBreakScheduler {
    state: BreakState,
    deadline: Instant,
    recent_break_starts: VecDeque<Instant>,
}

impl RandomBreakScheduler {
    pub fn new(now: Instant) -> Self {
        Self {
            state: BreakState::Active,
            deadline: now + Duration::from_secs(sample_range(ACTIVE_GAP_SECS)),
            recent_break_starts: VecDeque::new(),
        }
    }

    pub fn state(&self) -> BreakState {
        self.state
    }

    /// Advances the state machine to `now`. Returns `true` if jiggling
    /// should run right now (i.e. not currently on a break).
    pub fn tick(&mut self, now: Instant) -> bool {
        self.prune_quota(now);
        match self.state {
            BreakState::Active => {
                if now >= self.deadline {
                    self.try_start_break(now);
                }
            }
            BreakState::Break => {
                if now >= self.deadline {
                    self.state = BreakState::Active;
                    self.deadline = now + Duration::from_secs(sample_range(ACTIVE_GAP_SECS));
                }
            }
            BreakState::QuotaWait => {
                if now >= self.deadline {
                    self.try_start_break(now);
                }
            }
        }
        self.state != BreakState::Break
    }

    /// Drops recorded break starts that have aged out of the rolling
    /// `QUOTA_WINDOW` — a start exactly `QUOTA_WINDOW` old no longer counts
    /// against the quota.
    fn prune_quota(&mut self, now: Instant) {
        while let Some(&oldest) = self.recent_break_starts.front() {
            if now.duration_since(oldest) >= QUOTA_WINDOW {
                self.recent_break_starts.pop_front();
            } else {
                break;
            }
        }
    }

    fn try_start_break(&mut self, now: Instant) {
        self.prune_quota(now);
        if self.recent_break_starts.len() < BREAK_QUOTA {
            self.recent_break_starts.push_back(now);
            self.state = BreakState::Break;
            self.deadline = now + Duration::from_secs(sample_range(BREAK_DURATION_SECS));
        } else {
            let oldest = *self
                .recent_break_starts
                .front()
                .expect("quota full implies at least one recorded start");
            self.state = BreakState::QuotaWait;
            self.deadline = oldest + QUOTA_WINDOW;
        }
    }
}

fn sample_range(range: RangeInclusive<u64>) -> u64 {
    rand::thread_rng().gen_range(range)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_requires_idle_when_idle_only() {
        assert!(!should_jiggle(true, 0));
        assert!(should_jiggle(true, IDLE_THRESHOLD_SECS));
    }

    #[test]
    fn gate_always_open_when_idle_only_off() {
        assert!(should_jiggle(false, 0));
    }

    #[test]
    fn break_duration_and_active_gap_sampling_stays_in_range() {
        for _ in 0..1000 {
            let duration = sample_range(BREAK_DURATION_SECS);
            assert!((60..=300).contains(&duration));
            let gap = sample_range(ACTIVE_GAP_SECS);
            assert!((900..=1800).contains(&gap));
        }
    }

    #[test]
    fn quota_prunes_start_at_exactly_window_age() {
        let mut scheduler = RandomBreakScheduler::new(Instant::now());
        let now = Instant::now();
        scheduler.recent_break_starts.push_back(now);
        scheduler.prune_quota(now + QUOTA_WINDOW);
        assert!(
            scheduler.recent_break_starts.is_empty(),
            "a start exactly QUOTA_WINDOW old must age out of the quota"
        );
    }

    #[test]
    fn quota_keeps_start_one_tick_below_window_age() {
        let mut scheduler = RandomBreakScheduler::new(Instant::now());
        let now = Instant::now();
        scheduler.recent_break_starts.push_back(now);
        scheduler.prune_quota(now + QUOTA_WINDOW - Duration::from_secs(1));
        assert_eq!(scheduler.recent_break_starts.len(), 1);
    }

    #[test]
    fn seventh_break_start_deferred_to_oldest_plus_window() {
        let base = Instant::now();
        let mut scheduler = RandomBreakScheduler::new(base);
        // Fill the quota with 6 starts, all still within the window.
        for i in 0..6u64 {
            scheduler
                .recent_break_starts
                .push_back(base + Duration::from_secs(i * 10));
        }
        let oldest = *scheduler.recent_break_starts.front().unwrap();

        // A 7th attempt while the quota is still full must defer rather
        // than start immediately.
        scheduler.try_start_break(base + Duration::from_secs(100));
        assert_eq!(scheduler.state(), BreakState::QuotaWait);
        assert_eq!(scheduler.deadline, oldest + QUOTA_WINDOW);

        // At the deadline, the oldest ages out and the break can start.
        assert!(!scheduler.tick(scheduler.deadline));
        assert_eq!(scheduler.state(), BreakState::Break);
        assert_eq!(scheduler.recent_break_starts.len(), 6);
    }
}
