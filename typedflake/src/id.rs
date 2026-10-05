//! ID values: representation, validation, and decomposition.
//!
//! Everything here is pure: no global state, generators, or clock access.

use core::fmt;
use core::num::ParseIntError;
use core::str::FromStr;
use std::sync::OnceLock;

use crate::format::{Format, Layout};
use crate::generator::{self, GenerateError, Generator, GeneratorError};
use crate::node::{Node, NodeError};

mod sealed {
    pub trait Sealed {}
    impl Sealed for i64 {}
    impl Sealed for u64 {}
}

/// Integer an ID is stored in: `i64` or `u64`.
///
/// `i64` IDs reserve the sign bit and are never negative, which keeps them
/// portable to signed database columns.
pub trait Repr:
    sealed::Sealed
    + Copy
    + fmt::Debug
    + fmt::Display
    + FromStr<Err = ParseIntError>
    + Send
    + Sync
    + 'static
{
    #[doc(hidden)]
    const USABLE_BITS: u8;

    #[doc(hidden)]
    fn __bits(self) -> u64;

    #[doc(hidden)]
    fn __checked_bits(self) -> Result<u64, InvalidId>;

    #[doc(hidden)]
    fn __from_bits(bits: u64) -> Self;
}

impl Repr for i64 {
    const USABLE_BITS: u8 = 63;

    fn __bits(self) -> u64 {
        self as u64
    }

    fn __checked_bits(self) -> Result<u64, InvalidId> {
        if self < 0 {
            return Err(InvalidId::Negative { value: self });
        }
        Ok(self as u64)
    }

    fn __from_bits(bits: u64) -> Self {
        bits as i64
    }
}

impl Repr for u64 {
    const USABLE_BITS: u8 = 64;

    fn __bits(self) -> u64 {
        self
    }

    fn __checked_bits(self) -> Result<u64, InvalidId> {
        Ok(self)
    }

    fn __from_bits(bits: u64) -> Self {
        bits
    }
}

/// An ID type declared with [`#[typedflake]`](macro@crate::typedflake).
///
/// The attribute implements this trait; it is not meant to be implemented by
/// hand. Every method is also generated as an inherent method, so the trait
/// only needs importing for code that is generic over ID types:
///
/// ```
/// use typedflake::{Id, typedflake};
///
/// #[typedflake(epoch = "2025-01-01")]
/// pub struct UserId(i64);
///
/// fn sequence_of<I: Id>(id: I) -> u64 {
///     id.parts().sequence
/// }
///
/// let id = UserId::try_from(4_198_401_i64)?;
/// assert_eq!(sequence_of(id), 1);
/// # Ok::<(), typedflake::InvalidId>(())
/// ```
pub trait Id: Copy + Send + Sync + 'static {
    /// Integer the ID is stored in.
    type Repr: Repr;

    /// Node type: `u32`, or a struct deriving
    /// [`TypedNode`](macro@crate::TypedNode).
    type Node: Node;

    /// Persistent format of this ID type.
    const FORMAT: Format;

    #[doc(hidden)]
    const __LAYOUT: Layout = Layout::resolve::<Self::Repr, Self::Node>(Self::FORMAT);

    #[doc(hidden)]
    fn __from_repr(repr: Self::Repr) -> Self;

    #[doc(hidden)]
    fn __default_generator() -> &'static OnceLock<Generator<Self>>;

    /// Returns the stored integer.
    fn get(self) -> Self::Repr;

    /// Generates an ID with the node installed by [`init`](crate::init).
    ///
    /// Returns immediately. If this millisecond's sequence is used up it
    /// returns [`GenerateError::SequenceExhausted`] instead of waiting.
    fn generate() -> Result<Self, GenerateError> {
        generator::default_generator::<Self>()?.generate()
    }

    /// Like [`generate`](Self::generate), but blocks the thread while it
    /// waits for sequence capacity.
    fn generate_blocking() -> Result<Self, GenerateError> {
        generator::default_generator::<Self>()?.generate_blocking()
    }

    /// Like [`generate`](Self::generate), but waits for sequence capacity on a
    /// Tokio timer.
    #[cfg(feature = "tokio")]
    fn generate_async() -> impl Future<Output = Result<Self, GenerateError>> + Send {
        async {
            generator::default_generator::<Self>()?
                .generate_async()
                .await
        }
    }

    /// Returns a generator for an explicit node, without global
    /// initialization.
    ///
    /// Generators for the same ID type and node share their state, as do their
    /// clones and the static [`generate`](Self::generate) path.
    fn generator(node: Self::Node) -> Result<Generator<Self>, GeneratorError> {
        Generator::new(node)
    }

    /// Splits the ID into its timestamp, node, and sequence.
    fn parts(self) -> Parts<Self::Node> {
        let layout = Self::__LAYOUT;
        let raw = self.get().__bits();
        Parts {
            elapsed_millis: raw >> layout.timestamp_shift,
            node: Self::Node::unpack(
                (raw >> layout.node_shift) & layout.node_max,
                layout.node_bits,
            ),
            sequence: raw & layout.sequence_max,
        }
    }

    /// Builds an ID from its parts, checking each against the format.
    ///
    /// This does not reserve a sequence number or guarantee uniqueness.
    fn from_parts(parts: Parts<Self::Node>) -> Result<Self, InvalidId> {
        let layout = Self::__LAYOUT;
        if parts.elapsed_millis > layout.timestamp_max {
            return Err(InvalidId::Timestamp {
                value: parts.elapsed_millis,
                max: layout.timestamp_max,
            });
        }
        if parts.sequence > layout.sequence_max {
            return Err(InvalidId::Sequence {
                value: parts.sequence,
                max: layout.sequence_max,
            });
        }
        let node = parts.node.pack(layout.node_bits)?;

        Ok(from_valid_bits(
            (parts.elapsed_millis << layout.timestamp_shift)
                | (node << layout.node_shift)
                | parts.sequence,
        ))
    }

    /// Returns when the ID was created, in milliseconds since the Unix epoch.
    fn unix_millis(self) -> Result<u64, TimestampError> {
        let epoch_unix_millis = Self::__LAYOUT.epoch;
        let elapsed_millis = self.parts().elapsed_millis;
        epoch_unix_millis
            .checked_add(elapsed_millis)
            .ok_or(TimestampError {
                epoch_unix_millis,
                elapsed_millis,
            })
    }
}

/// Wraps bits already known to satisfy the format.
pub(crate) fn from_valid_bits<I: Id>(bits: u64) -> I {
    debug_assert!(bits <= I::__LAYOUT.raw_max);
    I::__from_repr(I::Repr::__from_bits(bits))
}

#[doc(hidden)]
pub fn from_u64<I: Id>(raw: u64) -> Result<I, InvalidId> {
    let max = I::__LAYOUT.raw_max;
    if raw > max {
        return Err(InvalidId::ReservedBits { value: raw, max });
    }
    Ok(from_valid_bits(raw))
}

#[doc(hidden)]
pub fn from_i64<I: Id>(raw: i64) -> Result<I, InvalidId> {
    from_u64(raw.__checked_bits()?)
}

#[doc(hidden)]
pub fn parse<I: Id>(text: &str) -> Result<I, ParseIdError> {
    let repr: I::Repr = text.parse().map_err(ParseIdError::Int)?;
    Ok(from_u64(repr.__checked_bits()?)?)
}

/// The timestamp, node, and sequence of an ID.
///
/// `N` is the ID's node type: `u32`, or its
/// [`TypedNode`](macro@crate::TypedNode) struct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Parts<N> {
    /// Milliseconds between the format's epoch and the ID's creation.
    pub elapsed_millis: u64,
    /// Node that generated the ID.
    pub node: N,
    /// Position of the ID within its millisecond.
    pub sequence: u64,
}

/// A raw value or set of parts is not a valid ID for the format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum InvalidId {
    /// The value is negative.
    Negative {
        /// Value that was supplied.
        value: i64,
    },
    /// The value uses bits above those the format assigns.
    ReservedBits {
        /// Value that was supplied.
        value: u64,
        /// Largest value the format can represent.
        max: u64,
    },
    /// The timestamp does not fit its width.
    Timestamp {
        /// Elapsed milliseconds that were supplied.
        value: u64,
        /// Largest timestamp the format accepts.
        max: u64,
    },
    /// The sequence does not fit its width.
    Sequence {
        /// Sequence that was supplied.
        value: u64,
        /// Largest sequence the format accepts.
        max: u64,
    },
    /// A node field does not fit its width.
    Node(NodeError),
}

impl fmt::Display for InvalidId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Negative { value } => write!(f, "ID {value} is negative"),
            Self::ReservedBits { value, max } => write!(
                f,
                "ID {value} sets reserved bits; the format's maximum is {max}"
            ),
            Self::Timestamp { value, max } => {
                write!(f, "timestamp {value} exceeds the format's maximum of {max}")
            }
            Self::Sequence { value, max } => {
                write!(f, "sequence {value} exceeds the format's maximum of {max}")
            }
            Self::Node(_) => f.write_str("node does not fit the format"),
        }
    }
}

impl std::error::Error for InvalidId {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Node(error) => Some(error),
            _ => None,
        }
    }
}

impl From<NodeError> for InvalidId {
    fn from(error: NodeError) -> Self {
        Self::Node(error)
    }
}

/// A string could not be parsed as an ID.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseIdError {
    /// The string is not a decimal integer that fits the ID's integer type.
    Int(ParseIntError),
    /// The integer is not a valid ID for the format.
    Invalid(InvalidId),
}

impl fmt::Display for ParseIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int(_) => f.write_str("ID is not a valid integer"),
            Self::Invalid(_) => f.write_str("integer is not a valid ID"),
        }
    }
}

impl std::error::Error for ParseIdError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Int(error) => Some(error),
            Self::Invalid(error) => Some(error),
        }
    }
}

impl From<InvalidId> for ParseIdError {
    fn from(error: InvalidId) -> Self {
        Self::Invalid(error)
    }
}

/// An ID's creation time does not fit in a `u64` of Unix milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct TimestampError {
    /// The format's epoch, in milliseconds since the Unix epoch.
    pub epoch_unix_millis: u64,
    /// Milliseconds the ID was created after the epoch.
    pub elapsed_millis: u64,
}

impl fmt::Display for TimestampError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "epoch {} plus {} elapsed milliseconds overflows",
            self.epoch_unix_millis, self.elapsed_millis
        )
    }
}

impl std::error::Error for TimestampError {}
