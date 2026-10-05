//! Persistent ID format: epoch and bit allocation.
//!
//! A [`Format`] describes how an ID is laid out in its integer. It belongs to
//! the ID type and never changes at runtime.

use core::fmt;

use crate::id::Repr;
use crate::node::Node;

const MILLIS_PER_DAY: u64 = 86_400_000;

/// Widest node a width-agnostic node type ([`Node::BITS`] of `None`) can fill.
const FLEXIBLE_NODE_MAX_BITS: u8 = 32;

/// Instant that ID timestamps are measured from.
///
/// Stored as milliseconds since the Unix epoch, in UTC.
///
/// ```
/// use typedflake::Epoch;
///
/// const LAUNCH: Epoch = Epoch::from_date(2025, 1, 1);
/// assert_eq!(LAUNCH.unix_millis(), 1_735_689_600_000);
/// assert_eq!(LAUNCH, Epoch::new(1_735_689_600_000));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Epoch {
    unix_millis: u64,
}

impl Epoch {
    /// Creates an epoch from milliseconds since the Unix epoch.
    pub const fn new(unix_millis: u64) -> Self {
        Self { unix_millis }
    }

    /// Creates an epoch at midnight UTC of a calendar date.
    ///
    /// # Panics
    ///
    /// Panics if the date does not exist or is before 1970-01-01. In a const
    /// context this is a compile-time error.
    pub const fn from_date(year: u16, month: u8, day: u8) -> Self {
        match Self::try_from_date(year, month, day) {
            Ok(epoch) => epoch,
            Err(error) => panic!("{}", error.message()),
        }
    }

    /// Creates an epoch at midnight UTC of a calendar date, rejecting dates
    /// that do not exist or are before 1970-01-01.
    pub const fn try_from_date(year: u16, month: u8, day: u8) -> Result<Self, FormatError> {
        if year < 1970 || month < 1 || month > 12 || day < 1 || day > days_in_month(year, month) {
            return Err(FormatError::InvalidDate { year, month, day });
        }
        Ok(Self::new(
            days_since_unix_epoch(year, month, day) * MILLIS_PER_DAY,
        ))
    }

    /// Milliseconds between the Unix epoch and this epoch.
    pub const fn unix_millis(self) -> u64 {
        self.unix_millis
    }
}

const fn is_leap_year(year: u16) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

const fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 31,
    }
}

/// Days from 1970-01-01 to a valid civil date on or after it.
const fn days_since_unix_epoch(year: u16, month: u8, day: u8) -> u64 {
    // Shift the year to start in March so the leap day falls at its end.
    let year = if month <= 2 {
        year as u64 - 1
    } else {
        year as u64
    };
    let era = year / 400;
    let year_of_era = year % 400;
    let shifted_month = (month as u64 + 9) % 12;
    let day_of_year = (153 * shifted_month + 2) / 5 + day as u64 - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// Number of bits given to each part of an ID.
///
/// Sequence occupies the least significant bits, then node, then timestamp.
/// The widths may add up to less than the integer's capacity; the unused upper
/// bits are reserved and must be zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BitLayout {
    /// Bits for milliseconds elapsed since the epoch. Must be greater than zero.
    pub timestamp: u8,
    /// Bits for the node. May be zero, in which case only node zero is valid.
    pub node: u8,
    /// Bits for the per-millisecond sequence. Must be greater than zero.
    pub sequence: u8,
}

/// Persistent format of an ID type: its epoch and bit allocation.
///
/// Declare one as a constant and share it between ID types:
///
/// ```
/// use typedflake::{BitLayout, Epoch, Format, typedflake};
///
/// pub const APP_IDS: Format = Format {
///     epoch: Epoch::from_date(2025, 1, 1),
///     bits: BitLayout {
///         timestamp: 41,
///         node: 10,
///         sequence: 12,
///     },
/// };
///
/// #[typedflake(format = APP_IDS)]
/// pub struct UserId(i64);
///
/// #[typedflake(format = APP_IDS)]
/// pub struct OrderId(i64);
/// ```
///
/// A format is only usable once an ID declaration has validated it against
/// that ID's integer and node type, which happens at compile time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Format {
    /// Instant that timestamps are measured from.
    pub epoch: Epoch,
    /// Bit allocation.
    pub bits: BitLayout,
}

impl Format {
    /// Checks this format against an ID integer (`i64` or `u64`) and node type.
    ///
    /// ID declarations run this check at compile time; call it directly to
    /// validate a format without declaring an ID.
    ///
    /// ```
    /// use typedflake::{BitLayout, Epoch, Format, FormatError};
    ///
    /// let format = Format {
    ///     epoch: Epoch::from_date(2025, 1, 1),
    ///     bits: BitLayout { timestamp: 42, node: 10, sequence: 12 },
    /// };
    ///
    /// assert_eq!(format.validate::<u64, u32>(), Ok(()));
    /// assert_eq!(
    ///     format.validate::<i64, u32>(),
    ///     Err(FormatError::TooManyBits { total: 64, max: 63 }),
    /// );
    /// ```
    pub const fn validate<R: Repr, N: Node>(&self) -> Result<(), FormatError> {
        let bits = self.bits;
        if bits.timestamp == 0 {
            return Err(FormatError::ZeroTimestampBits);
        }
        if bits.sequence == 0 {
            return Err(FormatError::ZeroSequenceBits);
        }

        let total = bits.timestamp as u16 + bits.node as u16 + bits.sequence as u16;
        if total > R::USABLE_BITS as u16 {
            return Err(FormatError::TooManyBits {
                total,
                max: R::USABLE_BITS,
            });
        }

        match N::BITS {
            Some(node) if node != bits.node => Err(FormatError::NodeWidthMismatch {
                format: bits.node,
                node,
            }),
            None if bits.node > FLEXIBLE_NODE_MAX_BITS => Err(FormatError::NodeTooWide {
                bits: bits.node,
                max: FLEXIBLE_NODE_MAX_BITS,
            }),
            _ => Ok(()),
        }
    }
}

/// Reason a [`Format`] cannot be used by an ID type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FormatError {
    /// The date does not exist or is before 1970-01-01.
    InvalidDate {
        /// Supplied year.
        year: u16,
        /// Supplied month.
        month: u8,
        /// Supplied day.
        day: u8,
    },
    /// The timestamp width is zero.
    ZeroTimestampBits,
    /// The sequence width is zero.
    ZeroSequenceBits,
    /// The widths add up to more bits than the ID integer can hold.
    TooManyBits {
        /// Sum of the three widths.
        total: u16,
        /// Usable bits of the integer: 63 for `i64`, 64 for `u64`.
        max: u8,
    },
    /// The node type declares a width different from the format's node width.
    NodeWidthMismatch {
        /// Node width in the format.
        format: u8,
        /// Width declared by the node type.
        node: u8,
    },
    /// The node width is larger than a plain `u32` node can fill.
    NodeTooWide {
        /// Node width in the format.
        bits: u8,
        /// Widest supported node.
        max: u8,
    },
}

impl FormatError {
    /// Message without the variant's values, usable in const panics.
    pub(crate) const fn message(&self) -> &'static str {
        match self {
            Self::InvalidDate { .. } => {
                "typedflake: epoch must be a real calendar date on or after 1970-01-01"
            }
            Self::ZeroTimestampBits => "typedflake: timestamp width must be greater than zero",
            Self::ZeroSequenceBits => "typedflake: sequence width must be greater than zero",
            Self::TooManyBits { max: 63, .. } => {
                "typedflake: bit widths exceed the 63 usable bits of `i64`"
            }
            Self::TooManyBits { .. } => "typedflake: bit widths exceed the 64 bits of `u64`",
            Self::NodeWidthMismatch { .. } => {
                "typedflake: node type width does not match the format's node bits"
            }
            Self::NodeTooWide { .. } => {
                "typedflake: a plain `u32` node supports at most 32 node bits"
            }
        }
    }
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::InvalidDate { year, month, day } => write!(
                f,
                "{year:04}-{month:02}-{day:02} is not a calendar date on or after 1970-01-01"
            ),
            Self::ZeroTimestampBits => f.write_str("timestamp width must be greater than zero"),
            Self::ZeroSequenceBits => f.write_str("sequence width must be greater than zero"),
            Self::TooManyBits { total, max } => write!(
                f,
                "bit widths add up to {total}, but the ID integer has {max} usable bits"
            ),
            Self::NodeWidthMismatch { format, node } => write!(
                f,
                "node type is {node} bits wide, but the format reserves {format} node bits"
            ),
            Self::NodeTooWide { bits, max } => write!(
                f,
                "format reserves {bits} node bits, but a plain `u32` node supports at most {max}"
            ),
        }
    }
}

impl std::error::Error for FormatError {}

/// Default widths for a representation: 10 node bits, 12 sequence bits, and
/// the remaining usable bits for the timestamp.
#[doc(hidden)]
pub const fn default_bits<R: Repr>() -> BitLayout {
    BitLayout {
        timestamp: R::USABLE_BITS - 22,
        node: 10,
        sequence: 12,
    }
}

pub(crate) const fn low_mask(bits: u8) -> u64 {
    if bits >= 64 {
        u64::MAX
    } else {
        (1 << bits) - 1
    }
}

/// A validated [`Format`] with its shifts and limits precomputed.
#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub struct Layout {
    pub(crate) epoch: u64,
    pub(crate) node_bits: u8,
    pub(crate) sequence_bits: u8,
    pub(crate) node_shift: u8,
    pub(crate) timestamp_shift: u8,
    pub(crate) timestamp_max: u64,
    pub(crate) node_max: u64,
    pub(crate) sequence_max: u64,
    /// Largest raw value; anything above it has reserved bits set.
    pub(crate) raw_max: u64,
}

impl Layout {
    /// Validates and resolves a format. Panics on an invalid format, which is
    /// a compile-time error because ID declarations evaluate it in a const.
    pub const fn resolve<R: Repr, N: Node>(format: Format) -> Self {
        if let Err(error) = format.validate::<R, N>() {
            panic!("{}", error.message());
        }

        let BitLayout {
            timestamp,
            node,
            sequence,
        } = format.bits;

        Self {
            epoch: format.epoch.unix_millis(),
            node_bits: node,
            sequence_bits: sequence,
            node_shift: sequence,
            timestamp_shift: sequence + node,
            timestamp_max: low_mask(timestamp),
            node_max: low_mask(node),
            sequence_max: low_mask(sequence),
            raw_max: low_mask(timestamp + node + sequence),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn format(timestamp: u8, node: u8, sequence: u8) -> Format {
        Format {
            epoch: Epoch::new(0),
            bits: BitLayout {
                timestamp,
                node,
                sequence,
            },
        }
    }

    #[test]
    fn epoch_from_date_matches_known_instants() {
        assert_eq!(Epoch::from_date(1970, 1, 1).unix_millis(), 0);
        assert_eq!(Epoch::from_date(1970, 1, 2).unix_millis(), MILLIS_PER_DAY);
        assert_eq!(Epoch::from_date(2000, 3, 1).unix_millis(), 951_868_800_000);
        assert_eq!(
            Epoch::from_date(2024, 2, 29).unix_millis(),
            1_709_164_800_000
        );
        assert_eq!(
            Epoch::from_date(2025, 1, 1).unix_millis(),
            1_735_689_600_000
        );
        assert_eq!(
            Epoch::from_date(2100, 12, 31).unix_millis(),
            4_133_894_400_000
        );
    }

    #[test]
    fn epoch_rejects_dates_that_do_not_exist() {
        for (year, month, day) in [
            (1969, 12, 31),
            (2025, 0, 1),
            (2025, 13, 1),
            (2025, 1, 0),
            (2025, 4, 31),
            (2025, 2, 29),
            (2100, 2, 29),
        ] {
            assert_eq!(
                Epoch::try_from_date(year, month, day),
                Err(FormatError::InvalidDate { year, month, day }),
            );
        }
        assert!(Epoch::try_from_date(2000, 2, 29).is_ok());
    }

    #[test]
    fn validate_accepts_full_and_reduced_widths() {
        assert_eq!(format(41, 10, 12).validate::<i64, u32>(), Ok(()));
        assert_eq!(format(42, 10, 12).validate::<u64, u32>(), Ok(()));
        assert_eq!(format(32, 5, 8).validate::<i64, u32>(), Ok(()));
        assert_eq!(format(1, 0, 1).validate::<i64, u32>(), Ok(()));
        assert_eq!(format(63, 0, 1).validate::<u64, u32>(), Ok(()));
    }

    #[test]
    fn validate_rejects_invalid_widths() {
        assert_eq!(
            format(0, 10, 12).validate::<i64, u32>(),
            Err(FormatError::ZeroTimestampBits)
        );
        assert_eq!(
            format(41, 10, 0).validate::<i64, u32>(),
            Err(FormatError::ZeroSequenceBits)
        );
        assert_eq!(
            format(42, 10, 12).validate::<i64, u32>(),
            Err(FormatError::TooManyBits { total: 64, max: 63 })
        );
        assert_eq!(
            format(255, 255, 255).validate::<u64, u32>(),
            Err(FormatError::TooManyBits {
                total: 765,
                max: 64
            })
        );
        assert_eq!(
            format(20, 33, 10).validate::<i64, u32>(),
            Err(FormatError::NodeTooWide { bits: 33, max: 32 })
        );
    }

    #[test]
    fn default_bits_fill_the_usable_width() {
        let signed = default_bits::<i64>();
        assert_eq!(
            (signed.timestamp, signed.node, signed.sequence),
            (41, 10, 12)
        );
        let unsigned = default_bits::<u64>();
        assert_eq!(
            (unsigned.timestamp, unsigned.node, unsigned.sequence),
            (42, 10, 12)
        );
    }

    #[test]
    fn layout_resolves_limits_without_overflow() {
        let full = Layout::resolve::<u64, u32>(format(42, 10, 12));
        assert_eq!(full.timestamp_shift, 22);
        assert_eq!(full.node_shift, 12);
        assert_eq!(full.timestamp_max, (1 << 42) - 1);
        assert_eq!(full.node_max, 1023);
        assert_eq!(full.sequence_max, 4095);
        assert_eq!(full.raw_max, u64::MAX);

        let signed = Layout::resolve::<i64, u32>(format(41, 10, 12));
        assert_eq!(signed.raw_max, i64::MAX as u64);

        let reduced = Layout::resolve::<i64, u32>(format(32, 5, 8));
        assert_eq!(reduced.raw_max, (1 << 45) - 1);

        let no_node = Layout::resolve::<u64, u32>(format(63, 0, 1));
        assert_eq!(no_node.node_max, 0);
        assert_eq!(no_node.timestamp_shift, 1);
        assert_eq!(no_node.raw_max, u64::MAX);
    }
}
