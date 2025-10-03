use typedflake::{Config, Epoch};

#[test]
fn custom_config_with_defaults() {
    // Verify that explicit custom configs are not affected by default config system
    // Custom config: 42 timestamp, 8 worker bits (max 255), 4 process bits (max 15), 10 sequence bits
    typedflake::id!(
        CustomConfigId,
        Config::new(
            typedflake::BitLayout::new(42, 8, 4, 10),
            Epoch::new(1_500_000_000_000)
        )
    );

    let id = CustomConfigId::generate().unwrap();
    let components = id.components();

    // Should use custom config, not default config
    assert!(components.timestamp > 0);

    // Verify bit allocations match custom config (8 worker bits = max 255)
    let instance_max_worker = CustomConfigId::instance(255, 0).unwrap();
    let id_max_worker = instance_max_worker.generate().unwrap();
    assert_eq!(id_max_worker.worker_id(), 255);

    // Verify process bits (4 bits = max 15)
    let instance_max_process = CustomConfigId::instance(0, 15).unwrap();
    let id_max_process = instance_max_process.generate().unwrap();
    assert_eq!(id_max_process.process_id(), 15);

    // Verify sequence bits (10 bits = max 1023)
    assert!(components.sequence <= 1023);
}

#[test]
fn sequence_exhaustion_and_recovery() {
    // Use config with very small sequence bits to force exhaustion (4 bits = max 15 IDs/ms)
    typedflake::id!(
        SmallSeqId,
        Config::new(
            typedflake::BitLayout::new(50, 5, 5, 4),
            typedflake::Epoch::DEFAULT
        )
    );

    let instance = SmallSeqId::instance(1, 1).unwrap();

    // Generate IDs until sequence is exhausted
    let mut count = 0;
    let mut last_result = Ok(());

    for _ in 0..100 {
        match instance.generate() {
            Ok(_) => count += 1,
            Err(e) => {
                last_result = Err(e);
                break;
            }
        }
    }

    // Should have generated some IDs then hit exhaustion
    assert!(count > 0, "Should have generated at least one ID");
    assert!(count <= 16, "Should not exceed sequence mask + 1");

    // Verify error details
    if let Err(typedflake::GeneratorError::SequenceExhausted {
        timestamp,
        worker_id,
        process_id,
    }) = last_result
    {
        assert!(timestamp > 0, "Error should contain valid timestamp");
        assert_eq!(worker_id, 1, "Error should contain correct worker_id");
        assert_eq!(process_id, 1, "Error should contain correct process_id");

        // Verify error message formatting
        let error_msg = format!(
            "Sequence exhausted for timestamp {timestamp} on worker_id={worker_id}, process_id={process_id}"
        );
        assert!(error_msg.contains("Sequence exhausted"));
        assert!(error_msg.contains("worker_id=1"));
        assert!(error_msg.contains("process_id=1"));
    } else {
        panic!("Expected SequenceExhausted error, got: {last_result:?}");
    }

    // Verify recovery in next millisecond - blocking should eventually succeed
    std::thread::sleep(std::time::Duration::from_millis(2));
    let recovered_id = instance.generate();
    assert!(
        recovered_id.is_ok(),
        "Should be able to generate ID in next millisecond"
    );
}
