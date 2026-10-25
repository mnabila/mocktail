//! Pure fixed-window counter shared by the native and Worker rate-limiter
//! adapters. The adapter supplies synchronization (a mutex or thread-local
//! `RefCell`); this type only owns the deterministic counting logic.

use super::ports::{RateLimitDecision, UnixMillis};
use std::collections::HashMap;

#[derive(Default)]
pub struct FixedWindowState {
    counters: HashMap<String, Counter>,
}

struct Counter {
    window_start: UnixMillis,
    window_secs: u32,
    count: u32,
}

impl FixedWindowState {
    /// Count one request. `key` is the endpoint + client identity; the window
    /// boundary is derived from `now` and `window_secs`.
    pub fn check(
        &mut self,
        key: &str,
        limit: u32,
        window_secs: u32,
        now: UnixMillis,
    ) -> RateLimitDecision {
        if limit == 0 || window_secs == 0 {
            return RateLimitDecision::Allow;
        }
        let window_ms = (window_secs as i64).saturating_mul(1000);
        let window_start = now.div_euclid(window_ms) * window_ms;

        // Keep long-lived processes/isolates from growing the map forever.
        if self.counters.len() > 4096 {
            self.counters.retain(|_, c| {
                let end = c
                    .window_start
                    .saturating_add((c.window_secs as i64).saturating_mul(1000));
                end > now
            });
        }

        let counter = self.counters.entry(key.to_string()).or_insert(Counter {
            window_start,
            window_secs,
            count: 0,
        });
        if counter.window_start != window_start || counter.window_secs != window_secs {
            counter.window_start = window_start;
            counter.window_secs = window_secs;
            counter.count = 0;
        }
        if counter.count >= limit {
            let remaining = window_start.saturating_add(window_ms).saturating_sub(now);
            let retry_after_secs = (remaining.max(0) as u64).div_ceil(1000);
            return RateLimitDecision::Deny {
                retry_after_secs: retry_after_secs.max(1),
            };
        }
        counter.count += 1;
        RateLimitDecision::Allow
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: UnixMillis = 1_700_000_000_000;

    #[test]
    fn allows_up_to_limit_then_denies() {
        let mut state = FixedWindowState::default();
        assert_eq!(
            state.check("m:1.2.3.4", 2, 60, NOW),
            RateLimitDecision::Allow
        );
        assert_eq!(
            state.check("m:1.2.3.4", 2, 60, NOW + 1),
            RateLimitDecision::Allow
        );
        assert!(matches!(
            state.check("m:1.2.3.4", 2, 60, NOW + 2),
            RateLimitDecision::Deny { .. }
        ));
    }

    #[test]
    fn resets_in_next_window_and_isolates_keys() {
        let mut state = FixedWindowState::default();
        assert_eq!(state.check("a", 1, 60, NOW), RateLimitDecision::Allow);
        assert!(matches!(
            state.check("a", 1, 60, NOW + 1),
            RateLimitDecision::Deny { .. }
        ));
        assert_eq!(state.check("b", 1, 60, NOW + 1), RateLimitDecision::Allow);
        assert_eq!(
            state.check("a", 1, 60, NOW + 60_000),
            RateLimitDecision::Allow
        );
    }

    #[test]
    fn retry_after_is_positive_and_within_window() {
        let mut state = FixedWindowState::default();
        let _ = state.check("a", 1, 60, NOW);
        let RateLimitDecision::Deny { retry_after_secs } = state.check("a", 1, 60, NOW + 1) else {
            panic!("expected deny");
        };
        assert!((1..=60).contains(&retry_after_secs));
    }

    #[test]
    fn zero_limit_or_window_disables_limiting() {
        let mut state = FixedWindowState::default();
        assert_eq!(state.check("a", 0, 60, NOW), RateLimitDecision::Allow);
        assert_eq!(state.check("a", 1, 0, NOW), RateLimitDecision::Allow);
    }
}
