//! Configuration and bit allocation for Snowflake-style ID generation.
//!
//! This module provides the core configuration types for TypedFlake ID generation:
//! - [`BitLayout`]: Defines how the 64-bit ID space is divided
//! - [`Config`]: Complete configuration including bit layout and epoch
//!
//! # Choosing a Bit Allocation
//!
//! The 64-bit ID space must be divided between four components:
//!
//! ## Timestamp Bits (typically 41-45 bits)
//! - Determines how long your IDs remain unique before wrapping
//! - 41 bits = ~69 years, 42 bits = ~139 years, 45 bits = ~1115 years
//! - **Trade-off**: More timestamp bits = longer lifespan, fewer worker/sequence bits
//!
//! ## Worker Bits (typically 4-10 bits)
//! - Number of independent worker processes across all machines
//! - 5 bits = 32 workers, 8 bits = 256 workers, 10 bits = 1024 workers
//! - **Trade-off**: More worker bits = more horizontal scaling, fewer sequence bits
//!
//! ## Process Bits (typically 0-5 bits)
//! - Number of processes per worker (can be 0 if not needed)
//! - 5 bits = 32 processes per worker, 0 bits = worker-only mode
//! - **Trade-off**: More process bits = more local parallelism, fewer sequence bits
//!
//! ## Sequence Bits (typically 10-15 bits)
//! - Number of IDs that can be generated per millisecond per instance
//! - 10 bits = 1024 IDs/ms, 12 bits = 4096 IDs/ms, 15 bits = 32768 IDs/ms
//! - **Trade-off**: More sequence bits = higher throughput, fewer worker/timestamp bits
//!
//! # Industry-Standard Presets
//!
//! Use battle-tested bit allocations from real-world implementations:
//!
//! ```
//! use typedflake::{BitLayout, Config, Epoch};
//!
//! // Twitter Snowflake-inspired (42t|5w|5p|12s)
//! const TWITTER_CONFIG: Config = Config::new_unchecked(BitLayout::TWITTER, Epoch::TWITTER);
//!
//! // Discord's allocation (42t|5w|5p|12s)
//! const DISCORD_CONFIG: Config = Config::new_unchecked(BitLayout::DISCORD, Epoch::DISCORD);
//! ```
//!
//! # Custom Allocation
//!
//! Create a custom allocation based on your needs:
//!
//! ```
//! use typedflake::{BitLayout, Config};
//!
//! // Custom: 45 timestamp bits (~1115 years), 8 worker bits (256 workers),
//! //         4 process bits (16 processes), 7 sequence bits (128 IDs/ms)
//! const CUSTOM: BitLayout = BitLayout::new(45, 8, 4, 7);
//!
//! // Check capacity before committing
//! assert!((CUSTOM.timestamp_duration_years() - 1114.9).abs() < 1.0);
//! assert_eq!(CUSTOM.worker_max(), 255);
//! assert_eq!(CUSTOM.process_max(), 15);
//! assert_eq!(CUSTOM.ids_per_millisecond(), 128);
//! assert_eq!(CUSTOM.total_instances(), 4096);
//! ```

use derive_more::{Display, Error, From};
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

/// Errors that can occur when creating or validating a BitLayout
#[derive(Display, Error, Debug, Clone, PartialEq, Eq)]
pub enum BitLayoutError {
    #[display("Bit allocation must sum to 64, got {actual}")]
    InvalidSum { actual: u8 },
    #[display("Timestamp bits must be greater than 0")]
    ZeroTimestampBits,
    #[display("Sequence bits must be greater than 0")]
    ZeroSequenceBits,
}

impl BitLayoutError {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::InvalidSum { .. } => "Bit allocation must sum to 64",
            Self::ZeroTimestampBits => "Timestamp bits must be greater than 0",
            Self::ZeroSequenceBits => "Sequence bits must be greater than 0",
        }
    }
}

/// Errors that can occur when creating an Epoch
#[derive(Display, Error, Debug, Clone, PartialEq, Eq)]
pub enum EpochError {
    #[display("Invalid year {year}, must be between 1970 and 2100")]
    InvalidYear { year: u16 },
    #[display("Invalid month {month}, must be between 1 and 12")]
    InvalidMonth { month: u8 },
    #[display("Invalid day {day} for month {month}")]
    InvalidDay { day: u8, month: u8 },
}

impl EpochError {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::InvalidYear { .. } => "Invalid year, must be between 1970 and 2100",
            Self::InvalidMonth { .. } => "Invalid month, must be between 1 and 12",
            Self::InvalidDay { .. } => "Invalid day for month",
        }
    }
}

/// Errors that can occur when creating a Config
#[derive(Display, Error, Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    #[display("Epoch {epoch}ms is in the future (current time: {current}ms)")]
    FutureEpoch { epoch: u64, current: u64 },
    #[display(
        "Epoch is too old: {elapsed}ms elapsed exceeds maximum {max_duration}ms (configured with {bits} timestamp bits)"
    )]
    EpochExceedsCapacity {
        elapsed: u64,
        max_duration: u64,
        bits: u8,
    },
}

impl ConfigError {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::FutureEpoch { .. } => "Epoch is in the future",
            Self::EpochExceedsCapacity { .. } => "Epoch is too old for timestamp bit allocation",
        }
    }
}

/// Epoch timestamp in milliseconds since UNIX epoch
///
/// Provides type-safe epoch configuration with predefined presets
/// and convenience constructors.
///
/// # Examples
///
/// ```
/// use typedflake::Epoch;
///
/// // Use presets
/// let epoch = Epoch::DISCORD;
///
/// // From raw milliseconds
/// let epoch = Epoch::new(1735689600000);
///
/// // From date (const, panics on invalid)
/// const CUSTOM_EPOCH: Epoch = Epoch::from_date(2025, 1, 1);
///
/// // From date (fallible)
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let epoch = Epoch::try_from_date(2025, 1, 1)?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Epoch {
    millis: u64,
}

impl Epoch {
    /// Twitter's epoch (Nov 4, 2010 01:42:54 UTC)
    pub const TWITTER: Self = Self::new(1288834974657);

    /// Discord's epoch (Jan 1, 2015 00:00:00 UTC)
    pub const DISCORD: Self = Self::new(1420070400000);

    /// Instagram's epoch (Sep 20, 2011 00:00:21 UTC)
    pub const INSTAGRAM: Self = Self::new(1314220021721);

    /// Default epoch (Jan 1, 2025 00:00:00 UTC)
    pub const DEFAULT: Self = Self::new(1735689600000);

    /// Create a new Epoch from milliseconds since UNIX epoch
    pub const fn new(millis: u64) -> Self {
        Self { millis }
    }

    /// Create an Epoch from a date with validation (year, month, day)
    ///
    /// # Arguments
    /// - `year`: Year (1970-2100)
    /// - `month`: Month (1-12)
    /// - `day`: Day of month (1-31)
    pub const fn try_from_date(year: u16, month: u8, day: u8) -> Result<Self, EpochError> {
        // Validate year
        if year < 1970 || year > 2100 {
            return Err(EpochError::InvalidYear { year });
        }

        // Validate month
        if month < 1 || month > 12 {
            return Err(EpochError::InvalidMonth { month });
        }

        // Days in each month (non-leap year)
        let days_in_month = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                // Check for leap year
                if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                    29
                } else {
                    28
                }
            }
            _ => unreachable!(),
        };

        // Validate day
        if day < 1 || day > days_in_month {
            return Err(EpochError::InvalidDay { day, month });
        }

        // Calculate days since UNIX epoch (Jan 1, 1970)
        let mut days = 0i64;

        // Add days for complete years
        let mut y: u16 = 1970;
        while y < year {
            days += if (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0) {
                366
            } else {
                365
            };
            y += 1;
        }

        // Add days for complete months in current year
        let mut m = 1;
        while m < month {
            days += match m {
                1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
                4 | 6 | 9 | 11 => 30,
                2 => {
                    if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                        29
                    } else {
                        28
                    }
                }
                _ => unreachable!(),
            } as i64;
            m += 1;
        }

        // Add remaining days (subtract 1 because day 1 = 0 days elapsed)
        days += (day - 1) as i64;

        // Convert to milliseconds
        let millis = (days * 24 * 60 * 60 * 1000) as u64;

        Ok(Self::new(millis))
    }

    /// Create an Epoch from a date (year, month, day)
    ///
    /// Panics if the date is invalid. Use `try_from_date` for fallible construction.
    pub const fn from_date(year: u16, month: u8, day: u8) -> Self {
        match Self::try_from_date(year, month, day) {
            Ok(epoch) => epoch,
            Err(error) => panic!("{}", error.as_str()),
        }
    }

    /// Create an Epoch from UNIX timestamp in seconds
    pub const fn from_seconds(secs: i64) -> Self {
        Self::new((secs * 1000) as u64)
    }

    /// Get milliseconds since UNIX epoch
    pub const fn as_millis(&self) -> u64 {
        self.millis
    }
}

impl fmt::Display for Epoch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}ms", self.millis)
    }
}

/// Bit allocation configuration for ID generation
///
/// Specifies how the 64-bit ID space is divided between timestamp, worker, process, and sequence components.
/// All bits must sum to 64.
///
/// # Examples
///
/// ```
/// use typedflake::BitLayout;
///
/// // Use industry-standard presets
/// let twitter = BitLayout::TWITTER;  // 42t|5w|5p|12s
/// let discord = BitLayout::DISCORD;  // 42t|5w|5p|12s
///
/// // Create custom allocation
/// let custom = BitLayout::new(45, 8, 4, 7);
///
/// // Check capacity
/// assert_eq!(custom.worker_max(), 255);
/// assert_eq!(custom.ids_per_millisecond(), 128);
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BitLayout {
    timestamp: u8,
    worker: u8,
    process: u8,
    sequence: u8,
}

impl BitLayout {
    /// Twitter Snowflake-inspired allocation (42t|5w|5p|12s)
    /// - 139 years lifespan
    /// - 1024 instances (32 workers × 32 processes)
    /// - 4096 IDs per millisecond
    ///
    /// Note: Twitter's original uses a sign bit; we allocate it to timestamp
    pub const TWITTER: Self = Self::new(42, 5, 5, 12);

    /// Discord's allocation (42t|5w|5p|12s)
    /// - 139 years lifespan
    /// - 1024 instances (32 workers × 32 processes)
    /// - 4096 IDs per millisecond
    pub const DISCORD: Self = Self::new(42, 5, 5, 12);

    /// Default allocation matching Config::DEFAULT (42t|5w|5p|12s)
    /// - 139 years lifespan
    /// - 1024 instances (32 workers × 32 processes)
    /// - 4096 IDs per millisecond
    pub const DEFAULT: Self = Self::new(42, 5, 5, 12);

    /// Create a new BitLayout from individual components
    ///
    /// # Panics
    ///
    /// Panics if the bit allocation is invalid (in const context).
    /// Use `validate()` for runtime checking.
    pub const fn new(timestamp: u8, worker: u8, process: u8, sequence: u8) -> Self {
        // Const validation - will panic at compile time for const contexts
        match Self::try_new(timestamp, worker, process, sequence) {
            Ok(layout) => layout,
            Err(error) => panic!("{}", error.as_str()),
        }
    }

    /// Create a new BitLayout with runtime validation
    pub const fn try_new(
        timestamp: u8,
        worker: u8,
        process: u8,
        sequence: u8,
    ) -> Result<Self, BitLayoutError> {
        let sum = timestamp as u16 + worker as u16 + process as u16 + sequence as u16;
        if sum != 64 {
            return Err(BitLayoutError::InvalidSum { actual: sum as u8 });
        }
        if timestamp == 0 {
            return Err(BitLayoutError::ZeroTimestampBits);
        }
        if sequence == 0 {
            return Err(BitLayoutError::ZeroSequenceBits);
        }
        Ok(Self {
            timestamp,
            worker,
            process,
            sequence,
        })
    }

    /// Get timestamp bits
    pub const fn timestamp(&self) -> u8 {
        self.timestamp
    }

    /// Get worker bits
    pub const fn worker(&self) -> u8 {
        self.worker
    }

    /// Get process bits
    pub const fn process(&self) -> u8 {
        self.process
    }

    /// Get sequence bits
    pub const fn sequence(&self) -> u8 {
        self.sequence
    }

    /// Total number of possible instances (2^worker_bits × 2^process_bits)
    pub const fn total_instances(&self) -> u64 {
        (1u64 << self.worker) * (1u64 << self.process)
    }

    /// Number of IDs that can be generated per millisecond (2^sequence_bits)
    pub const fn ids_per_millisecond(&self) -> u64 {
        1u64 << self.sequence
    }

    /// Duration in milliseconds before timestamp wraps around
    pub const fn timestamp_duration_ms(&self) -> u64 {
        self.timestamp_max()
    }

    /// Duration in years before timestamp wraps around (approximate)
    pub fn timestamp_duration_years(&self) -> f64 {
        // ms -> seconds -> minutes -> hours -> days -> years
        // Avoid overflow by doing division first
        let ms = self.timestamp_max() as f64;
        ms / 1000.0 / 60.0 / 60.0 / 24.0 / 365.25
    }

    // ===== Bit Manipulation: Shifts =====

    /// Shift amount for sequence component (always 0 - sequence is in LSB)
    pub const fn sequence_shift(&self) -> u8 {
        0
    }

    /// Shift amount for process component
    pub const fn process_shift(&self) -> u8 {
        self.sequence
    }

    /// Shift amount for worker component
    pub const fn worker_shift(&self) -> u8 {
        self.sequence + self.process
    }

    /// Shift amount for timestamp component
    pub const fn timestamp_shift(&self) -> u8 {
        self.sequence + self.process + self.worker
    }

    // ===== Bit Manipulation: Maximum Values =====

    /// Maximum sequence value (2^sequence_bits - 1)
    /// Also serves as bitmask for extracting sequence component
    pub const fn sequence_max(&self) -> u64 {
        (1u64 << self.sequence) - 1
    }

    /// Maximum process ID value (2^process_bits - 1)
    /// Also serves as bitmask for extracting process component
    pub const fn process_max(&self) -> u64 {
        (1u64 << self.process) - 1
    }

    /// Maximum worker ID value (2^worker_bits - 1)
    /// Also serves as bitmask for extracting worker component
    pub const fn worker_max(&self) -> u64 {
        (1u64 << self.worker) - 1
    }

    /// Maximum timestamp value (2^timestamp_bits - 1)
    /// Also serves as bitmask for extracting timestamp component
    pub const fn timestamp_max(&self) -> u64 {
        (1u64 << self.timestamp) - 1
    }
}

impl Default for BitLayout {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl fmt::Debug for BitLayout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BitLayout {{ timestamp: {}, worker: {}, process: {}, sequence: {} }}\n\
             Capacity: {:.1} years, {} instances, {} IDs/ms",
            self.timestamp(),
            self.worker(),
            self.process(),
            self.sequence(),
            self.timestamp_duration_years(),
            self.total_instances(),
            self.ids_per_millisecond()
        )
    }
}

impl fmt::Display for BitLayout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BitLayout[{}t|{}w|{}p|{}s]",
            self.timestamp(),
            self.worker(),
            self.process(),
            self.sequence()
        )
    }
}

impl From<(u8, u8, u8, u8)> for BitLayout {
    fn from((timestamp, worker, process, sequence): (u8, u8, u8, u8)) -> Self {
        Self::new(timestamp, worker, process, sequence)
    }
}

/// Validation error for component bounds checking
#[derive(Display, Error, From, Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    #[display("Timestamp {provided} exceeds maximum {maximum} (configured with {bits} bits)")]
    TimestampOutOfRange {
        provided: u64,
        maximum: u64,
        bits: u8,
    },
    #[display("Worker ID {provided} exceeds maximum {maximum} (configured with {bits} bits)")]
    WorkerIdOutOfRange {
        provided: u64,
        maximum: u64,
        bits: u8,
    },
    #[display("Process ID {provided} exceeds maximum {maximum} (configured with {bits} bits)")]
    ProcessIdOutOfRange {
        provided: u64,
        maximum: u64,
        bits: u8,
    },
    #[display("Sequence {provided} exceeds maximum {maximum} (configured with {bits} bits)")]
    SequenceOutOfRange {
        provided: u64,
        maximum: u64,
        bits: u8,
    },
    #[display("Failed to parse ID from string")]
    #[from]
    StringParseError(std::num::ParseIntError),
}

impl ValidationError {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::TimestampOutOfRange { .. } => "Timestamp exceeds maximum",
            Self::WorkerIdOutOfRange { .. } => "Worker ID exceeds maximum",
            Self::ProcessIdOutOfRange { .. } => "Process ID exceeds maximum",
            Self::SequenceOutOfRange { .. } => "Sequence exceeds maximum",
            Self::StringParseError(_) => "Failed to parse ID from string",
        }
    }
}

/// Configuration - contains bit allocation (via BitLayout) and epoch
///
/// All bit manipulation values (shifts, masks) are computed from the embedded BitLayout
/// using const fn methods, enabling compile-time evaluation while eliminating duplication.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    layout: BitLayout,
    epoch: Epoch,
}

impl Config {
    /// Twitter Snowflake-inspired configuration (42t|5w|5p|12s)
    /// - 139 years lifespan
    /// - 1024 instances (32 workers × 32 processes)
    /// - 4096 IDs per millisecond per worker
    /// - Epoch: Nov 4, 2010 01:42:54 UTC
    pub const TWITTER: Self = Self::new_unchecked(BitLayout::TWITTER, Epoch::TWITTER);

    /// Discord's configuration (42t|5w|5p|12s)
    /// - 139 years lifespan
    /// - 1024 instances (32 workers × 32 processes)
    /// - 4096 IDs per millisecond per instance
    /// - Epoch: Jan 1, 2015 00:00:00 UTC
    pub const DISCORD: Self = Self::new_unchecked(BitLayout::DISCORD, Epoch::DISCORD);

    /// Default configuration (42t|5w|5p|12s)
    /// - 139 years lifespan
    /// - 1024 instances (32 workers × 32 processes)
    /// - 4096 IDs per millisecond per instance
    /// - Epoch: Jan 1, 2025 00:00:00 UTC
    pub const DEFAULT: Self = Self::new_unchecked(BitLayout::DEFAULT, Epoch::DEFAULT);

    /// Create a new Config without validation (const-compatible)
    pub const fn new_unchecked(layout: BitLayout, epoch: Epoch) -> Self {
        Config { layout, epoch }
    }

    /// Create a new Config with validation
    ///
    /// Validates that the epoch is not in the future and that elapsed time
    /// since epoch doesn't exceed the timestamp bit capacity.
    pub fn try_new(layout: BitLayout, epoch: Epoch) -> Result<Self, ConfigError> {
        // Get current time in milliseconds since UNIX epoch
        let current_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time is before UNIX epoch")
            .as_millis() as u64;

        let epoch_ms = epoch.as_millis();

        // Validate epoch is not in the future
        if epoch_ms > current_ms {
            return Err(ConfigError::FutureEpoch {
                epoch: epoch_ms,
                current: current_ms,
            });
        }

        // Validate elapsed time doesn't exceed timestamp capacity
        let elapsed = current_ms - epoch_ms;
        let max_duration = layout.timestamp_max();

        if elapsed > max_duration {
            return Err(ConfigError::EpochExceedsCapacity {
                elapsed,
                max_duration,
                bits: layout.timestamp(),
            });
        }

        Ok(Config { layout, epoch })
    }

    /// Get the bit layout configuration
    pub const fn layout(&self) -> BitLayout {
        self.layout
    }

    /// Get the epoch timestamp
    pub const fn epoch(&self) -> Epoch {
        self.epoch
    }

    /// Get current timestamp relative to epoch
    pub(crate) fn current_timestamp_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis() as u64
            - self.epoch().as_millis()
    }

    /// Validate worker_id/process_id against bit limits
    pub fn validate_instance(
        &self,
        worker_id: u64,
        process_id: u64,
    ) -> Result<(), ValidationError> {
        if worker_id > self.layout().worker_max() {
            return Err(ValidationError::WorkerIdOutOfRange {
                provided: worker_id,
                maximum: self.layout().worker_max(),
                bits: self.layout().worker(),
            });
        }
        if process_id > self.layout().process_max() {
            return Err(ValidationError::ProcessIdOutOfRange {
                provided: process_id,
                maximum: self.layout().process_max(),
                bits: self.layout().process(),
            });
        }
        Ok(())
    }

    /// Validate all ID components against bit limits
    pub fn validate_components(
        &self,
        timestamp: u64,
        worker_id: u64,
        process_id: u64,
        sequence: u64,
    ) -> Result<(), ValidationError> {
        if timestamp > self.layout().timestamp_max() {
            return Err(ValidationError::TimestampOutOfRange {
                provided: timestamp,
                maximum: self.layout().timestamp_max(),
                bits: self.layout().timestamp(),
            });
        }
        if worker_id > self.layout().worker_max() {
            return Err(ValidationError::WorkerIdOutOfRange {
                provided: worker_id,
                maximum: self.layout().worker_max(),
                bits: self.layout().worker(),
            });
        }
        if process_id > self.layout().process_max() {
            return Err(ValidationError::ProcessIdOutOfRange {
                provided: process_id,
                maximum: self.layout().process_max(),
                bits: self.layout().process(),
            });
        }
        if sequence > self.layout().sequence_max() {
            return Err(ValidationError::SequenceOutOfRange {
                provided: sequence,
                maximum: self.layout().sequence_max(),
                bits: self.layout().sequence(),
            });
        }
        Ok(())
    }

    /// Validate a raw u64 ID by decomposing and checking component ranges
    pub fn validate_id(&self, id: u64) -> Result<(), ValidationError> {
        let timestamp = (id >> self.layout().timestamp_shift()) & self.layout().timestamp_max();
        let worker_id = (id >> self.layout().worker_shift()) & self.layout().worker_max();
        let process_id = (id >> self.layout().process_shift()) & self.layout().process_max();
        let sequence = (id >> self.layout().sequence_shift()) & self.layout().sequence_max();

        self.validate_components(timestamp, worker_id, process_id, sequence)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new_unchecked(BitLayout::DEFAULT, Epoch::DEFAULT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BitLayout;

    #[test]
    fn config_default() {
        let config = Config::default();
        let layout = config.layout();
        assert_eq!(
            layout.timestamp() + layout.worker() + layout.process() + layout.sequence(),
            64
        );
        assert!(layout.timestamp() > 0);
        assert!(layout.sequence() > 0);
    }

    #[test]
    fn config_new() {
        let config =
            Config::new_unchecked(BitLayout::new(42, 8, 4, 10), Epoch::new(1_600_000_000_000));

        assert_eq!(config.layout().timestamp(), 42);
        assert_eq!(config.layout().worker(), 8);
        assert_eq!(config.layout().process(), 4);
        assert_eq!(config.layout().sequence(), 10);
        assert_eq!(config.epoch().as_millis(), 1_600_000_000_000);

        // Check max values are calculated correctly via BitLayout methods
        assert_eq!(config.layout().worker_max(), (1u64 << 8) - 1); // 255
        assert_eq!(config.layout().process_max(), (1u64 << 4) - 1); // 15
        assert_eq!(config.layout().sequence_max(), (1u64 << 10) - 1); // 1023
    }

    #[test]
    fn config_zero_process_bits() {
        let config = Config::new_unchecked(
            BitLayout::new(41, 10, 0, 13), // process_bits = 0
            Epoch::DEFAULT,
        );

        assert_eq!(config.layout().process(), 0);
        assert_eq!(config.layout().process_max(), 0);
    }

    #[test]
    fn validate_instance_boundaries_and_errors() {
        let layout = BitLayout::new(42, 8, 4, 10); // max_worker = 255, max_process = 15
        let config = Config::new_unchecked(layout, Epoch::new(1_600_000_000_000));

        // Valid instances - boundaries
        assert!(config.validate_instance(255, 15).is_ok());
        assert!(config.validate_instance(0, 0).is_ok());

        // Invalid worker_id - verify error details
        let worker_error = config.validate_instance(256, 0);
        assert!(worker_error.is_err());
        if let Err(ValidationError::WorkerIdOutOfRange {
            provided,
            maximum,
            bits,
        }) = worker_error
        {
            assert_eq!(provided, 256);
            assert_eq!(maximum, 255);
            assert_eq!(bits, 8);

            // Verify error message format
            let error_msg = format!(
                "{}",
                ValidationError::WorkerIdOutOfRange {
                    provided,
                    maximum,
                    bits,
                }
            );
            assert!(error_msg.contains("Worker ID"));
            assert!(error_msg.contains("256"));
            assert!(error_msg.contains("255"));
            assert!(error_msg.contains("8 bits"));
        } else {
            panic!("Expected WorkerIdOutOfRange error");
        }

        // Invalid process_id - verify error details
        let process_error = config.validate_instance(0, 16);
        assert!(process_error.is_err());
        if let Err(ValidationError::ProcessIdOutOfRange {
            provided,
            maximum,
            bits,
        }) = process_error
        {
            assert_eq!(provided, 16);
            assert_eq!(maximum, 15);
            assert_eq!(bits, 4);

            // Verify error message format
            let error_msg = format!(
                "{}",
                ValidationError::ProcessIdOutOfRange {
                    provided,
                    maximum,
                    bits,
                }
            );
            assert!(error_msg.contains("Process ID"));
            assert!(error_msg.contains("16"));
            assert!(error_msg.contains("15"));
            assert!(error_msg.contains("4 bits"));
        } else {
            panic!("Expected ProcessIdOutOfRange error");
        }
    }

    #[test]
    fn validate_components_success() {
        let layout = BitLayout::new(42, 8, 4, 10);
        let config = Config::new_unchecked(layout, Epoch::new(1_600_000_000_000));

        // Valid at boundaries
        let max_timestamp = (1u64 << 42) - 1;
        let max_worker = (1u64 << 8) - 1;
        let max_process = (1u64 << 4) - 1;
        let max_sequence = (1u64 << 10) - 1;

        assert!(
            config
                .validate_components(max_timestamp, max_worker, max_process, max_sequence)
                .is_ok()
        );

        // Valid at zero
        assert!(config.validate_components(0, 0, 0, 0).is_ok());

        // Valid at typical values
        assert!(config.validate_components(1000000, 50, 5, 100).is_ok());
    }

    #[test]
    fn validate_components_errors() {
        let layout = BitLayout::new(42, 8, 4, 10);
        let config = Config::new_unchecked(layout, Epoch::new(1_600_000_000_000));

        // Timestamp overflow
        let timestamp_err = config.validate_components(1u64 << 42, 0, 0, 0);
        assert!(matches!(
            timestamp_err,
            Err(ValidationError::TimestampOutOfRange { .. })
        ));

        // Worker overflow
        let worker_err = config.validate_components(0, 256, 0, 0);
        assert!(matches!(
            worker_err,
            Err(ValidationError::WorkerIdOutOfRange { .. })
        ));

        // Process overflow
        let process_err = config.validate_components(0, 0, 16, 0);
        assert!(matches!(
            process_err,
            Err(ValidationError::ProcessIdOutOfRange { .. })
        ));

        // Sequence overflow
        let sequence_err = config.validate_components(0, 0, 0, 1024);
        assert!(matches!(
            sequence_err,
            Err(ValidationError::SequenceOutOfRange { .. })
        ));
    }

    #[test]
    fn validate_id_from_raw_u64() {
        let layout = BitLayout::new(42, 8, 4, 10);
        let config = Config::new_unchecked(layout, Epoch::new(1_600_000_000_000));

        // Create a valid ID manually
        let timestamp = 1000u64;
        let worker_id = 50u64;
        let process_id = 5u64;
        let sequence = 100u64;

        let valid_id = (timestamp << layout.timestamp_shift())
            | (worker_id << layout.worker_shift())
            | (process_id << layout.process_shift())
            | (sequence << layout.sequence_shift());

        assert!(config.validate_id(valid_id).is_ok());

        // Zero ID should be valid
        assert!(config.validate_id(0).is_ok());

        // Maximum valid ID - create with all components at their max values
        let max_timestamp = (1u64 << 42) - 1;
        let max_worker = (1u64 << 8) - 1;
        let max_process = (1u64 << 4) - 1;
        let max_sequence = (1u64 << 10) - 1;

        let max_valid_id = (max_timestamp << layout.timestamp_shift())
            | (max_worker << layout.worker_shift())
            | (max_process << layout.process_shift())
            | (max_sequence << layout.sequence_shift());

        assert!(config.validate_id(max_valid_id).is_ok());
    }

    #[test]
    fn epoch_const_from_date() {
        // Test const fn from_date (panicking version)
        const EPOCH_2025: Epoch = Epoch::from_date(2025, 1, 1);
        assert_eq!(EPOCH_2025.as_millis(), 1735689600000);

        const EPOCH_2024: Epoch = Epoch::from_date(2024, 6, 15);
        assert!(EPOCH_2024.as_millis() > 0);

        // Test const fn try_from_date (Result version)
        const EPOCH_RESULT: Result<Epoch, EpochError> = Epoch::try_from_date(2025, 12, 31);
        assert!(EPOCH_RESULT.is_ok());

        // Test in Config
        const CUSTOM_CONFIG: Config =
            Config::new_unchecked(BitLayout::DEFAULT, Epoch::from_date(2025, 3, 15));
        assert!(CUSTOM_CONFIG.epoch().as_millis() > 0);
    }

    #[test]
    fn epoch_try_from_date_errors() {
        // Invalid year
        assert!(Epoch::try_from_date(1969, 1, 1).is_err());
        assert!(Epoch::try_from_date(2101, 1, 1).is_err());

        // Invalid month
        assert!(Epoch::try_from_date(2025, 0, 1).is_err());
        assert!(Epoch::try_from_date(2025, 13, 1).is_err());

        // Invalid day
        assert!(Epoch::try_from_date(2025, 2, 30).is_err());
        assert!(Epoch::try_from_date(2025, 4, 31).is_err());

        // Leap year edge case
        assert!(Epoch::try_from_date(2024, 2, 29).is_ok()); // 2024 is leap year
        assert!(Epoch::try_from_date(2025, 2, 29).is_err()); // 2025 is not
    }

    #[test]
    fn config_try_new_valid() {
        // Valid config with past epoch
        let layout = BitLayout::DEFAULT;
        let epoch = Epoch::new(1_600_000_000_000); // Sept 2020
        let config = Config::try_new(layout, epoch);
        assert!(config.is_ok());

        let config = config.unwrap();
        assert_eq!(config.layout(), layout);
        assert_eq!(config.epoch(), epoch);
    }

    #[test]
    fn config_try_new_future_epoch() {
        let layout = BitLayout::DEFAULT;
        // Far future epoch (year 2286)
        let future_epoch = Epoch::new(9_999_999_999_999);

        let result = Config::try_new(layout, future_epoch);
        assert!(result.is_err());

        if let Err(ConfigError::FutureEpoch { epoch, current }) = result {
            assert_eq!(epoch, 9_999_999_999_999);
            assert!(current < epoch);
        } else {
            panic!("Expected FutureEpoch error");
        }
    }

    #[test]
    fn config_try_new_epoch_exceeds_capacity() {
        // Use small timestamp bits to force capacity overflow
        let layout = BitLayout::new(20, 10, 10, 24); // Only 20 timestamp bits (~12 days)

        // Epoch from 2 years ago will exceed 20-bit capacity
        let current = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let two_years_ago = current - (2 * 365 * 24 * 60 * 60 * 1000);
        let old_epoch = Epoch::new(two_years_ago);

        let result = Config::try_new(layout, old_epoch);
        assert!(result.is_err());

        if let Err(ConfigError::EpochExceedsCapacity {
            elapsed,
            max_duration,
            bits,
        }) = result
        {
            assert!(elapsed > max_duration);
            assert_eq!(bits, 20);
            assert_eq!(max_duration, (1u64 << 20) - 1);
        } else {
            panic!("Expected EpochExceedsCapacity error");
        }
    }

    #[test]
    fn config_try_new_edge_cases() {
        let layout = BitLayout::DEFAULT;

        // Epoch exactly at current time should work (edge case)
        let current = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let now_epoch = Epoch::new(current);
        assert!(Config::try_new(layout, now_epoch).is_ok());

        // Epoch 1 millisecond ago should work
        let one_ms_ago = Epoch::new(current - 1);
        assert!(Config::try_new(layout, one_ms_ago).is_ok());
    }

    #[test]
    fn config_try_new_with_presets() {
        // All preset epochs should be valid with their corresponding layouts
        assert!(Config::try_new(BitLayout::TWITTER, Epoch::TWITTER).is_ok());
        assert!(Config::try_new(BitLayout::DISCORD, Epoch::DISCORD).is_ok());

        // Cross-combination should also work (both are 42-bit layouts from ~2010-2015)
        assert!(Config::try_new(BitLayout::TWITTER, Epoch::DISCORD).is_ok());
        assert!(Config::try_new(BitLayout::DISCORD, Epoch::TWITTER).is_ok());
    }
}
