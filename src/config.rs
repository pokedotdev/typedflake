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
//! use typedflake::{BitLayout, Config};
//!
//! // Twitter Snowflake-inspired (42t|5w|5p|12s)
//! const TWITTER_CONFIG: Config = Config::new(BitLayout::TWITTER, 1288834974657);
//!
//! // Discord's allocation (42t|5w|5p|12s)
//! const DISCORD_CONFIG: Config = Config::new(BitLayout::DISCORD, 1420070400000);
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

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Errors that can occur when creating or validating a BitLayout
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum BitLayoutError {
    #[error("Bit allocation must sum to 64, got {actual}")]
    InvalidSum { actual: u8 },
    #[error("Timestamp bits must be greater than 0")]
    ZeroTimestampBits,
    #[error("Sequence bits must be greater than 0")]
    ZeroSequenceBits,
    #[error("Field '{field}' has {bits} bits which exceeds maximum of 64")]
    BitsExceedMaximum { field: &'static str, bits: u8 },
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
    pub timestamp: u8,
    pub worker: u8,
    pub process: u8,
    pub sequence: u8,
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
        assert!(
            timestamp as u16 + worker as u16 + process as u16 + sequence as u16 == 64,
            "Bit allocation must sum to 64"
        );
        assert!(timestamp > 0, "Timestamp bits must be > 0");
        assert!(sequence > 0, "Sequence bits must be > 0");

        Self {
            timestamp,
            worker,
            process,
            sequence,
        }
    }

    /// Validate this BitLayout
    pub const fn validate(&self) -> Result<(), BitLayoutError> {
        let sum =
            self.timestamp as u16 + self.worker as u16 + self.process as u16 + self.sequence as u16;
        if sum != 64 {
            return Err(BitLayoutError::InvalidSum { actual: sum as u8 });
        }
        if self.timestamp == 0 {
            return Err(BitLayoutError::ZeroTimestampBits);
        }
        if self.sequence == 0 {
            return Err(BitLayoutError::ZeroSequenceBits);
        }
        Ok(())
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
            self.timestamp,
            self.worker,
            self.process,
            self.sequence,
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
            self.timestamp, self.worker, self.process, self.sequence
        )
    }
}

impl From<(u8, u8, u8, u8)> for BitLayout {
    fn from((timestamp, worker, process, sequence): (u8, u8, u8, u8)) -> Self {
        Self::new(timestamp, worker, process, sequence)
    }
}

/// Validation error for component bounds checking
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    #[error("Timestamp {provided} exceeds maximum {maximum} (configured with {bits} bits)")]
    TimestampOutOfRange {
        provided: u64,
        maximum: u64,
        bits: u8,
    },
    #[error("Worker ID {provided} exceeds maximum {maximum} (configured with {bits} bits)")]
    WorkerIdOutOfRange {
        provided: u64,
        maximum: u64,
        bits: u8,
    },
    #[error("Process ID {provided} exceeds maximum {maximum} (configured with {bits} bits)")]
    ProcessIdOutOfRange {
        provided: u64,
        maximum: u64,
        bits: u8,
    },
    #[error("Sequence {provided} exceeds maximum {maximum} (configured with {bits} bits)")]
    SequenceOutOfRange {
        provided: u64,
        maximum: u64,
        bits: u8,
    },
    #[error("Invalid ID format: {0}")]
    ParseError(String),
}

/// Configuration - contains bit allocation (via BitLayout) and epoch
///
/// All bit manipulation values (shifts, masks) are computed from the embedded BitLayout
/// using const fn methods, enabling compile-time evaluation while eliminating duplication.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    /// Bit allocation configuration
    pub layout: BitLayout,
    /// Epoch in milliseconds since UNIX epoch
    pub epoch_ms: u64,
}

impl Config {
    /// Default epoch in milliseconds
    pub const DEFAULT_EPOCH_MS: u64 = 1735689600000; // ISO-8601 2025-01-01T00:00:00.000Z

    /// Create a new Config from BitLayout and epoch
    pub const fn new(layout: BitLayout, epoch_ms: u64) -> Self {
        // BitLayout validation is handled in BitLayout::new()
        Config { layout, epoch_ms }
    }

    /// Get current timestamp relative to epoch
    pub(crate) fn current_timestamp_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis() as u64
            - self.epoch_ms
    }

    /// Validate worker_id/process_id against bit limits
    pub fn validate_instance(
        &self,
        worker_id: u64,
        process_id: u64,
    ) -> Result<(), ValidationError> {
        if worker_id > self.layout.worker_max() {
            return Err(ValidationError::WorkerIdOutOfRange {
                provided: worker_id,
                maximum: self.layout.worker_max(),
                bits: self.layout.worker,
            });
        }
        if process_id > self.layout.process_max() {
            return Err(ValidationError::ProcessIdOutOfRange {
                provided: process_id,
                maximum: self.layout.process_max(),
                bits: self.layout.process,
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
        if timestamp > self.layout.timestamp_max() {
            return Err(ValidationError::TimestampOutOfRange {
                provided: timestamp,
                maximum: self.layout.timestamp_max(),
                bits: self.layout.timestamp,
            });
        }
        if worker_id > self.layout.worker_max() {
            return Err(ValidationError::WorkerIdOutOfRange {
                provided: worker_id,
                maximum: self.layout.worker_max(),
                bits: self.layout.worker,
            });
        }
        if process_id > self.layout.process_max() {
            return Err(ValidationError::ProcessIdOutOfRange {
                provided: process_id,
                maximum: self.layout.process_max(),
                bits: self.layout.process,
            });
        }
        if sequence > self.layout.sequence_max() {
            return Err(ValidationError::SequenceOutOfRange {
                provided: sequence,
                maximum: self.layout.sequence_max(),
                bits: self.layout.sequence,
            });
        }
        Ok(())
    }

    /// Validate a raw u64 ID by decomposing and checking component ranges
    pub fn validate_id(&self, id: u64) -> Result<(), ValidationError> {
        let timestamp = (id >> self.layout.timestamp_shift()) & self.layout.timestamp_max();
        let worker_id = (id >> self.layout.worker_shift()) & self.layout.worker_max();
        let process_id = (id >> self.layout.process_shift()) & self.layout.process_max();
        let sequence = (id >> self.layout.sequence_shift()) & self.layout.sequence_max();

        self.validate_components(timestamp, worker_id, process_id, sequence)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new(BitLayout::DEFAULT, Self::DEFAULT_EPOCH_MS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BitLayout;

    #[test]
    fn config_default() {
        let config = Config::default();
        let layout = config.layout;
        assert_eq!(
            layout.timestamp + layout.worker + layout.process + layout.sequence,
            64
        );
        assert!(layout.timestamp > 0);
        assert!(layout.sequence > 0);
    }

    #[test]
    fn config_new() {
        let config = Config::new(
            BitLayout::new(42, 8, 4, 10), // bits: timestamp, worker, process, sequence
            1_600_000_000_000,
        );

        assert_eq!(config.layout.timestamp, 42);
        assert_eq!(config.layout.worker, 8);
        assert_eq!(config.layout.process, 4);
        assert_eq!(config.layout.sequence, 10);
        assert_eq!(config.epoch_ms, 1_600_000_000_000);

        // Check max values are calculated correctly via BitLayout methods
        assert_eq!(config.layout.worker_max(), (1u64 << 8) - 1); // 255
        assert_eq!(config.layout.process_max(), (1u64 << 4) - 1); // 15
        assert_eq!(config.layout.sequence_max(), (1u64 << 10) - 1); // 1023
    }

    #[test]
    fn config_zero_process_bits() {
        let config = Config::new(
            BitLayout::new(41, 10, 0, 13), // process_bits = 0
            Config::DEFAULT_EPOCH_MS,
        );

        assert_eq!(config.layout.process, 0);
        assert_eq!(config.layout.process_max(), 0);
    }

    #[test]
    fn validate_instance_boundaries_and_errors() {
        let layout = BitLayout::new(42, 8, 4, 10); // max_worker = 255, max_process = 15
        let config = Config::new(layout, 1_600_000_000_000);

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
        let config = Config::new(layout, 1_600_000_000_000);

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
        let config = Config::new(layout, 1_600_000_000_000);

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
        let config = Config::new(layout, 1_600_000_000_000);

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
}
