//! Declare ID types, install a node once, and generate IDs anywhere.
//!
//! Run with `cargo run -p typedflake --example demo`.

use typedflake::typedflake;

#[typedflake(epoch = "2025-01-01")]
pub struct UserId(i64);

#[typedflake(epoch = "2025-01-01")]
pub struct OrderId(i64);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Once, during application startup. The node must be unique among the
    // running processes that generate the same ID types.
    typedflake::init(17)?;

    let user_id = UserId::generate()?;
    let order_id = OrderId::generate()?;
    println!("user  {user_id} ({user_id:?})");
    println!("order {order_id} ({order_id:?})");

    // IDs convert to and from their integer and decimal string.
    let raw: i64 = user_id.get();
    assert_eq!(UserId::try_from(raw)?, user_id);
    assert_eq!(user_id.to_string().parse::<UserId>()?, user_id);

    // Invalid values are rejected instead of becoming IDs.
    println!("negative input: {}", UserId::try_from(-1_i64).unwrap_err());

    // Every ID can be taken apart, without any global state.
    let parts = user_id.parts();
    println!(
        "elapsed {} ms, node {}, sequence {}",
        parts.elapsed_millis, parts.node, parts.sequence
    );
    println!(
        "created at {} ms since the Unix epoch",
        user_id.unix_millis()?
    );
    assert_eq!(UserId::from_parts(parts)?, user_id);

    Ok(())
}
