use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// (timestamp_bits, worker_bits, process_bits, sequence_bits)
type TupleBitAllocation = (u8, u8, u8, u8);

/// Validation error for worker_id and process_id bounds checking
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
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
}

/// Configuration - contains bit allocation, epoch, and pre-calculated values for performance
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    // Bit allocation
    pub timestamp_bits: u8,
    pub worker_bits: u8,
    pub process_bits: u8,
    pub sequence_bits: u8,

    // Epoch
    pub epoch_ms: u64,

    // Pre-calculated shifts for performance
    pub timestamp_shift: u8,
    pub worker_shift: u8,
    pub process_shift: u8,
    pub sequence_shift: u8,

    // Pre-calculated masks for performance
    pub timestamp_mask: u64,
    pub worker_mask: u64,
    pub process_mask: u64,
    pub sequence_mask: u64,
}

impl Config {
    /// Default bit allocation configuration: (42, 5, 5, 12)
    pub const DEFAULT_TIMESTAMP_BITS: u8 = 42;
    pub const DEFAULT_WORKER_BITS: u8 = 5;
    pub const DEFAULT_PROCESS_BITS: u8 = 5;
    pub const DEFAULT_SEQUENCE_BITS: u8 = 12;

    /// Default epoch in milliseconds
    pub const DEFAULT_EPOCH_MS: u64 = 1735689600000; // ISO-8601 2025-01-01T00:00:00.000Z

    pub const fn new(bits: TupleBitAllocation, epoch_ms: u64) -> Self {
        let (timestamp_bits, worker_bits, process_bits, sequence_bits) = bits;

        // Validation
        assert!(
            timestamp_bits + worker_bits + process_bits + sequence_bits == 64,
            "Bit configuration must sum to 64"
        );
        assert!(timestamp_bits > 0, "Timestamp bits must be > 0");
        assert!(sequence_bits > 0, "Sequence bits must be > 0");

        // Calculate shifts
        let sequence_shift = 0;
        let process_shift = sequence_bits;
        let worker_shift = sequence_bits + process_bits;
        let timestamp_shift = worker_shift + worker_bits;

        // Calculate masks
        let sequence_mask = (1u64 << sequence_bits) - 1;
        let process_mask = if process_bits == 0 {
            0
        } else {
            (1u64 << process_bits) - 1
        };
        let worker_mask = if worker_bits == 0 {
            0
        } else {
            (1u64 << worker_bits) - 1
        };
        let timestamp_mask = (1u64 << timestamp_bits) - 1;

        Config {
            timestamp_bits,
            worker_bits,
            process_bits,
            sequence_bits,
            epoch_ms,
            timestamp_shift,
            worker_shift,
            process_shift,
            sequence_shift,
            timestamp_mask,
            worker_mask,
            process_mask,
            sequence_mask,
        }
    }

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
        if worker_id > self.worker_mask {
            return Err(ValidationError::WorkerIdOutOfRange {
                provided: worker_id,
                maximum: self.worker_mask,
                bits: self.worker_bits,
            });
        }
        if process_id > self.process_mask {
            return Err(ValidationError::ProcessIdOutOfRange {
                provided: process_id,
                maximum: self.process_mask,
                bits: self.process_bits,
            });
        }
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new(
            (
                Self::DEFAULT_TIMESTAMP_BITS,
                Self::DEFAULT_WORKER_BITS,
                Self::DEFAULT_PROCESS_BITS,
                Self::DEFAULT_SEQUENCE_BITS,
            ),
            Self::DEFAULT_EPOCH_MS,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_algorithm_config_default() {
        let config = Config::default();
        assert_eq!(
            config.timestamp_bits + config.worker_bits + config.process_bits + config.sequence_bits,
            64
        );
        assert!(config.timestamp_bits > 0);
        assert!(config.sequence_bits > 0);
    }

    #[test]
    fn test_algorithm_config_new() {
        let config = Config::new(
            (42, 8, 4, 10), // bits: timestamp, worker, process, sequence
            1_600_000_000_000,
        );

        assert_eq!(config.timestamp_bits, 42);
        assert_eq!(config.worker_bits, 8);
        assert_eq!(config.process_bits, 4);
        assert_eq!(config.sequence_bits, 10);
        assert_eq!(config.epoch_ms, 1_600_000_000_000);

        // Check cached values are calculated correctly
        assert_eq!(config.worker_mask, (1u64 << 8) - 1); // 255
        assert_eq!(config.process_mask, (1u64 << 4) - 1); // 15
        assert_eq!(config.sequence_mask, (1u64 << 10) - 1); // 1023
    }

    #[test]
    fn test_algorithm_config_zero_process_bits() {
        let config = Config::new(
            (41, 10, 0, 13), // process_bits = 0
            Config::DEFAULT_EPOCH_MS,
        );

        assert_eq!(config.process_bits, 0);
        assert_eq!(config.process_mask, 0);
    }

    #[test]
    fn test_algorithm_config_validate_instance() {
        let config = Config::new(
            (42, 8, 4, 10), // max_worker = 255, max_process = 15
            1_600_000_000_000,
        );

        // Valid instances
        assert!(config.validate_instance(255, 15).is_ok());
        assert!(config.validate_instance(0, 0).is_ok());

        // Invalid worker_id
        assert!(config.validate_instance(256, 0).is_err());

        // Invalid process_id
        assert!(config.validate_instance(0, 16).is_err());
    }

    // Legacy Config tests for backward compatibility
    #[test]
    fn test_legacy_config_default() {
        let config = Config::default();
        assert_eq!(
            config.timestamp_bits + config.worker_bits + config.process_bits + config.sequence_bits,
            64
        );
        assert!(config.timestamp_bits > 0);
        assert!(config.sequence_bits > 0);
    }
}
