#![allow(clippy::unwrap_used)]
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, Utc};

/// A timestamp that can be modified via immutable reference
///
/// Essentially, an `Atomic<Option<DateTime<Utc>>>`, where all operations use sequentially
/// consistent ordering - if this causes perf issues, we can look to be more fine grained, but it's
/// probably good enough for now
#[derive(Debug)]
pub struct AtomicTimestamp {
    /// Milliseconds since epoch
    ///
    /// `i64::MIN` is treated as `None`
    inner: AtomicI64,
}

impl AtomicTimestamp {
    /// Create a new [`AtomicTimestamp`] from the given [`SystemTime`]
    pub fn new(time: Option<DateTime<Utc>>) -> Self {
        let inner = AtomicI64::new(Self::encode(time));
        Self { inner }
    }

    /// Store a time in this [`AtomicTimestamp`]
    pub fn store(&self, time: Option<DateTime<Utc>>) {
        self.inner.store(Self::encode(time), Ordering::SeqCst);
    }

    /// Load the stored time
    pub fn load(&self) -> Option<DateTime<Utc>> {
        let millis = self.inner.load(Ordering::SeqCst);
        Self::decode(millis)
    }

    /// Add a duration to this timestamp, returning the previous value
    ///
    /// If the current value is `None`, `None` is returned and the state isn't changed
    ///
    /// Note: the duration is rounded down to the nearest millisecond before it is added
    pub fn fetch_add(&self, duration: Duration) -> Option<DateTime<Utc>> {
        self.inner
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |old| match old {
                i64::MIN => None,
                old => Some(old + duration.num_milliseconds()),
            })
            .ok()
            .and_then(Self::decode)
    }

    fn encode(time: Option<DateTime<Utc>>) -> i64 {
        match time {
            None => i64::MIN,
            Some(x) => x.timestamp_millis(),
        }
    }

    fn decode(millis: i64) -> Option<DateTime<Utc>> {
        match millis {
            i64::MIN => None,
            millis => Some(DateTime::from_timestamp_millis(millis).unwrap()),
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{LocalResult, TimeZone};
    use proptest::{prop_assert_eq, prop_assume};
    use test_strategy::proptest;

    use super::*;

    #[proptest]
    fn any_sane_time_load_store_round_trip(
        #[strategy(1971..=2040)] year: i32,
        #[strategy(1u32..=12)] month: u32,
        #[strategy(1u32..=31)] day: u32,
        #[strategy(0u32..=23)] hour: u32,
        #[strategy(0u32..=60)] min: u32,
        #[strategy(0u32..=60)] sec: u32,
    ) {
        let time = Utc.with_ymd_and_hms(year, month, day, hour, min, sec);
        let time = match time {
            LocalResult::Single(time) => time,
            // handle months with fewer than 31 days
            LocalResult::None | LocalResult::Ambiguous(..) => {
                prop_assume!(false);
                panic!();
            }
        };

        let atomic = AtomicTimestamp::new(Some(time));

        let time_again = atomic.load().unwrap();
        prop_assert_eq!(time, time_again);

        // just in case store behaves differently
        atomic.store(Some(time));
        let time_again = atomic.load().unwrap();
        prop_assert_eq!(time, time_again);
    }

    #[test]
    fn fetch_add_sub_works() {
        let time = Utc.with_ymd_and_hms(2023, 1, 1, 5, 4, 3).unwrap();

        let atomic = AtomicTimestamp::new(Some(time));
        atomic.fetch_add(Duration::seconds(5));

        assert_eq!(
            atomic.load().unwrap(),
            Utc.with_ymd_and_hms(2023, 1, 1, 5, 4, 8).unwrap()
        );
    }
}
