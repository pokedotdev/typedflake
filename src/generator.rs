use crate::config::{Config, ValidationError};
use crate::state::State;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum GeneratorError {
    #[error(
        "Sequence exhausted for timestamp {timestamp} on worker_id={worker_id}, process_id={process_id}"
    )]
    SequenceExhausted {
        timestamp: u64,
        worker_id: u64,
        process_id: u64,
    },
}

#[derive(Debug, Clone)]
pub struct IdComponents {
    pub timestamp: u64,
    pub worker_id: u64,
    pub process_id: u64,
    pub sequence: u64,
}

/// Generator with pre-injected state - no state lookup needed during generation
#[derive(Debug)]
pub struct Generator {
    config: Config,
    state: Arc<State>,
    worker_id: u64,
    process_id: u64,
}

impl Generator {
    pub fn new(config: Config, worker_id: u64, process_id: u64) -> Result<Self, ValidationError> {
        let state = Arc::new(State::default());
        Self::new_with_state(config, state, worker_id, process_id)
    }

    pub fn new_with_state(
        config: Config,
        state: Arc<State>,
        worker_id: u64,
        process_id: u64,
    ) -> Result<Self, ValidationError> {
        config.validate_instance(worker_id, process_id)?;
        Ok(Self {
            config,
            state,
            worker_id,
            process_id,
        })
    }

    /// Generate ID with direct state access
    pub fn generate(&self) -> Result<u64, GeneratorError> {
        let current_timestamp = self.config.current_timestamp_ms();

        // Lock-free compare-and-swap retry loop using injected state
        loop {
            let current_packed = self.state.packed.load(Ordering::Acquire);
            let (last_timestamp, current_sequence) =
                self.state.unpack_state(current_packed, self.config);

            let timestamp = current_timestamp.max(last_timestamp);

            let (new_timestamp, new_sequence) = if timestamp == last_timestamp {
                if current_sequence >= self.config.layout.sequence_max() {
                    return Err(GeneratorError::SequenceExhausted {
                        timestamp,
                        worker_id: self.worker_id,
                        process_id: self.process_id,
                    });
                }
                (timestamp, current_sequence + 1)
            } else {
                (timestamp, 0)
            };

            let new_packed = self
                .state
                .pack_state(new_timestamp, new_sequence, self.config);

            match self.state.packed.compare_exchange_weak(
                current_packed,
                new_packed,
                Ordering::Release,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    let id = self.compose_id(new_timestamp, new_sequence);
                    return Ok(id);
                }
                Err(_) => continue,
            }
        }
    }

    /// Generate ID with blocking on sequence exhaustion
    pub fn generate_blocking(&self) -> u64 {
        loop {
            match self.generate() {
                Ok(id) => return id,
                Err(GeneratorError::SequenceExhausted { .. }) => {
                    self.wait_for_next_millis();
                    continue;
                }
            }
        }
    }

    /// Compose ID using injected worker_id and process_id
    #[inline]
    fn compose_id(&self, timestamp: u64, sequence: u64) -> u64 {
        let layout = &self.config.layout;
        let masked_timestamp = timestamp & layout.timestamp_max();
        let masked_worker = self.worker_id & layout.worker_max();
        let masked_process = self.process_id & layout.process_max();
        let masked_sequence = sequence & layout.sequence_max();

        (masked_timestamp << layout.timestamp_shift())
            | (masked_worker << layout.worker_shift())
            | (masked_process << layout.process_shift())
            | (masked_sequence << layout.sequence_shift())
    }

    /// Helper method for waiting until next millisecond
    fn wait_for_next_millis(&self) {
        let mut current = self.config.current_timestamp_ms();
        let start_time = current;

        while current <= start_time {
            std::thread::sleep(std::time::Duration::from_millis(1));
            current = self.config.current_timestamp_ms();
        }
    }

    /// Get the worker_id this generator is bound to
    pub fn worker_id(&self) -> u64 {
        self.worker_id
    }

    /// Get the process_id this generator is bound to
    pub fn process_id(&self) -> u64 {
        self.process_id
    }

    /// Get the ID components as a struct
    pub fn components(&self, id: u64) -> IdComponents {
        IdComponents {
            timestamp: self.extract_timestamp(id),
            worker_id: self.extract_worker_id(id),
            process_id: self.extract_process_id(id),
            sequence: self.extract_sequence(id),
        }
    }

    /// Decompose the ID into its components as a tuple
    pub fn decompose(&self, id: u64) -> (u64, u64, u64, u64) {
        let components = self.components(id);
        (
            components.timestamp,
            components.worker_id,
            components.process_id,
            components.sequence,
        )
    }

    /// Extract just the timestamp component from an ID
    pub fn extract_timestamp(&self, id: u64) -> u64 {
        let layout = &self.config.layout;
        (id >> layout.timestamp_shift()) & layout.timestamp_max()
    }

    /// Extract just the worker ID component from an ID
    pub fn extract_worker_id(&self, id: u64) -> u64 {
        let layout = &self.config.layout;
        (id >> layout.worker_shift()) & layout.worker_max()
    }

    /// Extract just the process ID component from an ID
    pub fn extract_process_id(&self, id: u64) -> u64 {
        let layout = &self.config.layout;
        (id >> layout.process_shift()) & layout.process_max()
    }

    /// Extract just the sequence component from an ID
    pub fn extract_sequence(&self, id: u64) -> u64 {
        let layout = &self.config.layout;
        (id >> layout.sequence_shift()) & layout.sequence_max()
    }

    /// Compose an ID from individual components
    pub fn compose_custom(
        &self,
        timestamp: u64,
        worker_id: u64,
        process_id: u64,
        sequence: u64,
    ) -> u64 {
        let layout = &self.config.layout;
        let masked_timestamp = timestamp & layout.timestamp_max();
        let masked_worker = worker_id & layout.worker_max();
        let masked_process = process_id & layout.process_max();
        let masked_sequence = sequence & layout.sequence_max();

        (masked_timestamp << layout.timestamp_shift())
            | (masked_worker << layout.worker_shift())
            | (masked_process << layout.process_shift())
            | (masked_sequence << layout.sequence_shift())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BitLayout;
    use crate::Config;

    #[test]
    fn generator_id_generation() {
        let config = Config::default();
        let generator = Generator::new(config, 5, 3).unwrap();

        // Generate IDs
        let id1 = generator.generate().unwrap();
        let id2 = generator.generate().unwrap();

        // IDs should be different
        assert_ne!(id1, id2);
        assert!(id1 > 0);
        assert!(id2 > 0);

        // Verify worker/process IDs
        assert_eq!(generator.worker_id(), 5);
        assert_eq!(generator.process_id(), 3);
    }

    #[test]
    fn extract_id_components() {
        let config = Config::default();
        let generator = Generator::new(config, 10, 2).unwrap();

        let id = generator.generate().unwrap();
        let components = generator.components(id);

        // Verify components match generator config
        assert_eq!(components.worker_id, 10);
        assert_eq!(components.process_id, 2);
        assert!(components.timestamp > 0);
        assert!(components.sequence <= config.layout.sequence_max());
    }

    #[test]
    fn decompose_compose_roundtrip() {
        let config = Config::default();
        let generator = Generator::new(config, 15, 7).unwrap();

        let id = generator.generate().unwrap();
        let (timestamp, worker_id, process_id, sequence) = generator.decompose(id);
        let recomposed = generator.compose_custom(timestamp, worker_id, process_id, sequence);

        assert_eq!(id, recomposed);
        assert_eq!(worker_id, 15);
        assert_eq!(process_id, 7);
    }

    #[test]
    fn max_value_composition_and_overflow() {
        // Test compose/decompose with maximum values for custom config
        let config = Config::new(
            BitLayout::new(42, 8, 4, 10), // 42 timestamp, 8 worker, 4 process, 10 sequence
            1_500_000_000_000,
        );
        let generator = Generator::new(config, 0, 0).unwrap();

        let max_timestamp = (1u64 << 42) - 1; // 42 bits
        let max_worker = (1u64 << 8) - 1; // 255
        let max_process = (1u64 << 4) - 1; // 15
        let max_sequence = (1u64 << 10) - 1; // 1023

        let composed =
            generator.compose_custom(max_timestamp, max_worker, max_process, max_sequence);
        let (dec_timestamp, dec_worker, dec_process, dec_sequence) = generator.decompose(composed);

        // Verify all components are preserved correctly
        assert_eq!(
            dec_timestamp, max_timestamp,
            "Timestamp should be preserved"
        );
        assert_eq!(dec_worker, max_worker, "Worker ID should be preserved");
        assert_eq!(dec_process, max_process, "Process ID should be preserved");
        assert_eq!(dec_sequence, max_sequence, "Sequence should be preserved");

        // Verify bit masking handles overflow correctly
        let overflow_timestamp = 1u64 << 50; // More than 42 bits
        let overflow_composed = generator.compose_custom(overflow_timestamp, 0, 0, 0);
        let overflow_dec = generator.decompose(overflow_composed);

        // Should be masked to 42 bits
        assert!(
            overflow_dec.0 <= max_timestamp,
            "Overflow timestamp should be masked"
        );
        assert_ne!(
            overflow_dec.0, overflow_timestamp,
            "Overflow should be truncated"
        );
    }
}
