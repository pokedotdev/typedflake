//! Global initialization with several node types in one process. Defaults are
//! process-wide, so this file is one ordered test in its own process.

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
fn each_node_type_has_its_own_default() {
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
        Err(GenerateError::NotInitialized { .. })
    ));

    let app_node = AppNode {
        worker: 17,
        process: 1,
    };
    assert_eq!(typedflake::init(app_node), Ok(()));
    assert_eq!(
        typedflake::init(app_node),
        Err(InitError::AlreadyInitialized)
    );

    // Only IDs with that node type are ready.
    assert_eq!(UserId::generate().unwrap().parts().node, app_node);
    assert_eq!(OrderId::generate().unwrap().parts().node, app_node);
    assert!(matches!(
        PlainId::generate(),
        Err(GenerateError::NotInitialized { node: "u32" })
    ));
    assert!(matches!(
        RegionId::generate(),
        Err(GenerateError::NotInitialized { node }) if node.ends_with("RegionNode")
    ));

    // Installing the other node types makes their IDs ready, each with its own
    // node, without disturbing the first.
    let region_node = RegionNode {
        region: 2,
        instance: 100,
    };
    assert_eq!(typedflake::init(9), Ok(()));
    assert_eq!(typedflake::init(region_node), Ok(()));

    assert_eq!(PlainId::generate().unwrap().parts().node, 9);
    assert_eq!(RegionId::generate().unwrap().parts().node, region_node);
    assert_eq!(UserId::generate().unwrap().parts().node, app_node);

    // Every type is still installed only once.
    assert_eq!(typedflake::init(10), Err(InitError::AlreadyInitialized));
    assert_eq!(
        typedflake::init(region_node),
        Err(InitError::AlreadyInitialized)
    );
    assert_eq!(PlainId::generate().unwrap().parts().node, 9);
}
