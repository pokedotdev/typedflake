use crate::config::Config;
use crate::state::State;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum GeneratorError {
    #[error("Sequence overflowed for timestamp {timestamp}")]
    SequenceExhausted { timestamp: u64 },
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
    pub fn new(config: Config, worker_id: u64, process_id: u64) -> Self {
        let state = Arc::new(State::default());
        Self::new_with_state(config, state, worker_id, process_id)
    }

    pub fn new_with_state(
        config: Config,
        state: Arc<State>,
        worker_id: u64,
        process_id: u64,
    ) -> Self {
        Self {
            config,
            state,
            worker_id,
            process_id,
        }
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
                if current_sequence >= self.config.cached.sequence_mask {
                    return Err(GeneratorError::SequenceExhausted { timestamp });
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
        let masked_timestamp = timestamp & self.config.cached.timestamp_mask;
        let masked_worker = self.worker_id & self.config.cached.worker_mask;
        let masked_process = self.process_id & self.config.cached.process_mask;
        let masked_sequence = sequence & self.config.cached.sequence_mask;

        (masked_timestamp << self.config.cached.timestamp_shift)
            | (masked_worker << self.config.cached.worker_shift)
            | (masked_process << self.config.cached.process_shift)
            | (masked_sequence << self.config.cached.sequence_shift)
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
        (id >> self.config.cached.timestamp_shift) & self.config.cached.timestamp_mask
    }

    /// Extract just the worker ID component from an ID
    pub fn extract_worker_id(&self, id: u64) -> u64 {
        (id >> self.config.cached.worker_shift) & self.config.cached.worker_mask
    }

    /// Extract just the process ID component from an ID
    pub fn extract_process_id(&self, id: u64) -> u64 {
        (id >> self.config.cached.process_shift) & self.config.cached.process_mask
    }

    /// Extract just the sequence component from an ID
    pub fn extract_sequence(&self, id: u64) -> u64 {
        (id >> self.config.cached.sequence_shift) & self.config.cached.sequence_mask
    }

    /// Compose an ID from individual components
    pub fn compose_custom(
        &self,
        timestamp: u64,
        worker_id: u64,
        process_id: u64,
        sequence: u64,
    ) -> u64 {
        let masked_timestamp = timestamp & self.config.cached.timestamp_mask;
        let masked_worker = worker_id & self.config.cached.worker_mask;
        let masked_process = process_id & self.config.cached.process_mask;
        let masked_sequence = sequence & self.config.cached.sequence_mask;

        (masked_timestamp << self.config.cached.timestamp_shift)
            | (masked_worker << self.config.cached.worker_shift)
            | (masked_process << self.config.cached.process_shift)
            | (masked_sequence << self.config.cached.sequence_shift)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;

    #[test]
    fn test_generator_generation() {
        let config = Config::new((41, 10, 5, 8), Config::DEFAULT_EPOCH_MS);
        let state = Arc::new(crate::state::State::default());
        let generator = Generator::new_with_state(config, state, 42, 7);

        // Generate IDs
        let id1 = generator.generate().unwrap();
        let id2 = generator.generate().unwrap();

        // IDs should be different
        assert_ne!(id1, id2);
        assert!(id1 > 0);
        assert!(id2 > 0);

        // Verify worker/process IDs
        assert_eq!(generator.worker_id(), 42);
        assert_eq!(generator.process_id(), 7);
    }

    #[test]
    fn test_generator_id_components() {
        let config = Config::new((41, 10, 5, 8), Config::DEFAULT_EPOCH_MS);
        let state = Arc::new(crate::state::State::default());
        let generator = Generator::new_with_state(config, state, 99, 3);

        let id = generator.generate().unwrap();
        let components = generator.components(id);

        // Verify components match generator config
        assert_eq!(components.worker_id, 99);
        assert_eq!(components.process_id, 3);
        assert!(components.timestamp > 0);
        assert!(components.sequence < config.cached.sequence_mask);
    }

    #[test]
    fn test_generator_decompose_compose() {
        let config = Config::new((41, 10, 5, 8), Config::DEFAULT_EPOCH_MS);
        let state = Arc::new(crate::state::State::default());
        let generator = Generator::new_with_state(config, state, 123, 15);

        let id = generator.generate().unwrap();
        let (timestamp, worker_id, process_id, sequence) = generator.decompose(id);
        let recomposed = generator.compose_custom(timestamp, worker_id, process_id, sequence);

        assert_eq!(id, recomposed);
        assert_eq!(worker_id, 123);
        assert_eq!(process_id, 15);
    }
}
