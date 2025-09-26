use crate::config::Config;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

/// Atomic state for each (worker_id, process_id) instance
#[derive(Debug)]
pub struct State {
    /// Packed state: upper bits = timestamp, lower bits = sequence
    pub packed: AtomicU64,
}

impl State {
    pub fn new(packed_state: u64) -> Self {
        Self {
            packed: AtomicU64::new(packed_state),
        }
    }

    /// Pack timestamp and sequence into single u64
    pub fn pack_state(&self, timestamp: u64, sequence: u64, config: Config) -> u64 {
        (timestamp << config.bits.sequence) | (sequence & config.cached.sequence_mask)
    }

    /// Unpack timestamp and sequence from single u64
    pub fn unpack_state(&self, packed: u64, config: Config) -> (u64, u64) {
        let timestamp = packed >> config.bits.sequence;
        let sequence = packed & config.cached.sequence_mask;
        (timestamp, sequence)
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            packed: AtomicU64::new(0),
        }
    }
}

/// Direct state vector with mathematical indexing
#[derive(Debug)]
pub struct StateVec {
    pub states: Vec<Arc<State>>,
    config: Config,
}

impl StateVec {
    /// Create with pre-allocated states for all possible (worker_id, process_id) combinations
    pub fn new(config: Config) -> Self {
        // Calculate total size based on bit allocation
        let max_workers = config.cached.max_worker_id + 1;
        let max_processes = config.cached.max_process_id + 1;
        let total_size = (max_workers * max_processes) as usize;

        Self {
            states: (0..total_size)
                .map(|_| Arc::new(State::default()))
                .collect(),
            config,
        }
    }

    /// Compute array index using mathematical formula based on config.cached values
    #[inline]
    fn compute_index(&self, worker_id: u64, process_id: u64) -> usize {
        // Use pre-calculated masks from config.cached for bounds safety
        let masked_worker = worker_id & self.config.cached.worker_mask;
        let masked_process = process_id & self.config.cached.process_mask;

        // Mathematical mapping: index = worker_id * max_processes + process_id
        // This uses the same shift logic as ID composition but for indexing
        let index = (masked_worker << self.config.bits.process) | masked_process;
        index as usize
    }

    /// Get state for (worker_id, process_id) - guaranteed O(1) direct access
    #[inline]
    pub fn get_state(&self, worker_id: u64, process_id: u64) -> &Arc<State> {
        let index = self.compute_index(worker_id, process_id);
        // Safe: index guaranteed in bounds by masking
        &self.states[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;
    use std::sync::Arc;

    #[test]
    fn test_direct_state_vec_creation() {
        let config = Config::new((41, 10, 5, 8), Config::DEFAULT_EPOCH_MS);
        let state_vec = StateVec::new(config);

        // Should create states for all combinations: (2^10) * (2^5) = 32768 states
        assert_eq!(state_vec.states.len(), 32768);
    }

    #[test]
    fn test_direct_state_vec_indexing() {
        let config = Config::new((41, 10, 5, 8), Config::DEFAULT_EPOCH_MS);
        let state_vec = StateVec::new(config);

        // Test mathematical indexing
        let state1 = state_vec.get_state(0, 0);
        let state2 = state_vec.get_state(1, 0);
        let state3 = state_vec.get_state(0, 1);

        // Different worker/process should give different states
        assert!(!Arc::ptr_eq(state1, state2));
        assert!(!Arc::ptr_eq(state1, state3));
        assert!(!Arc::ptr_eq(state2, state3));

        // Same worker/process should give same state
        let state1_again = state_vec.get_state(0, 0);
        assert!(Arc::ptr_eq(state1, state1_again));
    }
}
