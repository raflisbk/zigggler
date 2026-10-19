use chrono::{Local, NaiveTime};

/// Minimum idle time (as reported by `IdleWatcher`) before a jiggle is allowed
/// through when `idle_only` is on.
pub const IDLE_THRESHOLD_SECS: u64 = 8;

fn parse_hhmm(raw: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(raw, "%H:%M").ok()
}

/// True if `now` falls within `[start, end)`, handling the overnight case
/// where `start > end` (e.g. a 22:00-06:00 night shift window).
pub fn is_within_schedule(start: &str, end: &str) -> bool {
    let (Some(start), Some(end)) = (parse_hhmm(start), parse_hhmm(end)) else {
        // Unparseable schedule — fail open rather than silently never jiggling.
        return true;
    };
    let now = Local::now().time();
    if start <= end {
        now >= start && now < end
    } else {
        now >= start || now < end
    }
}

pub fn should_jiggle(
    in_schedule: bool,
    idle_only: bool,
    idle_seconds: u64,
) -> bool {
    in_schedule && (!idle_only || idle_seconds >= IDLE_THRESHOLD_SECS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_requires_schedule() {
        assert!(!should_jiggle(false, false, 0));
        assert!(should_jiggle(true, false, 0));
    }

    #[test]
    fn gate_requires_idle_when_idle_only() {
        assert!(!should_jiggle(true, true, 0));
        assert!(should_jiggle(true, true, IDLE_THRESHOLD_SECS));
    }
}
