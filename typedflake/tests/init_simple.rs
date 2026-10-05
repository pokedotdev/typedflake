//! Global initialization with a plain node. The default is process-wide, so
//! this file is one ordered test in its own process.

use std::collections::HashSet;
use std::thread;
use std::time::Duration;

use typedflake::{GenerateError, GeneratorError, InitError, NodeError, TypedNode, typedflake};

#[derive(Debug, Clone, Copy, TypedNode)]
pub struct AppNode {
    #[node(bits = 5)]
    pub worker: u8,
    #[node(bits = 5)]
    pub process: u8,
}

#[typedflake(epoch = "2025-01-01")]
pub struct UserId(i64);

#[typedflake(epoch = "2025-01-01")]
pub struct OrderId(u64);

#[typedflake(epoch = "2025-01-01", bits(timestamp = 41, node = 3, sequence = 12))]
pub struct NarrowId(i64);

#[typedflake(epoch = "2025-01-01", node = AppNode)]
pub struct TypedId(i64);

#[test]
fn plain_node_default_lifecycle() {
    // Before initialization: an error, and nothing is frozen.
    assert!(matches!(
        UserId::generate(),
        Err(GenerateError::NotInitialized)
    ));
    assert!(matches!(
        UserId::generate_blocking(Duration::from_millis(1)),
        Err(GenerateError::NotInitialized)
    ));

    // Reading and explicit generators never need initialization.
    assert_eq!(UserId::try_from(42_i64).unwrap().get(), 42);
    assert_eq!(
        UserId::generator(3)
            .unwrap()
            .generate()
            .unwrap()
            .parts()
            .node,
        3
    );

    assert_eq!(typedflake::init(17), Ok(()));
    assert_eq!(typedflake::init(17), Err(InitError::AlreadyInitialized));
    assert_eq!(typedflake::init(18), Err(InitError::AlreadyInitialized));
    let typed = AppNode {
        worker: 1,
        process: 1,
    };
    assert_eq!(typedflake::init(typed), Err(InitError::AlreadyInitialized));

    // Every plain-node ID type picks up the default.
    assert_eq!(UserId::generate().unwrap().parts().node, 17);
    assert_eq!(OrderId::generate().unwrap().parts().node, 17);
    assert_eq!(
        UserId::generate_blocking(Duration::from_millis(50))
            .unwrap()
            .parts()
            .node,
        17
    );

    // A default that does not fit one ID's node width fails only for that ID.
    assert!(matches!(
        NarrowId::generate(),
        Err(GenerateError::DefaultGenerator(GeneratorError::InvalidNode(error)))
            if error == NodeError::new("node", 17, 7)
    ));

    // A typed-node ID does not consume a plain default of the same width.
    match TypedId::generate() {
        Err(GenerateError::NodeSchemaMismatch { expected, found }) => {
            assert!(expected.ends_with("AppNode"), "{expected}");
            assert_eq!(found, "u32");
        }
        other => panic!("expected a schema mismatch, got {other:?}"),
    }
    assert!(TypedId::generator(typed).unwrap().generate().is_ok());

    // The static path and an explicit generator for the same node coordinate.
    let ids: Vec<UserId> = thread::scope(|scope| {
        let statics = scope.spawn(|| {
            (0..20_000)
                .map(|_| UserId::generate_blocking(Duration::from_secs(5)).unwrap())
                .collect::<Vec<_>>()
        });
        let explicit = scope.spawn(|| {
            let generator = UserId::generator(17).unwrap();
            (0..20_000)
                .map(|_| generator.generate_blocking(Duration::from_secs(5)).unwrap())
                .collect::<Vec<_>>()
        });
        let mut ids = statics.join().unwrap();
        ids.extend(explicit.join().unwrap());
        ids
    });
    let unique: HashSet<UserId> = ids.iter().copied().collect();
    assert_eq!(unique.len(), ids.len());
}
