//! Share one format between ID types and split the node into named fields.
//!
//! Run with `cargo run -p typedflake --example typed_node`.

use typedflake::{BitLayout, Epoch, Format, TypedNode, typedflake};

pub const APP_IDS: Format = Format {
    epoch: Epoch::from_date(2025, 1, 1),
    bits: BitLayout {
        timestamp: 41,
        node: 10,
        sequence: 12,
    },
};

/// Packs as `[worker: 5][process: 5]`, which must add up to the format's 10
/// node bits. The field order is part of the persisted format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, TypedNode)]
pub struct AppNode {
    #[node(bits = 5)]
    pub worker: u8,
    #[node(bits = 5)]
    pub process: u8,
}

#[typedflake(format = APP_IDS, node = AppNode)]
pub struct UserId(i64);

#[typedflake(format = APP_IDS, node = AppNode)]
pub struct OrderId(i64);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let node = AppNode {
        worker: 17,
        process: 1,
    };
    typedflake::init(node)?;

    let user_id = UserId::generate()?;
    let parts = user_id.parts();
    println!(
        "{user_id}: worker {}, process {}, sequence {}",
        parts.node.worker, parts.node.process, parts.sequence
    );

    // An explicit generator works without `init`, for a node of your choice.
    let other = AppNode {
        worker: 3,
        process: 0,
    };
    let generator = OrderId::generator(other)?;
    let order_id = generator.generate()?;
    assert_eq!(order_id.parts().node, other);

    // Node fields are validated when used, never truncated.
    let invalid = AppNode {
        worker: 40,
        process: 0,
    };
    println!("worker 40: {:?}", OrderId::generator(invalid).unwrap_err());

    Ok(())
}
