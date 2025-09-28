use crate::config::Config;
use crate::generator::Generator;
use crate::global;
use crate::state::StateVec;
use std::sync::OnceLock;

/// Unified manager for ID type state - combines factory and default generator
pub struct IdManager {
    config: Config,
    states: StateVec,
    default_generator: OnceLock<Generator>,
}

impl IdManager {
    /// Create new manager for given config
    pub fn new(config: Config) -> Self {
        let states = StateVec::new(config);
        Self {
            config,
            states,
            default_generator: OnceLock::new(),
        }
    }

    /// Get default generator (lazy initialization)
    pub fn default_generator(&self) -> &Generator {
        self.default_generator.get_or_init(|| {
            let (worker_id, process_id) = global::get_default_instance();
            self.create_generator(worker_id, process_id)
        })
    }

    /// Create generator with specific worker_id and process_id
    pub fn create_generator(&self, worker_id: u64, process_id: u64) -> Generator {
        // Get pre-allocated state (no lookup overhead)
        let state = self.states.get_state(worker_id, process_id).clone();

        Generator::new_with_state(self.config, state, worker_id, process_id)
    }

    /// Create generator with specific worker_id and default process_id
    pub fn create_worker(&self, worker_id: u64) -> Generator {
        let (_, process_id) = global::get_default_instance();
        self.create_generator(worker_id, process_id)
    }

    /// Create generator with default worker_id and specific process_id
    pub fn create_process(&self, process_id: u64) -> Generator {
        let (worker_id, _) = global::get_default_instance();
        self.create_generator(worker_id, process_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;

    #[test]
    fn test_id_manager() {
        let config = Config::new((41, 10, 5, 8), Config::DEFAULT_EPOCH_MS);
        let manager = IdManager::new(config);

        // Create different generators
        let gen1 = manager.create_generator(0, 0);
        let gen2 = manager.create_generator(1, 0);
        let gen3 = manager.create_generator(0, 1);

        // Verify they have different worker/process IDs
        assert_eq!(gen1.worker_id(), 0);
        assert_eq!(gen1.process_id(), 0);
        assert_eq!(gen2.worker_id(), 1);
        assert_eq!(gen2.process_id(), 0);
        assert_eq!(gen3.worker_id(), 0);
        assert_eq!(gen3.process_id(), 1);
    }

    #[test]
    fn test_manager_generation() {
        let config = Config::new((41, 10, 5, 8), Config::DEFAULT_EPOCH_MS);
        let manager = IdManager::new(config);
        let generator = manager.create_generator(42, 7);

        // Generate IDs
        let id1 = generator.generate().unwrap();
        let id2 = generator.generate().unwrap();

        // IDs should be different
        assert_ne!(id1, id2);
        assert!(id1 > 0);
        assert!(id2 > 0);
    }

    #[test]
    fn test_default_generator() {
        let config = Config::new((41, 10, 5, 8), Config::DEFAULT_EPOCH_MS);
        let manager = IdManager::new(config);

        // Test default generator
        let default_gen = manager.default_generator();
        let id = default_gen.generate().unwrap();
        assert!(id > 0);

        // Should be same instance on subsequent calls
        let default_gen2 = manager.default_generator();
        assert!(std::ptr::eq(default_gen, default_gen2));
    }

    #[test]
    fn test_worker_and_process_methods() {
        let config = Config::new((41, 10, 5, 8), Config::DEFAULT_EPOCH_MS);
        let manager = IdManager::new(config);

        let worker_gen = manager.create_worker(42);
        let process_gen = manager.create_process(7);

        assert_eq!(worker_gen.worker_id(), 42);
        assert_eq!(worker_gen.process_id(), 0); // default
        assert_eq!(process_gen.worker_id(), 0); // default
        assert_eq!(process_gen.process_id(), 7);
    }
}
