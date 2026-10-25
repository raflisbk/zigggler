pub fn idle_ms(now: u32, last: u32) -> u32 {
    now.wrapping_sub(last)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tick_wrap_stays_in_dword_domain() {
        assert_eq!(idle_ms(20, u32::MAX - 9), 30);
    }
    #[test]
    fn timestamp_need_not_increase() {
        assert_eq!(idle_ms(100, 90), 10);
    }
}
