//! Global initialization with a typed node. The default is process-wide, so
//! this file is one ordered test in its own process.

use typedflake::{GenerateError, InitError, NodeError, TypedNode, typedflake};

#[derive(Debug, Clone, Copy, PartialEq, Eq, TypedNode)]
pub struct AppNode {
    #[node(bits = 5)]
    pub worker: u8,
    #[node(bits = 5)]
    pub process: u8,
}

/// Same width as `AppNode`, but a different schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, TypedNode)]
pub struct RegionNode {
    #[node(bits = 3)]
    pub region: u8,
    #[node(bits = 7)]
    pub instance: u8,
}

#[typedflake(epoch = "2025-01-01", node = AppNode)]
pub struct UserId(i64);

#[typedflake(epoch = "2025-01-01", node = AppNode)]
pub struct OrderId(i64);

#[typedflake(epoch = "2025-01-01", node = RegionNode)]
pub struct RegionId(i64);

#[typedflake(epoch = "2025-01-01")]
pub struct PlainId(i64);

#[test]
fn typed_node_default_lifecycle() {
    // An invalid typed node is rejected and installs nothing.
    assert_eq!(
        typedflake::init(AppNode {
            worker: 17,
            process: 40,
        }),
        Err(InitError::InvalidNode(NodeError::new("process", 40, 31)))
    );
    assert!(matches!(
        UserId::generate(),
        Err(GenerateError::NotInitialized)
    ));

    let node = AppNode {
        worker: 17,
        process: 1,
    };
    assert_eq!(typedflake::init(node), Ok(()));
    assert_eq!(typedflake::init(node), Err(InitError::AlreadyInitialized));

    assert_eq!(UserId::generate().unwrap().parts().node, node);
    assert_eq!(OrderId::generate().unwrap().parts().node, node);

    // Another schema of the same width, and a plain node, are both mismatches.
    assert!(matches!(
        RegionId::generate(),
        Err(GenerateError::NodeSchemaMismatch { .. })
    ));
    assert!(matches!(
        PlainId::generate(),
        Err(GenerateError::NodeSchemaMismatch {
            expected: "u32",
            ..
        })
    ));

    // Those IDs remain usable through explicit generators.
    let region = RegionNode {
        region: 2,
        instance: 100,
    };
    assert_eq!(
        RegionId::generator(region)
            .unwrap()
            .generate()
            .unwrap()
            .parts()
            .node,
        region
    );
    assert_eq!(
        PlainId::generator(9)
            .unwrap()
            .generate()
            .unwrap()
            .parts()
            .node,
        9
    );
}
