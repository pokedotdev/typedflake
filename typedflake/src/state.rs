//! Coordinated generation state, shared by every handle of an ID type and node.

use std::any::TypeId;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use crate::format::Layout;
use crate::generator::GenerateError;
use crate::id::Id;

/// Last issued timestamp and sequence, packed as `timestamp << sequence_bits |
/// sequence` so both advance in one compare-and-swap.
///
/// A fresh state reads as "sequence 0 issued at elapsed 0". The only effect is
/// that an ID generated exactly at the epoch millisecond starts at sequence 1.
#[derive(Debug, Default)]
pub(crate) struct State(AtomicU64);

impl State {
    /// Reserves the next `(timestamp, sequence)`. A failed call leaves the
    /// state unchanged.
    ///
    /// `elapsed` reads the milliseconds since the epoch and is called after
    /// every load of the state. That order matters: a stored timestamp was
    /// read from the clock before it was stored, so a reading taken after the
    /// load can only be behind it if the clock itself went backwards. Reading
    /// the clock once up front would instead report a rollback whenever
    /// another thread stored a newer millisecond in between.
    pub(crate) fn next(
        &self,
        layout: &Layout,
        mut elapsed: impl FnMut() -> Result<u64, GenerateError>,
    ) -> Result<(u64, u64), GenerateError> {
        let mut current = self.0.load(Ordering::Acquire);
        loop {
            let elapsed = elapsed()?;
            if elapsed > layout.timestamp_max {
                return Err(GenerateError::TimestampExhausted);
            }

            let last_timestamp = current >> layout.sequence_bits;
            let last_sequence = current & layout.sequence_max;

            let sequence = if elapsed > last_timestamp {
                0
            } else if elapsed < last_timestamp {
                return Err(GenerateError::ClockRollback {
                    behind_millis: last_timestamp - elapsed,
                });
            } else if last_sequence < layout.sequence_max {
                last_sequence + 1
            } else {
                return Err(GenerateError::SequenceExhausted);
            };

            let next = (elapsed << layout.sequence_bits) | sequence;
            match self
                .0
                .compare_exchange_weak(current, next, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => return Ok((elapsed, sequence)),
                Err(actual) => current = actual,
            }
        }
    }
}

type Registry = HashMap<(TypeId, u64), Arc<State>>;

static REGISTRY: Mutex<Option<Registry>> = Mutex::new(None);

/// Returns the single state for an ID type and packed node, creating it on
/// first use. States live for the rest of the process.
pub(crate) fn shared<I: Id>(node: u64) -> Arc<State> {
    // The map is never left half-updated, so a poisoned lock is still valid.
    let mut registry = REGISTRY.lock().unwrap_or_else(PoisonError::into_inner);
    Arc::clone(
        registry
            .get_or_insert_default()
            .entry((TypeId::of::<I>(), node))
            .or_default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::{BitLayout, Epoch, Format};

    fn layout(timestamp: u8, node: u8, sequence: u8) -> Layout {
        Layout::resolve::<u64, u32>(Format {
            epoch: Epoch::new(0),
            bits: BitLayout {
                timestamp,
                node,
                sequence,
            },
        })
    }

    #[test]
    fn sequence_increments_within_a_millisecond_and_resets_on_the_next() {
        let layout = layout(42, 10, 12);
        let state = State::default();

        assert_eq!(state.next(&layout, || Ok(5)).unwrap(), (5, 0));
        assert_eq!(state.next(&layout, || Ok(5)).unwrap(), (5, 1));
        assert_eq!(state.next(&layout, || Ok(5)).unwrap(), (5, 2));
        assert_eq!(state.next(&layout, || Ok(6)).unwrap(), (6, 0));
        assert_eq!(state.next(&layout, || Ok(9)).unwrap(), (9, 0));
    }

    #[test]
    fn first_id_at_the_epoch_millisecond_skips_sequence_zero() {
        let layout = layout(42, 10, 12);
        let state = State::default();

        assert_eq!(state.next(&layout, || Ok(0)).unwrap(), (0, 1));
    }

    #[test]
    fn sequence_exhaustion_is_reported_and_recovers_on_the_next_millisecond() {
        let layout = layout(42, 10, 2);
        let state = State::default();

        for sequence in 0..=3 {
            assert_eq!(state.next(&layout, || Ok(1)).unwrap(), (1, sequence));
        }
        assert!(matches!(
            state.next(&layout, || Ok(1)),
            Err(GenerateError::SequenceExhausted)
        ));
        assert!(matches!(
            state.next(&layout, || Ok(1)),
            Err(GenerateError::SequenceExhausted)
        ));
        assert_eq!(state.next(&layout, || Ok(2)).unwrap(), (2, 0));
    }

    #[test]
    fn rollback_is_reported_without_changing_state_and_recovers() {
        let layout = layout(42, 10, 12);
        let state = State::default();

        assert_eq!(state.next(&layout, || Ok(100)).unwrap(), (100, 0));
        assert!(matches!(
            state.next(&layout, || Ok(97)),
            Err(GenerateError::ClockRollback { behind_millis: 3 })
        ));
        assert!(matches!(
            state.next(&layout, || Ok(99)),
            Err(GenerateError::ClockRollback { behind_millis: 1 })
        ));
        // Once the clock catches up, generation continues where it left off.
        assert_eq!(state.next(&layout, || Ok(100)).unwrap(), (100, 1));
        assert_eq!(state.next(&layout, || Ok(101)).unwrap(), (101, 0));
    }

    #[test]
    fn a_newer_millisecond_stored_by_another_caller_is_not_a_rollback() {
        let layout = layout(42, 10, 12);
        let state = State::default();
        let mut readings = 0;

        // The first reading is taken, then another caller stores a newer
        // millisecond before this one's compare-and-swap. The retry must read
        // the clock again instead of comparing the stale reading.
        let slot = state.next(&layout, || {
            readings += 1;
            if readings == 1 {
                state.next(&layout, || Ok(101)).unwrap();
                Ok(100)
            } else {
                Ok(101)
            }
        });

        assert_eq!(slot.unwrap(), (101, 1));
        assert!(readings >= 2);
    }

    #[test]
    fn clock_errors_stop_generation_without_changing_state() {
        let layout = layout(42, 10, 12);
        let state = State::default();

        assert_eq!(state.next(&layout, || Ok(7)).unwrap(), (7, 0));
        assert!(matches!(
            state.next(&layout, || Err(GenerateError::WaitTimeout)),
            Err(GenerateError::WaitTimeout)
        ));
        assert_eq!(state.next(&layout, || Ok(7)).unwrap(), (7, 1));
    }

    #[test]
    fn timestamp_exhaustion_never_wraps() {
        let layout = layout(4, 10, 12);
        let state = State::default();

        assert_eq!(state.next(&layout, || Ok(15)).unwrap(), (15, 0));
        assert!(matches!(
            state.next(&layout, || Ok(16)),
            Err(GenerateError::TimestampExhausted)
        ));
        assert!(matches!(
            state.next(&layout, || Ok(u64::MAX)),
            Err(GenerateError::TimestampExhausted)
        ));
        assert_eq!(state.next(&layout, || Ok(15)).unwrap(), (15, 1));
    }

    #[test]
    fn widest_timestamp_and_sequence_do_not_overflow() {
        let wide_timestamp = layout(63, 0, 1);
        let state = State::default();
        let last = (1 << 63) - 1;
        assert_eq!(state.next(&wide_timestamp, || Ok(last)).unwrap(), (last, 0));
        assert_eq!(state.next(&wide_timestamp, || Ok(last)).unwrap(), (last, 1));
        assert!(matches!(
            state.next(&wide_timestamp, || Ok(last)),
            Err(GenerateError::SequenceExhausted)
        ));

        let wide_sequence = layout(1, 0, 63);
        let state = State::default();
        assert_eq!(state.next(&wide_sequence, || Ok(1)).unwrap(), (1, 0));
        assert_eq!(state.next(&wide_sequence, || Ok(1)).unwrap(), (1, 1));
    }

    #[test]
    fn concurrent_callers_never_receive_the_same_slot() {
        use std::collections::HashSet;
        use std::sync::Barrier;
        use std::thread;

        const THREADS: usize = 8;
        const PER_THREAD: usize = 2_000;

        let layout = layout(20, 0, 20);
        let state = State::default();
        let barrier = Barrier::new(THREADS);

        let slots: Vec<(u64, u64)> = thread::scope(|scope| {
            let handles: Vec<_> = (0..THREADS)
                .map(|thread| {
                    let (state, layout, barrier) = (&state, &layout, &barrier);
                    scope.spawn(move || {
                        barrier.wait();
                        (0..PER_THREAD)
                            // Threads disagree about the time to exercise the
                            // rollback path under contention.
                            .filter_map(|call| {
                                state
                                    .next(layout, || Ok(((call + thread) / 100) as u64))
                                    .ok()
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            handles
                .into_iter()
                .flat_map(|handle| handle.join().unwrap())
                .collect()
        });

        let unique: HashSet<_> = slots.iter().copied().collect();
        assert_eq!(unique.len(), slots.len());
        assert!(!slots.is_empty());
    }
}
