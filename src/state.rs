use crate::config::Config;
use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

/// Atomic state for each (worker_id, process_id) instance
#[derive(Debug)]
#[repr(align(64))] // Cache-line alignment to prevent false sharing across cores
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
    #[inline(always)]
    pub fn pack_state(&self, timestamp: u64, sequence: u64, config: Config) -> u64 {
        (timestamp << config.layout().sequence()) | (sequence & config.layout().sequence_max())
    }

    /// Unpack timestamp and sequence from single u64
    #[inline(always)]
    pub fn unpack_state(&self, packed: u64, config: Config) -> (u64, u64) {
        let timestamp = packed >> config.layout().sequence();
        let sequence = packed & config.layout().sequence_max();
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

/// Lazy state pool with DashMap for on-demand allocation
#[derive(Debug)]
pub struct StatePool {
    states: DashMap<u32, Arc<State>>,
    config: Config,
}

impl StatePool {
    /// Create with empty DashMap - states allocated on-demand
    pub fn new(config: Config) -> Self {
        Self {
            states: DashMap::new(),
            config,
        }
    }

    /// Pack (worker_id, process_id) into u32 key using bit layout
    #[inline(always)]
    fn pack_key(&self, worker_id: u64, process_id: u64) -> u32 {
        // Use max values from config for bounds safety
        let masked_worker = worker_id & self.config.layout().worker_max();
        let masked_process = process_id & self.config.layout().process_max();

        // Mathematical mapping: key = worker_id * 2^process_bits + process_id
        // This uses the same shift logic as ID composition but for key packing
        let key = (masked_worker << self.config.layout().process()) | masked_process;
        key as u32
    }

    /// Get state for (worker_id, process_id) - lazy initialization on first access
    #[inline]
    pub fn get_state(&self, worker_id: u64, process_id: u64) -> Arc<State> {
        let key = self.pack_key(worker_id, process_id);

        self.states
            .entry(key)
            .or_insert_with(|| Arc::new(State::default()))
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;
    use std::sync::Arc;

    #[test]
    fn pool_starts_empty() {
        let config = Config::default();
        let state_pool = StatePool::new(config);

        // Should start with no states (lazy initialization)
        assert_eq!(state_pool.states.len(), 0);
    }

    #[test]
    fn pool_lazy_initialization() {
        let config = Config::default();
        let state_pool = StatePool::new(config);

        // Initially empty
        assert_eq!(state_pool.states.len(), 0);

        // Access state (0, 0)
        let _state1 = state_pool.get_state(0, 0);
        assert_eq!(state_pool.states.len(), 1);

        // Access different state (1, 0)
        let _state2 = state_pool.get_state(1, 0);
        assert_eq!(state_pool.states.len(), 2);

        // Access another different state (0, 1)
        let _state3 = state_pool.get_state(0, 1);
        assert_eq!(state_pool.states.len(), 3);

        // Re-accessing same state doesn't create new entry
        let _state1_again = state_pool.get_state(0, 0);
        assert_eq!(state_pool.states.len(), 3);
    }

    #[test]
    fn pool_state_identity() {
        let config = Config::default();
        let state_pool = StatePool::new(config);

        // Test state identity
        let state1 = state_pool.get_state(0, 0);
        let state2 = state_pool.get_state(1, 0);
        let state3 = state_pool.get_state(0, 1);

        // Different worker/process should give different states
        assert!(!Arc::ptr_eq(&state1, &state2));
        assert!(!Arc::ptr_eq(&state1, &state3));
        assert!(!Arc::ptr_eq(&state2, &state3));

        // Same worker/process should give same state
        let state1_again = state_pool.get_state(0, 0);
        assert!(Arc::ptr_eq(&state1, &state1_again));
    }

    #[test]
    fn pool_key_packing() {
        let config = Config::default();
        let state_pool = StatePool::new(config);

        // Test that key packing is deterministic
        let key1 = state_pool.pack_key(5, 3);
        let key2 = state_pool.pack_key(5, 3);
        assert_eq!(key1, key2);

        // Different (worker, process) should give different keys
        let key3 = state_pool.pack_key(5, 4);
        let key4 = state_pool.pack_key(6, 3);
        assert_ne!(key1, key3);
        assert_ne!(key1, key4);
        assert_ne!(key3, key4);
    }
}
