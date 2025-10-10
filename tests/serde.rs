#![cfg(feature = "serde")]

use serde::{Deserialize, Serialize};

typedflake::id!(UserId);
typedflake::id!(OrderId);

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Order {
    user_id: UserId,
    order_id: OrderId,
    amount: u64,
}

#[test]
fn serialize_as_string() {
    let id = UserId::generate();
    let json = serde_json::to_string(&id).unwrap();

    // Should be serialized as a quoted string, not a number
    assert!(json.starts_with('"'));
    assert!(json.ends_with('"'));
    assert_eq!(json, format!("\"{id}\""));
}

#[test]
fn deserialize_from_string() {
    let id = UserId::generate();
    let json = format!("\"{id}\"");

    let deserialized: UserId = serde_json::from_str(&json).unwrap();
    assert_eq!(id, deserialized);
}

#[test]
fn roundtrip() {
    let original = UserId::generate();
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: UserId = serde_json::from_str(&json).unwrap();

    assert_eq!(original, deserialized);
}

#[test]
fn serialize_in_struct() {
    let order = Order {
        user_id: UserId::generate(),
        order_id: OrderId::generate(),
        amount: 1000,
    };

    let json = serde_json::to_string(&order).unwrap();

    // IDs should be strings in JSON
    assert!(json.contains("\"user_id\":\""));
    assert!(json.contains("\"order_id\":\""));
    assert!(json.contains("\"amount\":1000"));

    // Deserialize back
    let deserialized: Order = serde_json::from_str(&json).unwrap();
    assert_eq!(order, deserialized);
}

#[test]
fn deserialize_validates() {
    // Invalid format should fail
    let invalid_json = "\"not_a_number\"";
    let result: Result<UserId, _> = serde_json::from_str(invalid_json);
    assert!(result.is_err());
}
