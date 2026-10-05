#![cfg(feature = "serde")]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use typedflake::{Alphabet, typedflake};

#[typedflake(epoch = "2025-01-01")]
#[derive(typedflake::Serde)]
pub struct UserId(i64);

#[typedflake(epoch = "2025-01-01")]
#[derive(typedflake::Serde)]
pub struct WideId(u64);

#[typedflake(epoch = "2025-01-01", bits(timestamp = 32, node = 5, sequence = 8))]
#[derive(typedflake::Serde)]
pub struct ReducedId(i64);

#[typedflake(epoch = "2025-01-01", alphabet = Alphabet::BASE62)]
#[derive(typedflake::SerdeEncoded)]
pub struct LinkId(i64);

#[typedflake(
    epoch = "2025-01-01",
    bits(timestamp = 32, node = 5, sequence = 8),
    alphabet = Alphabet::BASE36,
)]
#[derive(typedflake::SerdeEncoded)]
pub struct ReducedLinkId(i64);

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct User {
    id: UserId,
    name: String,
    referrer: Option<UserId>,
}

fn user_id(raw: i64) -> UserId {
    UserId::try_from(raw).unwrap()
}

#[test]
fn ids_serialize_as_decimal_strings() {
    // Larger than 2^53, so a JSON number would lose precision.
    let id = user_id(9_007_199_254_740_993);
    assert_eq!(serde_json::to_string(&id).unwrap(), r#""9007199254740993""#);
    assert_eq!(
        serde_json::to_string(&WideId::try_from(u64::MAX).unwrap()).unwrap(),
        r#""18446744073709551615""#
    );
}

#[test]
fn ids_deserialize_from_strings_and_integers() {
    let from_string: UserId = serde_json::from_str(r#""123456789""#).unwrap();
    let from_integer: UserId = serde_json::from_str("123456789").unwrap();
    assert_eq!(from_string, from_integer);
    assert_eq!(from_string.get(), 123_456_789);

    let wide: WideId = serde_json::from_str("18446744073709551615").unwrap();
    assert_eq!(wide.get(), u64::MAX);
}

#[test]
fn deserialization_applies_id_validation() {
    let error = |json: &str| {
        serde_json::from_str::<UserId>(json)
            .unwrap_err()
            .to_string()
    };

    assert!(error("-5").contains("ID -5 is negative"), "{}", error("-5"));
    assert!(error(r#""-5""#).contains("ID -5 is negative"));
    assert!(error("9223372036854775808").contains("sets reserved bits"));
    assert!(error(r#""abc""#).contains("invalid digit"));
    assert!(error("1.5").contains("an ID as a decimal string or an integer"));
    assert!(error("null").contains("an ID as a decimal string or an integer"));
    assert!(error("true").contains("an ID as a decimal string or an integer"));

    let reserved = (1_u64 << 45).to_string();
    assert!(serde_json::from_str::<ReducedId>(&reserved).is_err());
    assert!(serde_json::from_str::<ReducedId>(&format!("\"{reserved}\"")).is_err());
    assert!(serde_json::from_str::<ReducedId>(&((1_u64 << 45) - 1).to_string()).is_ok());
    assert!(serde_json::from_str::<WideId>("-1").is_err());
}

#[test]
fn ids_round_trip_inside_structs_and_as_map_keys() {
    let user = User {
        id: user_id(42),
        name: "Alice".to_owned(),
        referrer: Some(user_id(7)),
    };
    let json = serde_json::to_string(&user).unwrap();
    assert_eq!(json, r#"{"id":"42","name":"Alice","referrer":"7"}"#);
    assert_eq!(serde_json::from_str::<User>(&json).unwrap(), user);

    let lenient: User =
        serde_json::from_str(r#"{"id":42,"name":"Alice","referrer":null}"#).unwrap();
    assert_eq!(lenient.id, user.id);
    assert_eq!(lenient.referrer, None);

    let scores: BTreeMap<UserId, u32> = [(user_id(1), 10), (user_id(2), 20)].into();
    let json = serde_json::to_string(&scores).unwrap();
    assert_eq!(json, r#"{"1":10,"2":20}"#);
    assert_eq!(
        serde_json::from_str::<BTreeMap<UserId, u32>>(&json).unwrap(),
        scores
    );
}

#[test]
fn binary_formats_keep_the_string_representation() {
    let id = user_id(9_007_199_254_740_993);
    let bytes = bincode::serialize(&id).unwrap();
    assert_eq!(bytes, bincode::serialize("9007199254740993").unwrap());
    assert_eq!(bincode::deserialize::<UserId>(&bytes).unwrap(), id);

    let negative = bincode::serialize("-5").unwrap();
    assert!(bincode::deserialize::<UserId>(&negative).is_err());
}

#[test]
fn deserialization_needs_no_initialization() {
    // This process never calls `typedflake::init`.
    assert!(serde_json::from_str::<UserId>("1").is_ok());
    assert!(matches!(
        UserId::generate(),
        Err(typedflake::GenerateError::NotInitialized { .. })
    ));
}

#[test]
fn encoded_ids_serialize_as_their_encoded_text() {
    let id = LinkId::try_from(232_900_560_974_681_078_i64).unwrap();
    assert_eq!(serde_json::to_string(&id).unwrap(), r#""0HCgayuFvMs""#);
    assert_eq!(
        serde_json::from_str::<LinkId>(r#""0HCgayuFvMs""#).unwrap(),
        id
    );

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Link {
        id: LinkId,
        previous: Option<LinkId>,
        by_id: BTreeMap<LinkId, u8>,
    }
    let link = Link {
        id,
        previous: None,
        by_id: BTreeMap::from([(id, 1)]),
    };
    let json = serde_json::to_string(&link).unwrap();
    assert_eq!(
        json,
        r#"{"id":"0HCgayuFvMs","previous":null,"by_id":{"0HCgayuFvMs":1}}"#
    );
    assert_eq!(serde_json::from_str::<Link>(&json).unwrap(), link);

    let bytes = bincode::serialize(&id).unwrap();
    assert_eq!(bincode::deserialize::<LinkId>(&bytes).unwrap(), id);
}

#[test]
fn encoded_ids_deserialize_only_from_their_encoded_text() {
    // An integer, and a decimal string of another length.
    assert!(serde_json::from_str::<LinkId>("232900560974681078").is_err());
    let error = serde_json::from_str::<LinkId>(r#""232900560974681078""#).unwrap_err();
    assert!(error.to_string().contains("this ID type uses 11"));

    // Eleven digits are valid text in the alphabet: they decode as base 62,
    // not as the decimal number.
    let id: LinkId = serde_json::from_str(r#""00000000010""#).unwrap();
    assert_eq!(id.get(), 62);

    let error = serde_json::from_str::<LinkId>(r#""0HCga-uFvMs""#).unwrap_err();
    assert!(error.to_string().contains("not in the ID's alphabet"));

    // Reserved bits of a reduced format are reported by their cause.
    let error = serde_json::from_str::<ReducedLinkId>(r#""zzzzzzzzz""#).unwrap_err();
    assert!(error.to_string().contains("reserved bits"));
}
