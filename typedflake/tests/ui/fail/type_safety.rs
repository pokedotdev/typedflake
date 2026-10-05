use typedflake::{TypedNode, typedflake};

#[derive(Debug, Clone, Copy, TypedNode)]
struct AppNode {
    #[node(bits = 5)]
    worker: u8,
    #[node(bits = 5)]
    process: u8,
}

mod ids {
    use super::*;

    #[typedflake(epoch = "2025-01-01")]
    pub struct UserId(i64);

    #[typedflake(epoch = "2025-01-01")]
    pub struct OrderId(i64);

    #[typedflake(epoch = "2025-01-01", node = AppNode)]
    pub struct TypedId(i64);
}

use ids::*;

fn takes_user(_: UserId) {}

fn main() {
    let order = OrderId::try_from(1_i64).unwrap();

    // Identity types do not mix.
    takes_user(order);
    let _ = order == UserId::try_from(1_i64).unwrap();

    // No unchecked construction or implicit conversion from integers.
    let _ = UserId(1);
    let _: UserId = 1_i64.into();

    // No arithmetic or dereferencing.
    let _ = order + 1;
    let _ = *order;

    // A typed-node ID takes its declared node, not a packed integer.
    let _ = TypedId::generator(17);
    let _ = UserId::generator(AppNode {
        worker: 1,
        process: 1,
    });
}
