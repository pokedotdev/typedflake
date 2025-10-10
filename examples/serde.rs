//! Example demonstrating JSON serialization with serde
//!
//! Run with: cargo run --example serde --features serde

use serde::{Deserialize, Serialize};

typedflake::id!(UserId);

#[derive(Serialize, Deserialize, Debug)]
struct User {
    id: UserId,
    name: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a user with generated ID
    let user = User {
        id: UserId::generate(),
        name: "Alice".to_string(),
    };

    println!("User: {user:?}");

    // Serialize to JSON (ID will be a string, safe for JavaScript)
    let json = serde_json::to_string_pretty(&user)?;
    println!("\nJSON:\n{json}");

    // Deserialize back
    let deserialized: User = serde_json::from_str(&json)?;
    println!("\nDeserialized: {deserialized:?}");

    // Verify roundtrip
    assert_eq!(user.id, deserialized.id);
    println!("\n✅ Roundtrip successful!");

    Ok(())
}
