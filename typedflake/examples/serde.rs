//! Serialize IDs with Serde.
//!
//! Run with `cargo run -p typedflake --example serde --features serde`.

use serde::{Deserialize, Serialize};
use typedflake::typedflake;

#[typedflake(epoch = "2025-01-01")]
#[derive(typedflake::Serde)]
pub struct UserId(i64);

#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: UserId,
    name: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    typedflake::init(17)?;

    let user = User {
        id: UserId::generate()?,
        name: "Alice".to_owned(),
    };

    // IDs are written as strings: JSON numbers lose precision above 2^53.
    let json = serde_json::to_string(&user)?;
    println!("{json}");

    // Strings and integers are both accepted, and both are validated.
    let parsed: User = serde_json::from_str(&json)?;
    assert_eq!(parsed.id, user.id);
    let from_number: User = serde_json::from_str(r#"{"id":42,"name":"Bob"}"#)?;
    println!("{from_number:?}");
    println!(
        "negative: {}",
        serde_json::from_str::<UserId>("-1").unwrap_err()
    );

    Ok(())
}
