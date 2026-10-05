//! Wall-clock access, with a seam for deterministic tests.

use std::time::{Duration, SystemTime, SystemTimeError, UNIX_EPOCH};

const NANOS_PER_MILLI: u32 = 1_000_000;

/// Milliseconds since the Unix epoch.
pub(crate) fn unix_millis() -> Result<u64, SystemTimeError> {
    #[cfg(test)]
    if let Some(millis) = mock::get() {
        return Ok(millis);
    }

    let since_epoch = SystemTime::now().duration_since(UNIX_EPOCH)?;
    // Saturating keeps a clock beyond `u64` milliseconds failing as timestamp
    // exhaustion instead of wrapping.
    Ok(u64::try_from(since_epoch.as_millis()).unwrap_or(u64::MAX))
}

/// Time left until the wall clock reaches its next millisecond.
pub(crate) fn until_next_milli() -> Duration {
    let into_milli = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since_epoch| {
            since_epoch.subsec_nanos() % NANOS_PER_MILLI
        });
    Duration::from_nanos(u64::from(NANOS_PER_MILLI - into_milli))
}

#[cfg(test)]
pub(crate) mod mock {
    use std::cell::Cell;

    thread_local! {
        static NOW: Cell<Option<u64>> = const { Cell::new(None) };
    }

    pub(crate) fn get() -> Option<u64> {
        NOW.get()
    }

    /// Freezes the clock for the current thread until the guard is dropped.
    pub(crate) fn freeze(unix_millis: u64) -> Frozen {
        NOW.set(Some(unix_millis));
        Frozen
    }

    pub(crate) struct Frozen;

    impl Frozen {
        pub(crate) fn set(&self, unix_millis: u64) {
            NOW.set(Some(unix_millis));
        }
    }

    impl Drop for Frozen {
        fn drop(&mut self) {
            NOW.set(None);
        }
    }
}
