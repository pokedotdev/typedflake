use crate::config::Config;
use crate::factory::GeneratorFactory;
use crate::generator::Generator;
use crate::global;
use std::sync::OnceLock;

/// Unified manager for ID type state - combines factory and default generator
pub struct IdManager {
    factory: GeneratorFactory,
    default_generator: OnceLock<Generator>,
}

impl IdManager {
    /// Create new manager for given config
    pub fn new(config: Config) -> Self {
        Self {
            factory: GeneratorFactory::new(config),
            default_generator: OnceLock::new(),
        }
    }

    /// Get factory for creating custom generators
    pub fn factory(&self) -> &GeneratorFactory {
        &self.factory
    }

    /// Get default generator (lazy initialization)
    pub fn default_generator(&self) -> &Generator {
        self.default_generator.get_or_init(|| {
            let (worker_id, process_id) = global::get_default_instance();
            self.factory.create_generator(worker_id, process_id)
        })
    }

    /// Create generator with specific worker_id and process_id
    pub fn create_generator(&self, worker_id: u64, process_id: u64) -> Generator {
        self.factory.create_generator(worker_id, process_id)
    }

    /// Create generator with specific worker_id and default process_id
    pub fn create_worker(&self, worker_id: u64) -> Generator {
        let (_, process_id) = global::get_default_instance();
        self.factory.create_generator(worker_id, process_id)
    }

    /// Create generator with default worker_id and specific process_id
    pub fn create_process(&self, process_id: u64) -> Generator {
        let (worker_id, _) = global::get_default_instance();
        self.factory.create_generator(worker_id, process_id)
    }
}
