use typedflake::{BitLayout, Config, Epoch, TypedFlake};

const CUSTOM_CONFIG: Config =
    Config::new_unchecked(BitLayout::new(42, 8, 4, 10), Epoch::new(1_500_000_000_000));

#[derive(TypedFlake)]
#[typedflake(config = CUSTOM_CONFIG)]
pub struct CustomConfigId(u64);

#[test]
fn custom_config_with_defaults() {
    let id = CustomConfigId::generate();
    let components = id.components();

    assert!(components.timestamp > 0);

    let instance_max_worker = CustomConfigId::instance(255, 0).unwrap();
    let id_max_worker = instance_max_worker.generate();
    assert_eq!(id_max_worker.worker_id(), 255);

    let instance_max_process = CustomConfigId::instance(0, 15).unwrap();
    let id_max_process = instance_max_process.generate();
    assert_eq!(id_max_process.process_id(), 15);

    assert!(components.sequence <= 1023);
}

#[test]
fn sequence_exhaustion_and_recovery() {
    let config = Config::new_unchecked(BitLayout::new(50, 5, 5, 4), Epoch::DEFAULT);

    let generator = typedflake::Generator::new(config, 1, 1).unwrap();

    let mut count = 0;
    let mut last_result = Ok(0);

    for _ in 0..100 {
        match generator.generate_internal() {
            Ok(_) => count += 1,
            Err(e) => {
                last_result = Err(e);
                break;
            }
        }
    }

    assert!(count > 0, "Should have generated at least one ID");
    assert!(count <= 16, "Should not exceed sequence mask + 1");

    if let Err(typedflake::GeneratorError::SequenceExhausted {
        timestamp,
        worker_id,
        process_id,
    }) = last_result
    {
        assert!(timestamp > 0, "Error should contain valid timestamp");
        assert_eq!(worker_id, 1, "Error should contain correct worker_id");
        assert_eq!(process_id, 1, "Error should contain correct process_id");

        let error_msg = format!(
            "Sequence exhausted for timestamp {timestamp} on worker_id={worker_id}, process_id={process_id}"
        );
        assert!(error_msg.contains("Sequence exhausted"));
        assert!(error_msg.contains("worker_id=1"));
        assert!(error_msg.contains("process_id=1"));
    } else {
        panic!("Expected SequenceExhausted error, got: {last_result:?}");
    }

    std::thread::sleep(std::time::Duration::from_millis(2));
    let recovered_id = generator.generate();
    assert!(
        recovered_id > 0,
        "Should be able to generate ID in next millisecond"
    );
}
