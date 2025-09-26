use std::time::{SystemTime, UNIX_EPOCH};

/// (timestamp_bits, worker_bits, process_bits, sequence_bits)
type TupleBitAllocation = (u8, u8, u8, u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BitAllocation {
    pub timestamp: u8,
    pub worker: u8,
    pub process: u8,
    pub sequence: u8,
}

impl BitAllocation {
    pub const fn new(timestamp: u8, worker: u8, process: u8, sequence: u8) -> Self {
        assert!(
            timestamp + worker + process + sequence == 64,
            "Bit configuration must sum to 64"
        );
        assert!(timestamp > 0, "Timestamp bits must be > 0");
        assert!(sequence > 0, "Sequence bits must be > 0");

        BitAllocation {
            timestamp,
            worker,
            process,
            sequence,
        }
    }
}

impl Default for BitAllocation {
    fn default() -> Self {
        Config::DEFAULT_BITS
    }
}

impl From<TupleBitAllocation> for BitAllocation {
    fn from(bits: TupleBitAllocation) -> Self {
        BitAllocation::new(bits.0, bits.1, bits.2, bits.3)
    }
}

impl From<BitAllocation> for TupleBitAllocation {
    fn from(bits: BitAllocation) -> Self {
        (bits.timestamp, bits.worker, bits.process, bits.sequence)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CachedValues {
    pub timestamp_shift: u8,
    pub worker_shift: u8,
    pub process_shift: u8,
    pub sequence_shift: u8,

    pub timestamp_mask: u64,
    pub worker_mask: u64,
    pub process_mask: u64,
    pub sequence_mask: u64,
}

/// Configuration - contains bit allocation and epoch (compile-time only)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub bits: BitAllocation,
    pub epoch_ms: u64,
    pub cached: CachedValues,
}

impl Config {
    /// Default bit allocation configuration
    pub const DEFAULT_BITS: BitAllocation = BitAllocation::new(42, 5, 5, 12);
    /// Default epoch in milliseconds
    pub const DEFAULT_EPOCH_MS: u64 = 1735689600000; // ISO-8601 2025-01-01T00:00:00.000Z

    pub const fn new(bits: TupleBitAllocation, epoch_ms: u64) -> Self {
        let bits = BitAllocation::new(bits.0, bits.1, bits.2, bits.3);

        // Calculate shifts
        let sequence_shift = 0;
        let process_shift = bits.sequence;
        let worker_shift = bits.sequence + bits.process;
        let timestamp_shift = worker_shift + bits.worker;

        // Calculate masks
        let sequence_mask = (1u64 << bits.sequence) - 1;
        let process_mask = if bits.sequence == 0 {
            0
        } else {
            (1u64 << bits.process) - 1
        };
        let worker_mask = if bits.sequence == 0 {
            0
        } else {
            (1u64 << bits.worker) - 1
        };
        let timestamp_mask = (1u64 << bits.timestamp) - 1;

        Config {
            bits,
            epoch_ms,
            cached: CachedValues {
                timestamp_shift,
                worker_shift,
                process_shift,
                sequence_shift,
                timestamp_mask,
                worker_mask,
                process_mask,
                sequence_mask,
            },
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
    pub fn validate_instance(&self, worker_id: u64, process_id: u64) -> Result<(), String> {
        if worker_id > self.cached.worker_mask {
            return Err(format!(
                "Worker ID {} exceeds maximum {}",
                worker_id, self.cached.worker_mask
            ));
        }
        if process_id > self.cached.process_mask {
            return Err(format!(
                "Process ID {} exceeds maximum {}",
                process_id, self.cached.process_mask
            ));
        }
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new(Self::DEFAULT_BITS.into(), Self::DEFAULT_EPOCH_MS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_algorithm_config_default() {
        let config = Config::default();
        assert_eq!(
            config.bits.timestamp + config.bits.worker + config.bits.process + config.bits.sequence,
            64
        );
        assert!(config.bits.timestamp > 0);
        assert!(config.bits.sequence > 0);
    }

    #[test]
    fn test_algorithm_config_new() {
        let config = Config::new(
            (42, 10, 5, 7), // bits: timestamp, worker, process, sequence
            1_600_000_000_000,
        );

        assert_eq!(config.bits.timestamp, 42);
        assert_eq!(config.bits.worker, 10);
        assert_eq!(config.bits.process, 5);
        assert_eq!(config.bits.sequence, 7);
        assert_eq!(config.epoch_ms, 1_600_000_000_000);

        // Check cached values are calculated correctly
        assert_eq!(config.cached.worker_mask, (1u64 << 10) - 1); // 1023
        assert_eq!(config.cached.process_mask, (1u64 << 5) - 1); // 31
        assert_eq!(config.cached.sequence_mask, (1u64 << 7) - 1); // 127
    }

    #[test]
    fn test_algorithm_config_zero_process_bits() {
        let config = Config::new(
            (41, 10, 0, 13), // process_bits = 0
            Config::DEFAULT_EPOCH_MS,
        );

        assert_eq!(config.bits.process, 0);
        assert_eq!(config.cached.process_mask, 0);
    }

    #[test]
    fn test_algorithm_config_validate_instance() {
        let config = Config::new(
            (42, 10, 5, 7), // max_worker = 1023, max_process = 31
            1_600_000_000_000,
        );

        // Valid instances
        assert!(config.validate_instance(1023, 31).is_ok());
        assert!(config.validate_instance(0, 0).is_ok());

        // Invalid worker_id
        assert!(config.validate_instance(1024, 0).is_err());

        // Invalid process_id
        assert!(config.validate_instance(0, 32).is_err());
    }

    // Legacy Config tests for backward compatibility
    #[test]
    fn test_legacy_config_default() {
        let config = Config::default();
        assert_eq!(
            config.bits.timestamp + config.bits.worker + config.bits.process + config.bits.sequence,
            64
        );
        assert!(config.bits.timestamp > 0);
        assert!(config.bits.sequence > 0);
    }
}
