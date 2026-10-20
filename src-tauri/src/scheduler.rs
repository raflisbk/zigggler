/// Minimum idle time (as reported by `IdleWatcher`) before a jiggle is allowed
/// through when `idle_only` is on.
pub const IDLE_THRESHOLD_SECS: u64 = 8;

pub fn should_jiggle(idle_only: bool, idle_seconds: u64) -> bool {
    !idle_only || idle_seconds >= IDLE_THRESHOLD_SECS
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
}
