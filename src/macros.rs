#[macro_export]
macro_rules! id {
    ($name:ident) => {
        $crate::id!($name, $crate::global::get_default_config());
    };

    ($name:ident, $config:expr) => {
        paste::paste! {
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
            pub struct $name(u64);

            /// Typed generator wrapper that produces the correct ID type
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
                /// Returns the singleton IdContext for this ID type (lazy static initialization)
                fn context() -> &'static $crate::context::IdContext {
                    static CONTEXT: std::sync::OnceLock<$crate::context::IdContext> =
                        std::sync::OnceLock::new();
                    CONTEXT.get_or_init(|| $crate::context::IdContext::new($config))
                }

                /// Generate a new ID with default instance (0, 0)
                pub fn generate() -> Result<Self, $crate::generator::GeneratorError> {
                    let id = Self::context().default_generator().generate()?;
                    Ok(Self(id))
                }

                /// Generate a new ID with default instance (0, 0), blocking until next millisecond if sequence is exhausted
                pub fn generate_blocking() -> Self {
                    let id = Self::context().default_generator().generate_blocking();
                    Self(id)
                }

                /// Create a typed generator wrapper with specific worker_id and process_id
                pub fn instance(worker_id: u64, process_id: u64) -> Result<[<$name Generator>], $crate::config::ValidationError> {
                    let inner = Self::context().create_generator(worker_id, process_id)?;
                    Ok([<$name Generator>] { inner })
                }

                /// Create a typed generator wrapper with specific worker_id and default process_id
                pub fn worker(worker_id: u64) -> Result<[<$name Generator>], $crate::config::ValidationError> {
                    let inner = Self::context().create_worker(worker_id)?;
                    Ok([<$name Generator>] { inner })
                }

                /// Create a typed generator wrapper with default worker_id and specific process_id
                pub fn process(process_id: u64) -> Result<[<$name Generator>], $crate::config::ValidationError> {
                    let inner = Self::context().create_process(process_id)?;
                    Ok([<$name Generator>] { inner })
                }

                /// Decompose the ID into its components as a tuple
                pub fn decompose(self) -> (u64, u64, u64, u64) {
                    Self::context().default_generator().decompose(self.0)
                }

                /// Get the ID components as a struct
                pub fn components(self) -> $crate::generator::IdComponents {
                    Self::context().default_generator().components(self.0)
                }

                /// Get just the timestamp component
                pub fn timestamp(self) -> u64 {
                    Self::context().default_generator().extract_timestamp(self.0)
                }

                /// Get just the worker ID component
                pub fn worker_id(self) -> u64 {
                    Self::context().default_generator().extract_worker_id(self.0)
                }

                /// Get just the process ID component
                pub fn process_id(self) -> u64 {
                    Self::context().default_generator().extract_process_id(self.0)
                }

                /// Get just the sequence component
                pub fn sequence(self) -> u64 {
                    Self::context().default_generator().extract_sequence(self.0)
                }

                /// Compose an ID from timestamp and sequence with validation.
                /// Uses generator's worker_id and process_id.
                pub fn compose(timestamp: u64, sequence: u64) -> Result<Self, $crate::config::ValidationError> {
                    let id = Self::context().default_generator().compose(timestamp, sequence)?;
                    Ok(Self(id))
                }

                /// Compose an ID from timestamp and sequence without validation.
                /// Uses generator's worker_id and process_id.
                /// Values exceeding bit limits will be silently masked.
                pub fn compose_unchecked(timestamp: u64, sequence: u64) -> Self {
                    let id = Self::context().default_generator().compose_unchecked(timestamp, sequence);
                    Self(id)
                }

                /// Compose an ID from all components with validation
                pub fn compose_custom(
                    timestamp: u64,
                    worker_id: u64,
                    process_id: u64,
                    sequence: u64,
                ) -> Result<Self, $crate::config::ValidationError> {
                    let id = Self::context().default_generator().compose_custom(timestamp, worker_id, process_id, sequence)?;
                    Ok(Self(id))
                }

                /// Compose an ID from all components without validation.
                /// Components exceeding bit limits will be silently masked.
                pub fn compose_custom_unchecked(timestamp: u64, worker_id: u64, process_id: u64, sequence: u64) -> Self {
                    let id = Self::context().default_generator().compose_custom_unchecked(timestamp, worker_id, process_id, sequence);
                    Self(id)
                }
            }

            // Generate all standard trait implementations (from_u64_unchecked, try_from_u64, as_u64, Display, TryFrom, FromStr)
            $crate::impl_id_traits!($name);
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::{BitLayout, Config};

    #[test]
    fn macro_generates_default_config() {
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
    fn macro_with_custom_config() {
        const CUSTOM_CONFIG: Config = Config::new(
            BitLayout::new(42, 8, 4, 10), // bits: timestamp, worker, process, sequence
            1_600_000_000_000,            // epoch
        );

        crate::id!(CustomId, CUSTOM_CONFIG);

        // Test default instance (0, 0)
        let id = CustomId::generate().unwrap();
        let components = id.components();

        assert_eq!(components.worker_id, 0);
        assert_eq!(components.process_id, 0);

        // Test specific instance
        let instance = CustomId::instance(50, 5).unwrap();
        let instance_id = instance.generate().unwrap();
        let instance_components = instance_id.components();

        assert_eq!(instance_components.worker_id, 50);
        assert_eq!(instance_components.process_id, 5);
    }

    #[test]
    fn compose_decompose_roundtrip() {
        crate::id!(ComposeId);

        let timestamp = 123456789;
        let sequence = 100;

        // Test compose (uses default worker=0, process=0)
        let id = ComposeId::compose(timestamp, sequence).unwrap();
        let (dec_timestamp, dec_worker_id, dec_process_id, dec_sequence) = id.decompose();

        assert_eq!(dec_timestamp, timestamp);
        assert_eq!(dec_worker_id, 0);
        assert_eq!(dec_process_id, 0);
        assert_eq!(dec_sequence, sequence);

        // Test compose_custom
        let id_custom = ComposeId::compose_custom(timestamp, 5, 3, sequence).unwrap();
        let (ts, w, p, s) = id_custom.decompose();
        assert_eq!(ts, timestamp);
        assert_eq!(w, 5);
        assert_eq!(p, 3);
        assert_eq!(s, sequence);
    }

    #[test]
    fn extract_individual_components() {
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
    fn u64_conversions() {
        crate::id!(ConversionId);

        let id = ConversionId::generate().unwrap();
        let raw = id.as_u64();

        let id2 = ConversionId::from_u64_unchecked(raw);
        assert_eq!(id, id2);

        let id3: ConversionId = raw.try_into().unwrap();
        assert_eq!(id, id3);

        let raw2: u64 = id.into();
        assert_eq!(raw, raw2);
    }

    #[test]
    fn string_display_and_parsing() {
        crate::id!(ParseId);

        let id = ParseId::generate().unwrap();
        let id_str = id.to_string();

        let parsed_id: ParseId = id_str.parse().unwrap();
        assert_eq!(id, parsed_id);
    }

    #[test]
    fn multiple_types_independent() {
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
    fn instance_convenience_methods() {
        const CUSTOM_ALGORITHM: Config =
            Config::new(BitLayout::new(42, 8, 4, 10), 1_600_000_000_000);

        crate::id!(InstanceMethodId, CUSTOM_ALGORITHM);

        // Test worker method
        let worker_instance = InstanceMethodId::worker(50).unwrap();
        let worker_id = worker_instance.generate().unwrap();
        let worker_components = worker_id.components();
        assert_eq!(worker_components.worker_id, 50);
        assert_eq!(worker_components.process_id, 0);

        // Test process method
        let process_instance = InstanceMethodId::process(5).unwrap();
        let process_id = process_instance.generate().unwrap();
        let process_components = process_id.components();
        assert_eq!(process_components.worker_id, 0);
        assert_eq!(process_components.process_id, 5);

        // Test instance method
        let full_instance = InstanceMethodId::instance(50, 5).unwrap();
        let full_id = full_instance.generate().unwrap();
        let full_components = full_id.components();
        assert_eq!(full_components.worker_id, 50);
        assert_eq!(full_components.process_id, 5);
    }

    #[test]
    fn multiple_instances_independent_state() {
        crate::id!(MultiInstanceId);

        // Test that different instances work independently
        let instance1 = MultiInstanceId::instance(10, 5).unwrap();
        let instance2 = MultiInstanceId::instance(1, 2).unwrap();

        // Generate IDs from both instances
        let id1 = instance1.generate().unwrap();
        let id2 = instance2.generate().unwrap();

        let components1 = id1.components();
        let components2 = id2.components();

        assert_eq!(components1.worker_id, 10);
        assert_eq!(components1.process_id, 5);
        assert_eq!(components2.worker_id, 1);
        assert_eq!(components2.process_id, 2);
    }

    #[test]
    fn generator_with_bound_ids() {
        crate::id!(FactoryTestId);

        // Test Generator creation and usage
        let stateful_gen = FactoryTestId::instance(15, 7).unwrap();

        // Verify bound worker and process IDs
        assert_eq!(stateful_gen.worker_id(), 15);
        assert_eq!(stateful_gen.process_id(), 7);

        // Generate IDs with pre-injected state (no lookup overhead)
        let id1 = stateful_gen.generate().unwrap();
        let id2 = stateful_gen.generate().unwrap();

        // Verify IDs have correct components
        let components1 = id1.components();
        let components2 = id2.components();

        assert_eq!(components1.worker_id, 15);
        assert_eq!(components1.process_id, 7);
        assert_eq!(components2.worker_id, 15);
        assert_eq!(components2.process_id, 7);

        // IDs should be different
        assert_ne!(id1, id2);
    }

    #[test]
    fn try_from_u64_validates_components() {
        const CUSTOM_CONFIG: Config = Config::new(
            BitLayout::new(42, 8, 4, 10), // max: ts=4.4T, worker=255, process=15, seq=1023
            1_600_000_000_000,
        );
        crate::id!(ValidationTestId, CUSTOM_CONFIG);

        // Generate a valid ID
        let valid_id = ValidationTestId::generate().unwrap();
        let raw = valid_id.as_u64();

        // Valid ID should pass validation
        assert!(ValidationTestId::try_from_u64(raw).is_ok());

        // Note: Manually crafted IDs with simple bit shifts get masked during composition,
        // so they appear valid after extraction. Validation catches component overflow
        // during compose() rather than from arbitrary u64 values.
        // Generated IDs are always valid by construction.
    }

    #[test]
    fn try_from_trait_validates() {
        crate::id!(TryFromTestId);

        let valid_id = TryFromTestId::generate().unwrap();
        let raw = valid_id.as_u64();

        // TryFrom should work for valid IDs
        let converted: Result<TryFromTestId, _> = raw.try_into();
        assert!(converted.is_ok());
        assert_eq!(converted.unwrap(), valid_id);
    }

    #[test]
    fn compose_validates_components() {
        const CUSTOM_CONFIG: Config = Config::new(BitLayout::new(42, 8, 4, 10), 1_600_000_000_000);
        crate::id!(ComposeValidationId, CUSTOM_CONFIG);

        // Valid composition should succeed
        let result = ComposeValidationId::compose_custom(1000, 100, 5, 500);
        assert!(result.is_ok());

        // Invalid worker_id (256 > 255)
        let result = ComposeValidationId::compose_custom(1000, 256, 5, 500);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(crate::config::ValidationError::WorkerIdOutOfRange { .. })
        ));

        // Invalid process_id (16 > 15)
        let result = ComposeValidationId::compose_custom(1000, 100, 16, 500);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(crate::config::ValidationError::ProcessIdOutOfRange { .. })
        ));

        // Invalid sequence (1024 > 1023)
        let result = ComposeValidationId::compose_custom(1000, 100, 5, 1024);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(crate::config::ValidationError::SequenceOutOfRange { .. })
        ));

        // Invalid timestamp (2^42 > 2^42-1)
        let result = ComposeValidationId::compose_custom(1u64 << 42, 100, 5, 500);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(crate::config::ValidationError::TimestampOutOfRange { .. })
        ));

        // Test compose (2-param) validates timestamp and sequence
        let result = ComposeValidationId::compose(1000, 500);
        assert!(result.is_ok());

        let result = ComposeValidationId::compose(1u64 << 42, 500);
        assert!(result.is_err());

        let result = ComposeValidationId::compose(1000, 1024);
        assert!(result.is_err());
    }

    #[test]
    fn compose_unchecked_masks_overflow() {
        const CUSTOM_CONFIG: Config = Config::new(BitLayout::new(42, 8, 4, 10), 1_600_000_000_000);
        crate::id!(UncheckedComposeId, CUSTOM_CONFIG);

        // compose_custom_unchecked should mask values that exceed limits
        let id = UncheckedComposeId::compose_custom_unchecked(1u64 << 42, 256, 16, 1024);

        // Verify components are masked (not validated)
        let components = id.components();
        assert_eq!(components.timestamp, 0); // (1 << 42) masked to 42 bits = 0
        assert_eq!(components.worker_id, 0); // 256 masked to 8 bits = 0
        assert_eq!(components.process_id, 0); // 16 masked to 4 bits = 0
        assert_eq!(components.sequence, 0); // 1024 masked to 10 bits = 0

        // compose_unchecked (2-param) should mask timestamp/sequence
        let id2 = UncheckedComposeId::compose_unchecked(1u64 << 42, 1024);
        let comp2 = id2.components();
        assert_eq!(comp2.timestamp, 0);
        assert_eq!(comp2.sequence, 0);
    }

    #[test]
    fn from_str_validates() {
        const CUSTOM_CONFIG: Config = Config::new(BitLayout::new(42, 8, 4, 10), 1_600_000_000_000);
        crate::id!(ParseValidationId, CUSTOM_CONFIG);

        // Valid ID string should parse
        let valid_id = ParseValidationId::generate().unwrap();
        let id_str = valid_id.to_string();
        let parsed: Result<ParseValidationId, _> = id_str.parse();
        assert!(parsed.is_ok());
        assert_eq!(parsed.unwrap(), valid_id);

        // Invalid format should fail
        let result: Result<ParseValidationId, _> = "not_a_number".parse();
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(crate::config::ValidationError::ParseError(_))
        ));
    }

    #[test]
    fn from_u64_unchecked_no_validation() {
        const CUSTOM_CONFIG: Config = Config::new(BitLayout::new(42, 8, 4, 10), 1_600_000_000_000);
        crate::id!(UncheckedFromId, CUSTOM_CONFIG);

        // from_u64_unchecked should accept any value
        let layout = CUSTOM_CONFIG.layout;
        let invalid_value = 256u64 << layout.worker_shift(); // worker_id=256 > max=255

        // Should not panic or error
        let id = UncheckedFromId::from_u64_unchecked(invalid_value);
        assert_eq!(id.as_u64(), invalid_value);
    }
}
