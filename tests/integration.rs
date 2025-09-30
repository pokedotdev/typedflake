use typedflake::{Config, global};

#[test]
fn test_multiple_id_types_independence() {
    // Create different ID types with default config
    typedflake::id!(IdType1);
    typedflake::id!(IdType2);

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
    let instance1 = IdType1::instance(1, 1).unwrap();
    let instance2 = IdType2::instance(2, 2).unwrap();

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

    typedflake::id!(ZeroProcessId, ZERO_PROCESS_ALGORITHM);

    // Test with instance that has worker_id but process_id must be 0
    let instance = ZeroProcessId::instance(100, 0).unwrap();
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
    let worker_instance = ZeroProcessId::worker(100).unwrap();
    let worker_id = worker_instance.generate().unwrap();
    let worker_components = worker_id.components();

    assert_eq!(worker_components.worker_id, 100);
    assert_eq!(worker_components.process_id, 0);
}

#[test]
fn test_const_algorithm_integration() {
    const TEST_ALGORITHM: Config = Config::new(
        (42, 8, 4, 10),    // bits: timestamp, worker, process, sequence
        1_500_000_000_000, // epoch
    );

    typedflake::id!(ConstAlgorithmId, TEST_ALGORITHM);

    // Test custom instance
    let instance = ConstAlgorithmId::instance(20, 5).unwrap();
    let id = instance.generate().unwrap();
    let components = id.components();

    assert_eq!(components.worker_id, 20);
    assert_eq!(components.process_id, 5);

    // Test default instance
    let default_id = ConstAlgorithmId::generate().unwrap();
    let default_components = default_id.components();

    assert_eq!(default_components.worker_id, 0);
    assert_eq!(default_components.process_id, 0);
}

#[test]
fn test_blocking_generation() {
    typedflake::id!(BlockingTestId);

    // This should not hang and should generate different IDs
    let id1 = BlockingTestId::generate_blocking();
    let id2 = BlockingTestId::generate_blocking();

    assert_ne!(id1, id2);
}

#[test]
fn test_string_conversion_integration() {
    typedflake::id!(StringId);

    let id = StringId::generate().unwrap();
    let id_string = id.to_string();

    let parsed_id: StringId = id_string.parse().unwrap();
    assert_eq!(id, parsed_id);
    assert_eq!(id.as_u64(), parsed_id.as_u64());
}

#[test]
fn test_large_scale_generation() {
    typedflake::id!(ScaleId);

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
    // Shared algorithm configuration across services
    const SHARED_ALGORITHM: Config = Config::new(
        (42, 8, 4, 10), // bits: timestamp, worker, process, sequence
        1_600_000_000_000,
    );

    typedflake::id!(UserServiceId, SHARED_ALGORITHM);
    typedflake::id!(OrderServiceId, SHARED_ALGORITHM);

    // Different instances for different services
    let user_instance = UserServiceId::instance(1, 0).unwrap();
    let order_instance = OrderServiceId::instance(2, 1).unwrap();

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
    let another_user = UserServiceId::worker(3).unwrap();
    let another_order = OrderServiceId::process(2).unwrap();

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

    typedflake::id!(StressTestId);

    const NUM_THREADS: usize = 8;
    const IDS_PER_THREAD: usize = 1000;

    let handles: Vec<_> = (0..NUM_THREADS)
        .map(|thread_id| {
            thread::spawn(move || {
                let instance = StressTestId::instance(thread_id as u64, 0).unwrap();
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

    // Test default fallback behavior
    let default_config = global::get_default_config();
    let hardcoded_default = Config::default();

    assert_eq!(
        default_config.timestamp_bits,
        hardcoded_default.timestamp_bits
    );
    assert_eq!(default_config.epoch_ms, hardcoded_default.epoch_ms);

    let (worker_id, process_id) = global::get_default_instance();
    assert_eq!(worker_id, 0);
    assert_eq!(process_id, 0);
}

#[test]
fn test_custom_config_still_works_with_defaults() {
    // Verify that explicit custom configs are not affected by default config system
    const CUSTOM_ALGORITHM: Config = Config::new((42, 8, 4, 10), 1_500_000_000_000);

    typedflake::id!(CustomConfigId, CUSTOM_ALGORITHM);

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

    typedflake::id!(IsolationTestId);

    // Test that different (worker_id, process_id) pairs have independent sequences
    const NUM_INSTANCES: usize = 4;
    const IDS_PER_INSTANCE: usize = 100;

    let handles: Vec<_> = (0..NUM_INSTANCES)
        .map(|i| {
            thread::spawn(move || {
                let worker_id = i as u64;
                let process_id = (i * 2) as u64; // Different process IDs
                let instance = IsolationTestId::instance(worker_id, process_id).unwrap();

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

    typedflake::id!(SameInstanceTestId);

    // Test that the same (worker_id, process_id) from different threads shares state correctly
    const NUM_THREADS: usize = 4;
    const IDS_PER_THREAD: usize = 250;
    const WORKER_ID: u64 = 10; // Must fit in 5 bits (max 31)
    const PROCESS_ID: u64 = 7;

    let handles: Vec<_> = (0..NUM_THREADS)
        .map(|_| {
            thread::spawn(move || {
                let instance = SameInstanceTestId::instance(WORKER_ID, PROCESS_ID).unwrap();
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