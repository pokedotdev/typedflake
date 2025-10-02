use typedflake::Config;

#[test]
fn zero_process_bits() {
    // Config with zero process bits
    const CONFIG: Config = Config::new(
        typedflake::BitLayout::new(42, 10, 0, 12),
        Config::DEFAULT_EPOCH_MS,
    );
    typedflake::id!(ZeroProcessId, CONFIG);

    // Test with instance that has worker_id but process_id must be 0
    let instance = ZeroProcessId::instance(100, 0).unwrap();
    let id = instance.generate().unwrap();
    let components = id.components();

    assert_eq!(components.worker_id, 100);
    assert_eq!(components.process_id, 0);

    // Ensure compose/decompose works correctly
    let composed = ZeroProcessId::compose_custom(
        components.timestamp,
        components.worker_id,
        components.process_id,
        components.sequence,
    )
    .unwrap();
    assert_eq!(id.as_u64(), composed.as_u64());

    // Test worker method (process should default to 0)
    let worker_instance = ZeroProcessId::worker(100).unwrap();
    let worker_id = worker_instance.generate().unwrap();
    assert_eq!(worker_id.worker_id(), 100);
    assert_eq!(worker_id.process_id(), 0);

    // Test that non-zero process_id is rejected with validation error
    assert!(ZeroProcessId::instance(100, 1).is_err());
}

#[test]
fn zero_worker_bits() {
    // Config with zero worker bits
    const CONFIG: Config = Config::new(
        typedflake::BitLayout::new(42, 0, 10, 12),
        Config::DEFAULT_EPOCH_MS,
    );
    typedflake::id!(ZeroWorkerBitsId, CONFIG);

    // Test that worker_id must be 0
    let valid_instance = ZeroWorkerBitsId::instance(0, 100).unwrap();
    let id = valid_instance.generate().unwrap();
    let components = id.components();

    assert_eq!(components.worker_id, 0);
    assert_eq!(components.process_id, 100);

    // Test that non-zero worker_id is rejected
    assert!(ZeroWorkerBitsId::instance(1, 0).is_err());

    // Test process method works (worker defaults to 0)
    let process_instance = ZeroWorkerBitsId::process(500).unwrap();
    let process_id = process_instance.generate().unwrap();
    assert_eq!(process_id.worker_id(), 0);
    assert_eq!(process_id.process_id(), 500);

    // Verify composition/decomposition works
    let composed = ZeroWorkerBitsId::compose_custom(
        components.timestamp,
        components.worker_id,
        components.process_id,
        components.sequence,
    )
    .unwrap();
    assert_eq!(id.as_u64(), composed.as_u64());
}
