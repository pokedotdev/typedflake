use typedflake::{BitLayout, Config, Epoch, TypedFlake};

// ==================== ID Types ====================

#[derive(TypedFlake)]
pub struct TestId(u64);

const CUSTOM_CONFIG: Config =
    Config::new_unchecked(BitLayout::new(42, 8, 4, 10), Epoch::new(1_600_000_000_000));

#[derive(TypedFlake)]
#[typedflake(config = CUSTOM_CONFIG)]
pub struct CustomId(u64);

#[derive(TypedFlake)]
pub struct ComposeId(u64);

#[derive(TypedFlake)]
pub struct ComponentId(u64);

#[derive(TypedFlake)]
pub struct ConversionId(u64);

#[derive(TypedFlake)]
pub struct ParseId(u64);

#[derive(TypedFlake)]
pub struct DeriveUserId(u64);

#[derive(TypedFlake)]
pub struct DeriveOrderId(u64);

const CUSTOM_ALGORITHM: Config =
    Config::new_unchecked(BitLayout::new(42, 8, 4, 10), Epoch::new(1_600_000_000_000));

#[derive(TypedFlake)]
#[typedflake(config = CUSTOM_ALGORITHM)]
pub struct InstanceMethodId(u64);

#[derive(TypedFlake)]
pub struct MultiInstanceId(u64);

#[derive(TypedFlake)]
pub struct FactoryTestId(u64);

const VALIDATION_CONFIG: Config =
    Config::new_unchecked(BitLayout::new(42, 8, 4, 10), Epoch::new(1_600_000_000_000));

#[derive(TypedFlake)]
#[typedflake(config = VALIDATION_CONFIG)]
pub struct ValidationTestId(u64);

#[derive(TypedFlake)]
pub struct TryFromTestId(u64);

const COMPOSE_VALIDATION_CONFIG: Config =
    Config::new_unchecked(BitLayout::new(42, 8, 4, 10), Epoch::new(1_600_000_000_000));

#[derive(TypedFlake)]
#[typedflake(config = COMPOSE_VALIDATION_CONFIG)]
pub struct ComposeValidationId(u64);

const UNCHECKED_CONFIG: Config =
    Config::new_unchecked(BitLayout::new(42, 8, 4, 10), Epoch::new(1_600_000_000_000));

#[derive(TypedFlake)]
#[typedflake(config = UNCHECKED_CONFIG)]
pub struct UncheckedComposeId(u64);

const PARSE_VALIDATION_CONFIG: Config =
    Config::new_unchecked(BitLayout::new(42, 8, 4, 10), Epoch::new(1_600_000_000_000));

#[derive(TypedFlake)]
#[typedflake(config = PARSE_VALIDATION_CONFIG)]
pub struct ParseValidationId(u64);

const UNCHECKED_FROM_CONFIG: Config =
    Config::new_unchecked(BitLayout::new(42, 8, 4, 10), Epoch::new(1_600_000_000_000));

#[derive(TypedFlake)]
#[typedflake(config = UNCHECKED_FROM_CONFIG)]
pub struct UncheckedFromId(u64);

// ==================== Tests ====================

#[test]
fn macro_generates_default_config() {
    let id1 = TestId::generate();
    let id2 = TestId::generate();

    assert_ne!(id1, id2);
    assert!(id1.as_u64() > 0);
    assert!(id2.as_u64() > 0);

    let components = id1.components();
    assert_eq!(components.worker_id, 0);
    assert_eq!(components.process_id, 0);
}

#[test]
fn macro_with_custom_config() {
    let id = CustomId::generate();
    let components = id.components();

    assert_eq!(components.worker_id, 0);
    assert_eq!(components.process_id, 0);

    let instance = CustomId::instance(50, 5).unwrap();
    let instance_id = instance.generate();
    let instance_components = instance_id.components();

    assert_eq!(instance_components.worker_id, 50);
    assert_eq!(instance_components.process_id, 5);
}

#[test]
fn compose_decompose_roundtrip() {
    let timestamp = 123456789;
    let sequence = 100;

    let id = ComposeId::compose(timestamp, sequence).unwrap();
    let (dec_timestamp, dec_worker_id, dec_process_id, dec_sequence) = id.decompose();

    assert_eq!(dec_timestamp, timestamp);
    assert_eq!(dec_worker_id, 0);
    assert_eq!(dec_process_id, 0);
    assert_eq!(dec_sequence, sequence);

    let id_custom = ComposeId::compose_custom(timestamp, 5, 3, sequence).unwrap();
    let (ts, w, p, s) = id_custom.decompose();
    assert_eq!(ts, timestamp);
    assert_eq!(w, 5);
    assert_eq!(p, 3);
    assert_eq!(s, sequence);
}

#[test]
fn extract_individual_components() {
    let id = ComponentId::generate();

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
    let id = ConversionId::generate();
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
    let id = ParseId::generate();
    let id_str = id.to_string();

    let parsed_id: ParseId = id_str.parse().unwrap();
    assert_eq!(id, parsed_id);
}

#[test]
fn multiple_types_independent() {
    let user_id1 = DeriveUserId::generate();
    let user_id2 = DeriveUserId::generate();
    let order_id1 = DeriveOrderId::generate();
    let order_id2 = DeriveOrderId::generate();

    assert_ne!(user_id1, user_id2);
    assert_ne!(order_id1, order_id2);

    assert!(user_id1.as_u64() > 0);
    assert!(order_id1.as_u64() > 0);

    assert!(user_id1.as_u64() != user_id2.as_u64());
    assert!(order_id1.as_u64() != order_id2.as_u64());
}

#[test]
fn instance_convenience_methods() {
    let worker_instance = InstanceMethodId::worker(50).unwrap();
    let worker_id = worker_instance.generate();
    let worker_components = worker_id.components();
    assert_eq!(worker_components.worker_id, 50);
    assert_eq!(worker_components.process_id, 0);

    let process_instance = InstanceMethodId::process(5).unwrap();
    let process_id = process_instance.generate();
    let process_components = process_id.components();
    assert_eq!(process_components.worker_id, 0);
    assert_eq!(process_components.process_id, 5);

    let full_instance = InstanceMethodId::instance(50, 5).unwrap();
    let full_id = full_instance.generate();
    let full_components = full_id.components();
    assert_eq!(full_components.worker_id, 50);
    assert_eq!(full_components.process_id, 5);
}

#[test]
fn multiple_instances_independent_state() {
    let instance1 = MultiInstanceId::instance(10, 5).unwrap();
    let instance2 = MultiInstanceId::instance(1, 2).unwrap();

    let id1 = instance1.generate();
    let id2 = instance2.generate();

    let components1 = id1.components();
    let components2 = id2.components();

    assert_eq!(components1.worker_id, 10);
    assert_eq!(components1.process_id, 5);
    assert_eq!(components2.worker_id, 1);
    assert_eq!(components2.process_id, 2);
}

#[test]
fn generator_with_bound_ids() {
    let stateful_gen = FactoryTestId::instance(15, 7).unwrap();

    assert_eq!(stateful_gen.worker_id(), 15);
    assert_eq!(stateful_gen.process_id(), 7);

    let id1 = stateful_gen.generate();
    let id2 = stateful_gen.generate();

    let components1 = id1.components();
    let components2 = id2.components();

    assert_eq!(components1.worker_id, 15);
    assert_eq!(components1.process_id, 7);
    assert_eq!(components2.worker_id, 15);
    assert_eq!(components2.process_id, 7);

    assert_ne!(id1, id2);
}

#[test]
fn try_from_u64_validates_components() {
    let valid_id = ValidationTestId::generate();
    let raw = valid_id.as_u64();

    assert!(ValidationTestId::try_from_u64(raw).is_ok());
}

#[test]
fn try_from_trait_validates() {
    let valid_id = TryFromTestId::generate();
    let raw = valid_id.as_u64();

    let converted: Result<TryFromTestId, _> = raw.try_into();
    assert!(converted.is_ok());
    assert_eq!(converted.unwrap(), valid_id);
}

#[test]
fn compose_validates_components() {
    let result = ComposeValidationId::compose_custom(1000, 100, 5, 500);
    assert!(result.is_ok());

    let result = ComposeValidationId::compose_custom(1000, 256, 5, 500);
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(typedflake::ValidationError::WorkerIdOutOfRange { .. })
    ));

    let result = ComposeValidationId::compose_custom(1000, 100, 16, 500);
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(typedflake::ValidationError::ProcessIdOutOfRange { .. })
    ));

    let result = ComposeValidationId::compose_custom(1000, 100, 5, 1024);
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(typedflake::ValidationError::SequenceOutOfRange { .. })
    ));

    let result = ComposeValidationId::compose_custom(1u64 << 42, 100, 5, 500);
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(typedflake::ValidationError::TimestampOutOfRange { .. })
    ));

    let result = ComposeValidationId::compose(1000, 500);
    assert!(result.is_ok());

    let result = ComposeValidationId::compose(1u64 << 42, 500);
    assert!(result.is_err());

    let result = ComposeValidationId::compose(1000, 1024);
    assert!(result.is_err());
}

#[test]
fn compose_unchecked_masks_overflow() {
    let id = UncheckedComposeId::compose_custom_unchecked(1u64 << 42, 256, 16, 1024);

    let components = id.components();
    assert_eq!(components.timestamp, 0);
    assert_eq!(components.worker_id, 0);
    assert_eq!(components.process_id, 0);
    assert_eq!(components.sequence, 0);

    let id2 = UncheckedComposeId::compose_unchecked(1u64 << 42, 1024);
    let comp2 = id2.components();
    assert_eq!(comp2.timestamp, 0);
    assert_eq!(comp2.sequence, 0);
}

#[test]
fn from_str_validates() {
    let valid_id = ParseValidationId::generate();
    let id_str = valid_id.to_string();
    let parsed: Result<ParseValidationId, _> = id_str.parse();
    assert!(parsed.is_ok());
    assert_eq!(parsed.unwrap(), valid_id);

    let result: Result<ParseValidationId, _> = "not_a_number".parse();
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(typedflake::ValidationError::StringParseError(_))
    ));
}

#[test]
fn from_u64_unchecked_no_validation() {
    let layout = UNCHECKED_FROM_CONFIG.layout();
    let invalid_value = 256u64 << layout.worker_shift();

    let id = UncheckedFromId::from_u64_unchecked(invalid_value);
    assert_eq!(id.as_u64(), invalid_value);
}
