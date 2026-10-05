//! ID generation handles.

use core::fmt;
use core::marker::PhantomData;
use std::sync::Arc;
use std::thread;
use std::time::SystemTimeError;

use crate::clock;
use crate::global;
use crate::id::{self, Id};
use crate::node::{Node, NodeError};
use crate::state::{self, State};

/// Generates IDs of type `I` for one node.
///
/// Obtain one from [`Id::generator`]. Cloning is cheap, and every generator
/// for the same ID type and node shares one state, so no two of them can issue
/// the same ID.
///
/// ```
/// use typedflake::typedflake;
///
/// #[typedflake(epoch = "2025-01-01")]
/// pub struct UserId(i64);
///
/// let generator = UserId::generator(17)?;
/// let first = generator.generate()?;
/// let second = generator.clone().generate()?;
///
/// assert!(first < second);
/// assert_eq!(first.parts().node, 17);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct Generator<I: Id> {
    state: Arc<State>,
    /// Packed node, already shifted into its position in the ID.
    node: u64,
    _id: PhantomData<fn() -> I>,
}

impl<I: Id> Generator<I> {
    pub(crate) fn new(node: I::Node) -> Result<Self, GeneratorError> {
        let layout = I::__LAYOUT;
        let packed = node
            .pack(layout.node_bits)
            .map_err(GeneratorError::InvalidNode)?;

        Ok(Self {
            state: state::shared::<I>(packed),
            node: packed << layout.node_shift,
            _id: PhantomData,
        })
    }

    /// Returns the node this generator issues IDs for.
    pub fn node(&self) -> I::Node {
        let layout = I::__LAYOUT;
        I::Node::unpack(self.node >> layout.node_shift, layout.node_bits)
    }

    /// Generates an ID.
    ///
    /// Returns immediately. If this millisecond's sequence is used up it
    /// returns [`GenerateError::SequenceExhausted`] instead of waiting.
    pub fn generate(&self) -> Result<I, GenerateError> {
        let layout = I::__LAYOUT;
        let (timestamp, sequence) = self.state.next(&layout, || {
            let now_unix_millis = clock::unix_millis().map_err(GenerateError::Clock)?;
            now_unix_millis
                .checked_sub(layout.epoch)
                .ok_or(GenerateError::ClockBeforeEpoch {
                    now_unix_millis,
                    epoch_unix_millis: layout.epoch,
                })
        })?;
        Ok(id::from_valid_bits(
            (timestamp << layout.timestamp_shift) | self.node | sequence,
        ))
    }

    /// Generates an ID, blocking the thread while it waits for sequence
    /// capacity.
    ///
    /// Only sequence exhaustion is retried; every other error returns at once.
    /// Capacity returns with the next millisecond.
    pub fn generate_blocking(&self) -> Result<I, GenerateError> {
        loop {
            match self.generate() {
                Err(GenerateError::SequenceExhausted) => {}
                result => return result,
            }
            thread::sleep(clock::until_next_milli());
        }
    }

    /// Generates an ID, waiting for sequence capacity on a Tokio timer.
    ///
    /// Only sequence exhaustion is retried, and the timer is used only when
    /// waiting is needed. Capacity returns with the next millisecond; to bound
    /// the wait, wrap the call in `tokio::time::timeout`.
    #[cfg(feature = "tokio")]
    pub async fn generate_async(&self) -> Result<I, GenerateError> {
        loop {
            match self.generate() {
                Err(GenerateError::SequenceExhausted) => {}
                result => return result,
            }
            tokio::time::sleep(clock::until_next_milli()).await;
        }
    }
}

impl<I: Id> Clone for Generator<I> {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            node: self.node,
            _id: PhantomData,
        }
    }
}

impl<I: Id> fmt::Debug for Generator<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Generator")
            .field("id", &core::any::type_name::<I>())
            .field("node", &(self.node >> I::__LAYOUT.node_shift))
            .finish()
    }
}

/// Returns the generator behind `I::generate()`, resolving it from the global
/// default node of its type on first use. A failure leaves it unresolved, so
/// generating before [`init`](crate::init) does not freeze anything.
pub(crate) fn default_generator<I: Id>() -> Result<&'static Generator<I>, GenerateError> {
    let cell = I::__default_generator();
    if let Some(generator) = cell.get() {
        return Ok(generator);
    }

    let node = global::default_node::<I::Node>()?;
    let generator = Generator::new(node).map_err(GenerateError::DefaultGenerator)?;
    Ok(cell.get_or_init(|| generator))
}

/// A generator could not be created for a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum GeneratorError {
    /// The node does not fit the ID's format.
    InvalidNode(NodeError),
}

impl fmt::Display for GeneratorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNode(_) => f.write_str("node does not fit the ID format"),
        }
    }
}

impl std::error::Error for GeneratorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidNode(error) => Some(error),
        }
    }
}

/// An ID could not be generated.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum GenerateError {
    /// [`init`](crate::init) has not been called with a node of the type this
    /// ID uses.
    NotInitialized {
        /// Node type of the ID: `u32`, or its `TypedNode` struct.
        node: &'static str,
    },
    /// The default node does not fit this ID's format.
    DefaultGenerator(GeneratorError),
    /// The system clock could not be read.
    Clock(SystemTimeError),
    /// The system clock is before the format's epoch.
    ClockBeforeEpoch {
        /// Current time, in milliseconds since the Unix epoch.
        now_unix_millis: u64,
        /// The format's epoch, in milliseconds since the Unix epoch.
        epoch_unix_millis: u64,
    },
    /// The system clock moved behind the last issued timestamp. Generation
    /// resumes on its own once the clock catches up.
    ClockRollback {
        /// How far the clock is behind the last issued timestamp.
        behind_millis: u64,
    },
    /// Every sequence number of the current millisecond is used. Retrying on
    /// the next millisecond succeeds.
    SequenceExhausted,
    /// The current time no longer fits the format's timestamp width.
    TimestampExhausted,
}

impl fmt::Display for GenerateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotInitialized { node } => write!(
                f,
                "no default `{node}` node: call `typedflake::init` with one during startup"
            ),
            Self::DefaultGenerator(_) => f.write_str("default node is not valid for this ID"),
            Self::Clock(_) => f.write_str("system clock could not be read"),
            Self::ClockBeforeEpoch {
                now_unix_millis,
                epoch_unix_millis,
            } => write!(
                f,
                "system clock ({now_unix_millis} ms) is before the ID epoch ({epoch_unix_millis} ms)"
            ),
            Self::ClockRollback { behind_millis } => write!(
                f,
                "system clock is {behind_millis} ms behind the last generated ID"
            ),
            Self::SequenceExhausted => {
                f.write_str("sequence exhausted for the current millisecond")
            }
            Self::TimestampExhausted => {
                f.write_str("current time no longer fits the ID's timestamp bits")
            }
        }
    }
}

impl std::error::Error for GenerateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::DefaultGenerator(error) => Some(error),
            Self::Clock(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use typedflake::typedflake;

    use super::*;
    use crate::Parts;
    use crate::clock::mock;

    const EPOCH: u64 = 1_735_689_600_000;

    #[test]
    fn ids_encode_time_node_and_sequence() {
        #[typedflake(epoch = "2025-01-01")]
        struct TestId(i64);

        let clock = mock::freeze(EPOCH + 1_000);
        let generator = TestId::generator(17).unwrap();

        let first = generator.generate().unwrap();
        let second = generator.generate().unwrap();
        clock.set(EPOCH + 1_001);
        let third = generator.generate().unwrap();

        let parts = |elapsed_millis, sequence| Parts {
            elapsed_millis,
            node: 17,
            sequence,
        };
        assert_eq!(first.parts(), parts(1_000, 0));
        assert_eq!(second.parts(), parts(1_000, 1));
        assert_eq!(third.parts(), parts(1_001, 0));
        assert_eq!(first.get(), (1_000 << 22) | (17 << 12));
        assert_eq!(first.unix_millis(), Ok(EPOCH + 1_000));
        assert!(first < second && second < third);
    }

    #[test]
    fn clock_before_epoch_is_an_error() {
        #[typedflake(epoch = "2025-01-01")]
        struct TestId(i64);

        let _clock = mock::freeze(EPOCH - 1);
        let generator = TestId::generator(0).unwrap();

        assert!(matches!(
            generator.generate(),
            Err(GenerateError::ClockBeforeEpoch {
                now_unix_millis,
                epoch_unix_millis: EPOCH,
            }) if now_unix_millis == EPOCH - 1
        ));
    }

    #[test]
    fn rollback_is_reported_and_generation_recovers() {
        #[typedflake(epoch = "2025-01-01")]
        struct TestId(i64);

        let clock = mock::freeze(EPOCH + 500);
        let generator = TestId::generator(1).unwrap();
        let before = generator.generate().unwrap();

        clock.set(EPOCH + 480);
        assert!(matches!(
            generator.generate(),
            Err(GenerateError::ClockRollback { behind_millis: 20 })
        ));
        // Waiting entry points do not retry a rollback.
        assert!(matches!(
            generator.generate_blocking(),
            Err(GenerateError::ClockRollback { behind_millis: 20 })
        ));

        clock.set(EPOCH + 500);
        let after = generator.generate().unwrap();
        assert_eq!(after.parts().sequence, before.parts().sequence + 1);
    }

    #[test]
    fn sequence_exhaustion_is_retryable() {
        #[typedflake(epoch = "2025-01-01", bits(timestamp = 41, node = 10, sequence = 2))]
        struct TestId(i64);

        let clock = mock::freeze(EPOCH + 1);
        let generator = TestId::generator(3).unwrap();
        for _ in 0..4 {
            generator.generate().unwrap();
        }

        assert!(matches!(
            generator.generate(),
            Err(GenerateError::SequenceExhausted)
        ));
        clock.set(EPOCH + 2);
        assert_eq!(generator.generate().unwrap().parts().sequence, 0);
    }

    #[test]
    fn timestamp_exhaustion_is_permanent() {
        #[typedflake(epoch = "2025-01-01", bits(timestamp = 4, node = 10, sequence = 12))]
        struct TestId(i64);

        let clock = mock::freeze(EPOCH + 15);
        let generator = TestId::generator(0).unwrap();
        assert_eq!(generator.generate().unwrap().parts().elapsed_millis, 15);

        clock.set(EPOCH + 16);
        assert!(matches!(
            generator.generate(),
            Err(GenerateError::TimestampExhausted)
        ));
        assert!(matches!(
            generator.generate_blocking(),
            Err(GenerateError::TimestampExhausted)
        ));
    }

    #[test]
    fn generators_for_the_same_node_share_state() {
        #[typedflake(epoch = "2025-01-01")]
        struct TestId(i64);
        #[typedflake(epoch = "2025-01-01")]
        struct OtherId(i64);

        let _clock = mock::freeze(EPOCH + 7);
        let first = TestId::generator(1).unwrap();
        let again = TestId::generator(1).unwrap();
        let cloned = first.clone();
        let other_node = TestId::generator(2).unwrap();
        let other_type = OtherId::generator(1).unwrap();

        assert_eq!(first.generate().unwrap().parts().sequence, 0);
        assert_eq!(again.generate().unwrap().parts().sequence, 1);
        assert_eq!(cloned.generate().unwrap().parts().sequence, 2);
        assert_eq!(other_node.generate().unwrap().parts().sequence, 0);
        assert_eq!(other_type.generate().unwrap().parts().sequence, 0);
        assert_eq!(again.node(), 1);
        assert_eq!(other_node.node(), 2);
    }

    #[test]
    fn out_of_range_node_is_rejected_without_truncation() {
        #[typedflake(epoch = "2025-01-01")]
        struct TestId(i64);
        #[typedflake(epoch = "2025-01-01", bits(timestamp = 41, node = 0, sequence = 12))]
        struct NodelessId(i64);

        assert_eq!(
            TestId::generator(1024).unwrap_err(),
            GeneratorError::InvalidNode(NodeError::new("node", 1024, 1023))
        );
        assert!(TestId::generator(1023).is_ok());
        assert_eq!(
            NodelessId::generator(1).unwrap_err(),
            GeneratorError::InvalidNode(NodeError::new("node", 1, 0))
        );
        assert!(NodelessId::generator(0).is_ok());
    }
}

#[cfg(all(test, feature = "tokio"))]
mod tokio_tests {
    use std::time::Duration;

    use typedflake::typedflake;

    use super::*;
    use crate::clock::mock;

    const EPOCH: u64 = 1_735_689_600_000;

    /// With the clock frozen, capacity never returns. A wait built on a
    /// blocking sleep would hang here instead of letting the timeout fire.
    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn async_wait_yields_to_the_runtime_and_can_be_cancelled() {
        #[typedflake(epoch = "2025-01-01", bits(timestamp = 41, node = 10, sequence = 1))]
        struct TestId(i64);

        let _clock = mock::freeze(EPOCH + 1);
        let generator = TestId::generator(0).unwrap();
        generator.generate_async().await.unwrap();
        generator.generate_async().await.unwrap();

        let waited =
            tokio::time::timeout(Duration::from_millis(10), generator.generate_async()).await;
        assert!(waited.is_err());
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn async_wait_does_not_retry_other_errors() {
        #[typedflake(epoch = "2025-01-01")]
        struct TestId(i64);

        let clock = mock::freeze(EPOCH + 10);
        let generator = TestId::generator(0).unwrap();
        generator.generate_async().await.unwrap();

        clock.set(EPOCH + 5);
        assert!(matches!(
            generator.generate_async().await,
            Err(GenerateError::ClockRollback { behind_millis: 5 })
        ));
    }
}
