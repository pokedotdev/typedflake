use crate::config::Config;
use crate::generator::Generator;
use crate::state::StateVec;

/// Factory for creating generators with pre-injected state
#[derive(Debug)]
pub struct GeneratorFactory {
    config: Config,
    states: StateVec,
}

impl GeneratorFactory {
    /// Create new factory with pre-allocated states
    pub fn new(config: Config) -> Self {
        let states = StateVec::new(config);
        Self { config, states }
    }

    /// Create a Generator with injected state for specific (worker_id, process_id)
    pub fn create_generator(&self, worker_id: u64, process_id: u64) -> Generator {
        // Get pre-allocated state (no lookup overhead)
        let state = self.states.get_state(worker_id, process_id).clone();

        Generator::new_with_state(self.config, state, worker_id, process_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;

    #[test]
    fn test_generator_factory() {
        let config = Config::new((41, 10, 5, 8), Config::DEFAULT_EPOCH_MS);
        let factory = GeneratorFactory::new(config);

        // Create different generators
        let gen1 = factory.create_generator(0, 0);
        let gen2 = factory.create_generator(1, 0);
        let gen3 = factory.create_generator(0, 1);

        // Verify they have different worker/process IDs
        assert_eq!(gen1.worker_id(), 0);
        assert_eq!(gen1.process_id(), 0);
        assert_eq!(gen2.worker_id(), 1);
        assert_eq!(gen2.process_id(), 0);
        assert_eq!(gen3.worker_id(), 0);
        assert_eq!(gen3.process_id(), 1);
    }

    #[test]
    fn test_generator_generation() {
        let config = Config::new((41, 10, 5, 8), Config::DEFAULT_EPOCH_MS);
        let factory = GeneratorFactory::new(config);
        let generator = factory.create_generator(42, 7);

        // Generate IDs
        let id1 = generator.generate().unwrap();
        let id2 = generator.generate().unwrap();

        // IDs should be different
        assert_ne!(id1, id2);
        assert!(id1 > 0);
        assert!(id2 > 0);
    }

    // #[test]
    // fn test_factory_validation() {
    //     let config = Config::new((41, 10, 5, 8), Config::DEFAULT_EPOCH_MS);
    //     let factory = GeneratorFactory::new(config);

    //     // Valid instances
    //     assert!(factory.create_generator(1023, 31).is_ok()); // max values
    //     assert!(factory.create_generator(0, 0).is_ok());

    //     // Invalid instances
    //     assert!(factory.create_generator(1024, 0).is_err()); // worker_id too high
    //     assert!(factory.create_generator(0, 32).is_err()); // process_id too high
    // }
}
