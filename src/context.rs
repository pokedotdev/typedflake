//! Per-type ID context and generator factory.
//!
//! Each ID type created with the [`id!`](crate::id) macro maintains its own static
//! [`IdContext`] that holds:
//! - Configuration (bit layout and epoch)
//! - Shared state pool for (worker_id, process_id) instances
//! - Default generator (lazily initialized)
//!
//! The context provides factory methods to create generators with specific worker/process IDs.

use crate::config::{Config, ValidationError};
use crate::generator::Generator;
use crate::global;
use crate::state::StatePool;
use std::sync::OnceLock;

/// Context for ID type - holds config, state pool, and default generator
pub struct IdContext {
    config: Config,
    states: StatePool,
    default_generator: OnceLock<Generator>,
}

impl IdContext {
    /// Create new context for given config
    pub fn new(config: Config) -> Self {
        let states = StatePool::new(config);
        Self {
            config,
            states,
            default_generator: OnceLock::new(),
        }
    }

    /// Get the configuration
    pub const fn config(&self) -> Config {
        self.config
    }

    /// Get default generator (lazy initialization)
    pub fn default_generator(&self) -> &Generator {
        self.default_generator.get_or_init(|| {
            let (worker_id, process_id) = global::get_default_instance();
            self.create_generator(worker_id, process_id)
                .expect("Default instance should always be valid")
        })
    }

    /// Create generator with specific worker_id and process_id
    pub fn create_generator(
        &self,
        worker_id: u64,
        process_id: u64,
    ) -> Result<Generator, ValidationError> {
        // Get state (lazy initialization on first access)
        let state = self.states.get_state(worker_id, process_id);

        Generator::new_with_state(self.config(), state, worker_id, process_id)
    }

    /// Create generator with specific worker_id and default process_id
    pub fn create_worker(&self, worker_id: u64) -> Result<Generator, ValidationError> {
        let (_, process_id) = global::get_default_instance();
        self.create_generator(worker_id, process_id)
    }

    /// Create generator with default worker_id and specific process_id
    pub fn create_process(&self, process_id: u64) -> Result<Generator, ValidationError> {
        let (worker_id, _) = global::get_default_instance();
        self.create_generator(worker_id, process_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;

    #[test]
    fn context_create_generators() {
        let config = Config::default();
        let context = IdContext::new(config);

        // Create different generators
        let gen1 = context.create_generator(0, 0).unwrap();
        let gen2 = context.create_generator(1, 0).unwrap();
        let gen3 = context.create_generator(0, 1).unwrap();

        // Verify they have different worker/process IDs
        assert_eq!(gen1.worker_id(), 0);
        assert_eq!(gen1.process_id(), 0);
        assert_eq!(gen2.worker_id(), 1);
        assert_eq!(gen2.process_id(), 0);
        assert_eq!(gen3.worker_id(), 0);
        assert_eq!(gen3.process_id(), 1);
    }

    #[test]
    fn context_id_generation() {
        let config = Config::default();
        let context = IdContext::new(config);
        let generator = context.create_generator(5, 3).unwrap();

        // Generate IDs
        let id1 = generator.generate();
        let id2 = generator.generate();

        // IDs should be different
        assert_ne!(id1, id2);
        assert!(id1 > 0);
        assert!(id2 > 0);
    }

    #[test]
    fn default_generator_singleton() {
        let config = Config::default();
        let context = IdContext::new(config);

        // Test default generator
        let default_gen = context.default_generator();
        let id = default_gen.generate();
        assert!(id > 0);

        // Should be same instance on subsequent calls
        let default_gen2 = context.default_generator();
        assert!(std::ptr::eq(default_gen, default_gen2));
    }

    #[test]
    fn worker_and_process_convenience_methods() {
        let config = Config::default();
        let context = IdContext::new(config);

        let worker_gen = context.create_worker(10).unwrap();
        let process_gen = context.create_process(5).unwrap();

        assert_eq!(worker_gen.worker_id(), 10);
        assert_eq!(worker_gen.process_id(), 0); // default
        assert_eq!(process_gen.worker_id(), 0); // default
        assert_eq!(process_gen.process_id(), 5);
    }

    #[test]
    fn generators_share_state_but_not_instances() {
        let config = Config::default();
        let context = IdContext::new(config);

        // Create two generators with same (worker_id, process_id)
        let gen1 = context.create_generator(5, 3).unwrap();
        let gen2 = context.create_generator(5, 3).unwrap();

        // Generators are different instances
        assert!(!std::ptr::eq(&gen1, &gen2));

        // But they share the same underlying State (verified by sequential sequence numbers)
        let id1 = gen1.generate();
        let id2 = gen2.generate();

        // Extract sequences from both IDs
        let seq1 = (id1 >> config.layout().sequence_shift()) & config.layout().sequence_max();
        let seq2 = (id2 >> config.layout().sequence_shift()) & config.layout().sequence_max();

        // Sequences should be sequential (0, 1) proving shared state
        assert_eq!(seq1, 0);
        assert_eq!(seq2, 1);
    }

    #[test]
    fn default_generator_shares_state_with_manual_instance() {
        let config = Config::default();
        let context = IdContext::new(config);

        // Generate using default generator
        let default_gen = context.default_generator();
        let id1 = default_gen.generate();

        // Create manual generator for same instance (0, 0) to test state sharing
        let manual_gen = context.create_generator(0, 0).unwrap();
        let id2 = manual_gen.generate();

        // Extract sequences
        let seq1 = (id1 >> config.layout().sequence_shift()) & config.layout().sequence_max();
        let seq2 = (id2 >> config.layout().sequence_shift()) & config.layout().sequence_max();

        // Sequences should be sequential, proving they share the same State
        assert_eq!(seq1, 0);
        assert_eq!(seq2, 1);
    }
}
