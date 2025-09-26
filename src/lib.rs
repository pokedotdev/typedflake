//! # TypedFlake
//!
//! A Snowflake-style ID generator library with newtype-driven design.
//!
//! TypedFlake provides a flexible and type-safe way to generate distributed unique IDs
//! similar to Twitter's Snowflake algorithm. Each ID type has its own independent generator
//! state and configuration, ensuring thread-safety and preventing ID collisions between
//! different entity types.
//!
//! ## Features
//!
//! - **Type-safe ID generation**: Each ID type is a distinct newtype
//! - **Customizable bit allocation**: Configure timestamp, worker, process, and sequence bits
//! - **Custom epoch support**: Set your own epoch for timestamp calculation
//! - **Thread-safe**: Each ID type maintains its own independent generator state
//! - **Zero-cost abstractions**: Pre-calculated shifts and masks for optimal performance
//! - **Flexible configuration**: Runtime builder pattern or compile-time const creation
//!
//! ## Quick Start
//!
//! ```rust
//! // Create an ID type with default configuration
//! typedflake::id!(UserId);
//!
//! // Generate IDs (default instance with worker_id=0, process_id=0)
//! let user_id = UserId::generate().unwrap();
//! println!("Generated user ID: {}", user_id);
//!
//! // Generate with specific instance
//! let user_instance = UserId::instance(42, 7);
//! let custom_id = user_instance.generate().unwrap();
//!
//! // Access components
//! let timestamp = user_id.timestamp();
//! let (timestamp, worker_id, process_id, sequence) = user_id.decompose();
//! ```
//!
//! ## Custom Configuration
//!
//! ```rust
//! use typedflake::Config;
//!
//! // Custom algorithm configuration
//! const SESSION_ALGORITHM: Config = Config::new(
//!     (42, 10, 0, 12),   // bits: timestamp, worker, process, sequence
//!     1_600_000_000_000, // epoch
//! );
//!
//! typedflake::id!(SessionId, SESSION_ALGORITHM);
//!
//! // Another ID type with different algorithm
//! const USER_ALGORITHM: Config = Config::new(
//!     (42, 10, 5, 7),
//!     1_600_000_000_000,
//! );
//!
//! typedflake::id!(UserId, USER_ALGORITHM);
//!
//! let session_id = SessionId::generate().unwrap();
//! let user_id = UserId::generate().unwrap();
//! println!("Generated session ID: {}", session_id);
//! println!("Generated user ID: {}", user_id);
//! ```
//!
//! ## Shared Algorithm with Dynamic Instances
//!
//! ```rust
//! use typedflake::Config;
//!
//! // Shared algorithm configuration across services
//! const SHARED_ALGORITHM: Config = Config::new(
//!     (42, 6, 4, 12),        // bits: timestamp, worker, process, sequence
//!     1_640_000_000_000,     // 2022 epoch
//! );
//!
//! // Service-specific ID types
//! typedflake::id!(AuthServiceId, SHARED_ALGORITHM);
//! typedflake::id!(PaymentServiceId, SHARED_ALGORITHM);
//!
//! // Generate IDs with dynamic instances
//! let auth_instance = AuthServiceId::instance(1, 0);
//! let payment_instance = PaymentServiceId::instance(2, 1);
//!
//! let auth_id = auth_instance.generate().unwrap();
//! let payment_id = payment_instance.generate().unwrap();
//! ```
//!
//! ## ID Operations
//!
//! ```rust
//! # use typedflake::Config;
//! # typedflake::id!(ExampleId);
//! let id = ExampleId::generate().unwrap();
//!
//! // Convert to/from u64
//! let raw: u64 = id.as_u64();
//! let id2 = ExampleId::from_u64(raw);
//!
//! // Decompose into components
//! let (timestamp, worker_id, process_id, sequence) = id.decompose();
//! let components = id.components();
//!
//! // Access individual components
//! let timestamp = id.timestamp();
//! let worker_id = id.worker_id();
//! let process_id = id.process_id();
//! let sequence = id.sequence();
//!
//! // Compose from components
//! let composed_id = ExampleId::compose(timestamp, worker_id, process_id, sequence);
//! ```

pub mod config;
pub mod factory;
pub mod generator;
pub mod global;
pub mod state;

mod macros; // Keep macros private, they're exported via the macro itself

// Re-export main types and the macro
pub use config::Config;
pub use factory::GeneratorFactory;
pub use generator::{Generator, GeneratorError, IdComponents};

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_multiple_id_types_independence() {
        // Create different ID types with different algorithms
        const ALGORITHM1: Config = Config::new(
            (40, 10, 6, 8), // bits: timestamp, worker, process, sequence
            Config::DEFAULT_EPOCH_MS,
        );

        const ALGORITHM2: Config = Config::new(
            (42, 8, 4, 10), // bits: timestamp, worker, process, sequence
            Config::DEFAULT_EPOCH_MS,
        );

        crate::id!(IdType1, ALGORITHM1);
        crate::id!(IdType2, ALGORITHM2);

        // Test default instances (0, 0)
        let id1 = IdType1::generate().unwrap();
        let id2 = IdType2::generate().unwrap();

        let components1 = id1.components();
        let components2 = id2.components();

        // Both should use default instance (0, 0)
        assert_eq!(components1.worker_id, 0);
        assert_eq!(components1.process_id, 0);
        assert_eq!(components2.worker_id, 0);
        assert_eq!(components2.process_id, 0);

        // Test custom instances
        let instance1 = IdType1::instance(1, 1);
        let instance2 = IdType2::instance(2, 2);

        let custom_id1 = instance1.generate().unwrap();
        let custom_id2 = instance2.generate().unwrap();

        let custom_components1 = custom_id1.components();
        let custom_components2 = custom_id2.components();

        assert_eq!(custom_components1.worker_id, 1);
        assert_eq!(custom_components1.process_id, 1);
        assert_eq!(custom_components2.worker_id, 2);
        assert_eq!(custom_components2.process_id, 2);

        // Generate multiple IDs from each type to ensure independence
        let id1_2 = IdType1::generate().unwrap();
        let id2_2 = IdType2::generate().unwrap();

        assert_ne!(id1, id1_2);
        assert_ne!(id2, id2_2);
    }

    #[test]
    fn test_zero_process_bits_integration() {
        const ZERO_PROCESS_ALGORITHM: Config = Config::new(
            (41, 10, 0, 13), // bits: timestamp, worker, process, sequence (zero process bits)
            Config::DEFAULT_EPOCH_MS,
        );

        crate::id!(ZeroProcessId, ZERO_PROCESS_ALGORITHM);

        // Test with instance that has worker_id but process_id must be 0
        let instance = ZeroProcessId::instance(100, 0);
        let id = instance.generate().unwrap();
        let components = id.components();

        assert_eq!(components.worker_id, 100);
        assert_eq!(components.process_id, 0);

        // Ensure compose/decompose works correctly
        let composed = ZeroProcessId::compose(
            components.timestamp,
            components.worker_id,
            components.process_id,
            components.sequence,
        );

        assert_eq!(id.as_u64(), composed.as_u64());

        // Test worker method (process should default to 0)
        let worker_instance = ZeroProcessId::worker(100);
        let worker_id = worker_instance.generate().unwrap();
        let worker_components = worker_id.components();

        assert_eq!(worker_components.worker_id, 100);
        assert_eq!(worker_components.process_id, 0);
    }

    #[test]
    fn test_const_algorithm_integration() {
        const TEST_ALGORITHM: Config = Config::new(
            (41, 10, 5, 8),    // bits: timestamp, worker, process, sequence
            1_500_000_000_000, // epoch
        );

        crate::id!(ConstAlgorithmId, TEST_ALGORITHM);

        // Test custom instance
        let instance = ConstAlgorithmId::instance(42, 7);
        let id = instance.generate().unwrap();
        let components = id.components();

        assert_eq!(components.worker_id, 42);
        assert_eq!(components.process_id, 7);

        // Test default instance
        let default_id = ConstAlgorithmId::generate().unwrap();
        let default_components = default_id.components();

        assert_eq!(default_components.worker_id, 0);
        assert_eq!(default_components.process_id, 0);
    }

    #[test]
    fn test_blocking_generation() {
        crate::id!(BlockingTestId);

        // This should not hang and should generate different IDs
        let id1 = BlockingTestId::generate_blocking();
        let id2 = BlockingTestId::generate_blocking();

        assert_ne!(id1, id2);
    }

    #[test]
    fn test_string_conversion_integration() {
        crate::id!(StringId);

        let id = StringId::generate().unwrap();
        let id_string = id.to_string();

        let parsed_id: StringId = id_string.parse().unwrap();
        assert_eq!(id, parsed_id);
        assert_eq!(id.as_u64(), parsed_id.as_u64());
    }

    #[test]
    fn test_large_scale_generation() {
        crate::id!(ScaleId);

        let mut ids = std::collections::HashSet::new();

        // Generate many IDs and ensure they're all unique
        for _ in 0..1000 {
            let id = ScaleId::generate_blocking();
            assert!(ids.insert(id), "Duplicate ID generated: {id}");
        }

        assert_eq!(ids.len(), 1000);
    }

    #[test]
    fn test_shared_algorithm_different_instances() {
        use super::*;

        // Shared algorithm configuration across services
        const SHARED_ALGORITHM: Config = Config::new(
            (42, 10, 5, 7), // bits: timestamp, worker, process, sequence
            1_600_000_000_000,
        );

        crate::id!(UserServiceId, SHARED_ALGORITHM);
        crate::id!(OrderServiceId, SHARED_ALGORITHM);

        // Different instances for different services
        let user_instance = UserServiceId::instance(1, 0);
        let order_instance = OrderServiceId::instance(2, 1);

        let user_id = user_instance.generate().unwrap();
        let order_id = order_instance.generate().unwrap();

        let user_components = user_id.components();
        let order_components = order_id.components();

        // Should have different worker IDs
        assert_eq!(user_components.worker_id, 1);
        assert_eq!(order_components.worker_id, 2);

        // Should have different process IDs
        assert_eq!(user_components.process_id, 0);
        assert_eq!(order_components.process_id, 1);

        // Both should use the shared algorithm config
        assert!(user_id.as_u64() > 0);
        assert!(order_id.as_u64() > 0);

        // Test different worker/process combinations
        let another_user = UserServiceId::worker(3);
        let another_order = OrderServiceId::process(2);

        let another_user_id = another_user.generate().unwrap();
        let another_order_id = another_order.generate().unwrap();

        let another_user_components = another_user_id.components();
        let another_order_components = another_order_id.components();

        assert_eq!(another_user_components.worker_id, 3);
        assert_eq!(another_user_components.process_id, 0); // worker() sets process to 0
        assert_eq!(another_order_components.worker_id, 0); // process() sets worker to 0
        assert_eq!(another_order_components.process_id, 2);
    }

    #[test]
    fn test_lock_free_concurrency_stress() {
        use std::thread;

        crate::id!(StressTestId);

        const NUM_THREADS: usize = 8;
        const IDS_PER_THREAD: usize = 1000;

        let handles: Vec<_> = (0..NUM_THREADS)
            .map(|thread_id| {
                thread::spawn(move || {
                    let instance = StressTestId::instance(thread_id as u64, 0);
                    let mut ids = Vec::with_capacity(IDS_PER_THREAD);

                    for _ in 0..IDS_PER_THREAD {
                        let id = instance.generate_blocking();
                        ids.push(id);
                    }

                    // Verify all IDs are unique within this thread
                    let mut sorted_ids = ids.clone();
                    sorted_ids.sort();
                    sorted_ids.dedup();
                    assert_eq!(
                        ids.len(),
                        sorted_ids.len(),
                        "Duplicate IDs generated in thread {thread_id}"
                    );

                    ids
                })
            })
            .collect();

        // Collect all generated IDs
        let mut all_ids = Vec::new();
        for handle in handles {
            all_ids.extend(handle.join().unwrap());
        }

        // Verify global uniqueness
        let original_len = all_ids.len();
        all_ids.sort();
        all_ids.dedup();
        assert_eq!(original_len, all_ids.len(), "Duplicate IDs across threads");
        assert_eq!(original_len, NUM_THREADS * IDS_PER_THREAD);
    }

    #[test]
    fn test_default_configuration_integration() {
        // This test demonstrates default configuration behavior
        // Note: Since OnceLock can only be set once per program execution,
        // we test the initialization behavior and status checking
        use super::*;

        // Test default fallback behavior
        let default_config = global::get_default_config();
        let hardcoded_default = Config::default();

        assert_eq!(
            default_config.bits.timestamp,
            hardcoded_default.bits.timestamp
        );
        assert_eq!(default_config.epoch_ms, hardcoded_default.epoch_ms);

        let (worker_id, process_id) = global::get_default_instance();
        assert_eq!(worker_id, 0);
        assert_eq!(process_id, 0);
    }

    #[test]
    fn test_custom_config_still_works_with_defaults() {
        // Verify that explicit custom configs are not affected by default config system
        const CUSTOM_ALGORITHM: Config = Config::new((40, 12, 4, 8), 1_500_000_000_000);

        crate::id!(CustomConfigId, CUSTOM_ALGORITHM);

        let id = CustomConfigId::generate().unwrap();
        let components = id.components();

        // Should use custom config, not default config
        assert!(components.timestamp > 0);
        // The exact timestamp depends on current time and epoch, but we can verify
        // the structure is consistent with the custom config bit allocation
    }

    #[test]
    fn test_per_instance_state_isolation() {
        use std::thread;

        crate::id!(IsolationTestId);

        // Test that different (worker_id, process_id) pairs have independent sequences
        const NUM_INSTANCES: usize = 4;
        const IDS_PER_INSTANCE: usize = 100;

        let handles: Vec<_> = (0..NUM_INSTANCES)
            .map(|i| {
                thread::spawn(move || {
                    let worker_id = i as u64;
                    let process_id = (i * 2) as u64; // Different process IDs
                    let instance = IsolationTestId::instance(worker_id, process_id);

                    let mut sequences = Vec::new();

                    // Generate IDs rapidly to test sequence isolation
                    for _ in 0..IDS_PER_INSTANCE {
                        let id = instance.generate_blocking();
                        let components = id.components();

                        // Verify correct instance IDs
                        assert_eq!(components.worker_id, worker_id);
                        assert_eq!(components.process_id, process_id);

                        sequences.push(components.sequence);
                    }

                    (worker_id, process_id, sequences)
                })
            })
            .collect();

        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        // Verify that each instance has its own sequence progression
        for (worker_id, process_id, sequences) in results {
            // Each instance should start from sequence 0 and increment
            assert_eq!(
                sequences[0], 0,
                "Instance ({worker_id}, {process_id}) should start with sequence 0"
            );

            // Sequences should be generally increasing (allowing for potential millisecond resets)
            let max_sequence = *sequences.iter().max().unwrap();
            assert!(
                max_sequence >= IDS_PER_INSTANCE as u64 / 2,
                "Instance ({worker_id}, {process_id}) should have reasonable sequence progression"
            );
        }
    }

    #[test]
    fn test_same_instance_different_threads() {
        use std::thread;

        crate::id!(SameInstanceTestId);

        // Test that the same (worker_id, process_id) from different threads shares state correctly
        const NUM_THREADS: usize = 4;
        const IDS_PER_THREAD: usize = 250;
        const WORKER_ID: u64 = 10; // Must fit in 5 bits (max 31)
        const PROCESS_ID: u64 = 7;

        let handles: Vec<_> = (0..NUM_THREADS)
            .map(|_| {
                thread::spawn(move || {
                    let instance = SameInstanceTestId::instance(WORKER_ID, PROCESS_ID);
                    let mut ids = Vec::new();

                    for _ in 0..IDS_PER_THREAD {
                        let id = instance.generate_blocking();
                        ids.push(id);
                    }

                    ids
                })
            })
            .collect();

        // Collect all IDs from all threads
        let mut all_ids: Vec<SameInstanceTestId> = Vec::new();
        for handle in handles {
            all_ids.extend(handle.join().unwrap());
        }

        // All IDs should be unique even though they share the same (worker_id, process_id)
        let original_len = all_ids.len();
        all_ids.sort();
        all_ids.dedup();
        assert_eq!(
            original_len,
            all_ids.len(),
            "Duplicate IDs generated from same instance across threads"
        );

        // Verify all IDs have the correct worker_id and process_id
        for id in &all_ids {
            let components = id.components();
            assert_eq!(components.worker_id, WORKER_ID);
            assert_eq!(components.process_id, PROCESS_ID);
        }
    }
}
