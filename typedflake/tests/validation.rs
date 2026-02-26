use typedflake::{BitLayout, Config, Epoch, TypedFlake};

const ZERO_PROCESS_CONFIG: Config = Config::new(BitLayout::new(42, 10, 0, 12), Epoch::DEFAULT);

const ZERO_WORKER_CONFIG: Config = Config::new(BitLayout::new(42, 0, 10, 12), Epoch::DEFAULT);

#[derive(TypedFlake)]
#[typedflake(config = ZERO_PROCESS_CONFIG)]
pub struct ZeroProcessId(u64);

#[derive(TypedFlake)]
#[typedflake(config = ZERO_WORKER_CONFIG)]
pub struct ZeroWorkerBitsId(u64);

#[test]
fn zero_process_bits() {
    let instance = ZeroProcessId::instance(100, 0).unwrap();
    let id = instance.generate();
    let components = id.components();

    assert_eq!(components.worker_id, 100);
    assert_eq!(components.process_id, 0);

    let composed = ZeroProcessId::compose_custom(
        components.timestamp,
        components.worker_id,
        components.process_id,
        components.sequence,
    )
    .unwrap();
    assert_eq!(id.as_u64(), composed.as_u64());

    let worker_instance = ZeroProcessId::worker(100).unwrap();
    let worker_id = worker_instance.generate();
    assert_eq!(worker_id.worker_id(), 100);
    assert_eq!(worker_id.process_id(), 0);

    assert!(ZeroProcessId::instance(100, 1).is_err());
}

#[test]
fn zero_worker_bits() {
    let valid_instance = ZeroWorkerBitsId::instance(0, 100).unwrap();
    let id = valid_instance.generate();
    let components = id.components();

    assert_eq!(components.worker_id, 0);
    assert_eq!(components.process_id, 100);

    assert!(ZeroWorkerBitsId::instance(1, 0).is_err());

    let process_instance = ZeroWorkerBitsId::process(500).unwrap();
    let process_id = process_instance.generate();
    assert_eq!(process_id.worker_id(), 0);
    assert_eq!(process_id.process_id(), 500);

    let composed = ZeroWorkerBitsId::compose_custom(
        components.timestamp,
        components.worker_id,
        components.process_id,
        components.sequence,
    )
    .unwrap();
    assert_eq!(id.as_u64(), composed.as_u64());
}
