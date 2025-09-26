#[macro_export]
macro_rules! id {
    ($($tokens:tt)*) => {
        $crate::typedflake_id!($($tokens)*);
    };
}

#[macro_export]
macro_rules! typedflake_id {
    ($name:ident) => {
        $crate::typedflake_id!($name, $crate::global::get_default_config());
    };

    ($name:ident, $algorithm:expr) => {
        paste::paste! {
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
            pub struct $name(u64);

            /// Factory-generated instance with pre-injected state for optimal performance
            pub struct [<$name Generator>] {
                inner: $crate::generator::Generator,
            }

            impl [<$name Generator>] {
                /// Generate an ID using pre-injected state (no lookup overhead)
                pub fn generate(&self) -> Result<$name, $crate::generator::GeneratorError> {
                    let id = self.inner.generate()?;
                    Ok($name(id))
                }

                /// Generate an ID using pre-injected state, blocking on sequence exhaustion
                pub fn generate_blocking(&self) -> $name {
                    let id = self.inner.generate_blocking();
                    $name(id)
                }

                /// Get the worker_id this generator is bound to
                pub fn worker_id(&self) -> u64 {
                    self.inner.worker_id()
                }

                /// Get the process_id this generator is bound to
                pub fn process_id(&self) -> u64 {
                    self.inner.process_id()
                }
            }

            impl $name {
	            /// Returns the singleton GeneratorFactory for this ID type (lazy static initialization)
	            fn factory() -> &'static $crate::factory::GeneratorFactory {
	                static FACTORY: std::sync::OnceLock<$crate::factory::GeneratorFactory> =
	                    std::sync::OnceLock::new();
	                FACTORY.get_or_init(|| $crate::factory::GeneratorFactory::new($algorithm))
	            }

	            /// Returns the singleton default Generator for this ID type (lazy static initialization)
	            fn generator() -> &'static $crate::generator::Generator {
	                static GENERATOR: std::sync::OnceLock<$crate::generator::Generator> =
	                    std::sync::OnceLock::new();
	                GENERATOR.get_or_init(|| {
	                    let (worker_id, process_id) = $crate::global::get_default_instance();
	                    Self::factory().create_generator(worker_id, process_id)
	                })
	            }

	            /// Generate a new ID with default instance (0, 0)
	            pub fn generate() -> Result<Self, $crate::generator::GeneratorError> {
	                let id = Self::generator().generate()?;
	                Ok(Self(id))
	            }

	            /// Generate a new ID with default instance (0, 0), blocking until next millisecond if sequence is exhausted
	            pub fn generate_blocking() -> Self {
	                let id = Self::generator().generate_blocking();
	                Self(id)
	            }

	            /// Create an instance generator wrapper with specific worker_id and process_id
	            pub fn instance(worker_id: u64, process_id: u64) -> [<$name Generator>] {
	                let inner = Self::factory().create_generator(worker_id, process_id);
	                    // .expect("Invalid worker_id or process_id");
	                [<$name Generator>] { inner }
	            }

	            /// Create an instance generator wrapper with specific worker_id and process_id = 0
	            pub fn worker(worker_id: u64) -> [<$name Generator>] {
              		let (_, process_id) = $crate::global::get_default_instance();
	                Self::instance(worker_id, process_id)
	            }

	            /// Create an instance generator wrapper with worker_id = 0 and specific process_id
	            pub fn process(process_id: u64) -> [<$name Generator>] {
		            let (worker_id, _) = $crate::global::get_default_instance();
	                Self::instance(worker_id, process_id)
	            }
	        }

	        impl $name {
	            /// Create an ID from a raw u64 value
	            pub fn from_u64(id: u64) -> Self {
	                Self(id)
	            }

	            /// Get the raw u64 value of this ID
	            pub fn as_u64(self) -> u64 {
	                self.0
	            }

	            /// Decompose the ID into its components as a tuple
	            pub fn decompose(self) -> (u64, u64, u64, u64) {
	                Self::generator().decompose(self.0)
	            }

	            /// Get the ID components as a struct
	            pub fn components(self) -> $crate::generator::IdComponents {
	                Self::generator().components(self.0)
	            }

	            /// Get just the timestamp component
	            pub fn timestamp(self) -> u64 {
	                Self::generator().extract_timestamp(self.0)
	            }

	            /// Get just the worker ID component
	            pub fn worker_id(self) -> u64 {
	                Self::generator().extract_worker_id(self.0)
	            }

	            /// Get just the process ID component
	            pub fn process_id(self) -> u64 {
	                Self::generator().extract_process_id(self.0)
	            }

	            /// Get just the sequence component
	            pub fn sequence(self) -> u64 {
	                Self::generator().extract_sequence(self.0)
	            }

	            /// Compose an ID from individual components
	            pub fn compose(timestamp: u64, worker_id: u64, process_id: u64, sequence: u64) -> Self {
	                let id = Self::generator().compose_custom(timestamp, worker_id, process_id, sequence);
	                Self(id)
	            }
	        }

	        impl std::fmt::Display for $name {
	            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
	                write!(f, "{}", self.0)
	            }
	        }

	        impl From<u64> for $name {
	            fn from(id: u64) -> Self {
	                Self(id)
	            }
	        }

	        impl From<$name> for u64 {
	            fn from(id: $name) -> u64 {
	                id.0
	            }
	        }

	        impl std::str::FromStr for $name {
	            type Err = std::num::ParseIntError;

	            fn from_str(s: &str) -> Result<Self, Self::Err> {
	                let id = s.parse::<u64>()?;
	                Ok(Self(id))
	            }
	        }
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::Config;

    #[test]
    fn test_id_macro_default() {
        crate::id!(TestId);

        let id1 = TestId::generate().unwrap();
        let id2 = TestId::generate().unwrap();

        assert_ne!(id1, id2);
        assert!(id1.as_u64() > 0);
        assert!(id2.as_u64() > 0);

        let components = id1.components();
        assert_eq!(components.worker_id, 0); // Default config
        assert_eq!(components.process_id, 0); // Default config
    }

    #[test]
    fn test_id_macro_algorithm_config() {
        const CUSTOM_ALGORITHM: Config = Config::new(
            (42, 10, 5, 7),    // bits: timestamp, worker, process, sequence
            1_600_000_000_000, // epoch
        );

        crate::id!(AlgorithmId, CUSTOM_ALGORITHM);

        // Test default instance (0, 0)
        let id = AlgorithmId::generate().unwrap();
        let components = id.components();

        assert_eq!(components.worker_id, 0);
        assert_eq!(components.process_id, 0);

        // Test specific instance
        let instance = AlgorithmId::instance(99, 3);
        let instance_id = instance.generate().unwrap();
        let instance_components = instance_id.components();

        assert_eq!(instance_components.worker_id, 99);
        assert_eq!(instance_components.process_id, 3);
    }

    #[test]
    fn test_id_compose_decompose() {
        crate::id!(ComposeId);

        let timestamp = 123456789;
        let worker_id = 0; // Default config
        let process_id = 0; // Default config
        let sequence = 100;

        let id = ComposeId::compose(timestamp, worker_id, process_id, sequence);
        let (dec_timestamp, dec_worker_id, dec_process_id, dec_sequence) = id.decompose();

        assert_eq!(dec_timestamp, timestamp);
        assert_eq!(dec_worker_id, worker_id);
        assert_eq!(dec_process_id, process_id);
        assert_eq!(dec_sequence, sequence);
    }

    #[test]
    fn test_id_individual_components() {
        crate::id!(ComponentId);

        let id = ComponentId::generate().unwrap();

        let timestamp = id.timestamp();
        let worker_id = id.worker_id();
        let process_id = id.process_id();
        let sequence = id.sequence();

        let components = id.components();

        assert_eq!(timestamp, components.timestamp);
        assert_eq!(worker_id, components.worker_id);
        assert_eq!(process_id, components.process_id);
        assert_eq!(sequence, components.sequence);
    }

    #[test]
    fn test_id_conversions() {
        crate::id!(ConversionId);

        let id = ConversionId::generate().unwrap();
        let raw = id.as_u64();

        let id2 = ConversionId::from_u64(raw);
        assert_eq!(id, id2);

        let id3: ConversionId = raw.into();
        assert_eq!(id, id3);

        let raw2: u64 = id.into();
        assert_eq!(raw, raw2);
    }

    #[test]
    fn test_id_display_and_parsing() {
        crate::id!(ParseId);

        let id = ParseId::generate().unwrap();
        let id_str = id.to_string();

        let parsed_id: ParseId = id_str.parse().unwrap();
        assert_eq!(id, parsed_id);
    }

    #[test]
    fn test_id_blocking_generate() {
        crate::id!(BlockingId);

        let id1 = BlockingId::generate_blocking();
        let id2 = BlockingId::generate_blocking();

        assert_ne!(id1, id2);
    }

    #[test]
    fn test_multiple_id_types() {
        crate::id!(UserId);
        crate::id!(OrderId);

        let user_id1 = UserId::generate().unwrap();
        let user_id2 = UserId::generate().unwrap();
        let order_id1 = OrderId::generate().unwrap();
        let order_id2 = OrderId::generate().unwrap();

        // Verify that each type generates different IDs
        assert_ne!(user_id1, user_id2);
        assert_ne!(order_id1, order_id2);

        // Verify that IDs are valid u64s
        assert!(user_id1.as_u64() > 0);
        assert!(order_id1.as_u64() > 0);

        // Different types can't be compared directly - they're different types!
        // But we can verify their raw u64 values are different due to different timing
        // or at least that the system is working correctly
        assert!(user_id1.as_u64() != user_id2.as_u64());
        assert!(order_id1.as_u64() != order_id2.as_u64());
    }

    #[test]
    fn test_id_macro_instance_methods() {
        const CUSTOM_ALGORITHM: Config = Config::new((42, 10, 5, 7), 1_600_000_000_000);

        crate::id!(InstanceMethodId, CUSTOM_ALGORITHM);

        // Test worker method
        let worker_instance = InstanceMethodId::worker(99);
        let worker_id = worker_instance.generate().unwrap();
        let worker_components = worker_id.components();
        assert_eq!(worker_components.worker_id, 99);
        assert_eq!(worker_components.process_id, 0);

        // Test process method
        let process_instance = InstanceMethodId::process(3);
        let process_id = process_instance.generate().unwrap();
        let process_components = process_id.components();
        assert_eq!(process_components.worker_id, 0);
        assert_eq!(process_components.process_id, 3);

        // Test instance method
        let full_instance = InstanceMethodId::instance(99, 3);
        let full_id = full_instance.generate().unwrap();
        let full_components = full_id.components();
        assert_eq!(full_components.worker_id, 99);
        assert_eq!(full_components.process_id, 3);
    }

    #[test]
    fn test_id_macro_multiple_instances() {
        const SHARED_ALGORITHM: Config = Config::new((41, 10, 5, 8), Config::DEFAULT_EPOCH_MS);

        crate::id!(MultiInstanceId, SHARED_ALGORITHM);

        // Test that different instances work independently
        let instance1 = MultiInstanceId::instance(42, 7);
        let instance2 = MultiInstanceId::instance(1, 2);

        // Generate IDs from both instances
        let id1 = instance1.generate().unwrap();
        let id2 = instance2.generate().unwrap();

        let components1 = id1.components();
        let components2 = id2.components();

        assert_eq!(components1.worker_id, 42);
        assert_eq!(components1.process_id, 7);
        assert_eq!(components2.worker_id, 1);
        assert_eq!(components2.process_id, 2);
    }

    #[test]
    fn test_factory_pattern_macro_integration() {
        const FACTORY_ALGORITHM: Config = Config::new((41, 10, 5, 8), Config::DEFAULT_EPOCH_MS);

        crate::id!(FactoryTestId, FACTORY_ALGORITHM);

        // Test Generator creation and usage
        let stateful_gen = FactoryTestId::instance(99, 3);

        // Verify bound worker and process IDs
        assert_eq!(stateful_gen.worker_id(), 99);
        assert_eq!(stateful_gen.process_id(), 3);

        // Generate IDs with pre-injected state (no lookup overhead)
        let id1 = stateful_gen.generate().unwrap();
        let id2 = stateful_gen.generate().unwrap();

        // Verify IDs have correct components
        let components1 = id1.components();
        let components2 = id2.components();

        assert_eq!(components1.worker_id, 99);
        assert_eq!(components1.process_id, 3);
        assert_eq!(components2.worker_id, 99);
        assert_eq!(components2.process_id, 3);

        // IDs should be different
        assert_ne!(id1, id2);
    }
}
