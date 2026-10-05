//! Encoding IDs in an alphabet. Nothing here needs `init`.

use proptest::prelude::*;
use typedflake::{Alphabet, DecodeIdError, EncodedId, InvalidId, typedflake};

pub const FRIENDLY: Alphabet = Alphabet::new("23456789abcdefghjkmnpqrstuvwxyz");

#[typedflake(epoch = "2025-01-01", alphabet = Alphabet::BASE62)]
pub struct UserId(i64);

#[typedflake(epoch = "2025-01-01", alphabet = Alphabet::BASE62)]
pub struct UnsignedId(u64);

#[typedflake(epoch = "2025-01-01", alphabet = Alphabet::BASE64_URL)]
pub struct UrlId(i64);

#[typedflake(epoch = "2025-01-01", alphabet = FRIENDLY)]
pub struct InviteId(i64);

#[typedflake(
    epoch = "2025-01-01",
    bits(timestamp = 32, node = 5, sequence = 8),
    alphabet = Alphabet::BASE36,
)]
pub struct ReducedId(i64);

#[typedflake(epoch = "2025-01-01", bits(timestamp = 1, node = 0, sequence = 1), alphabet = Alphabet::BASE58)]
pub struct MinimalId(i64);

const RAW: i64 = 232_900_560_974_681_078;

#[test]
fn ids_encode_in_their_alphabet_and_decode_back() {
    let id = UserId::try_from(RAW).unwrap();
    let text = id.encode();

    assert_eq!(text.as_str(), "0HCgayuFvMs");
    assert_eq!(text.to_string(), "0HCgayuFvMs");
    assert_eq!(&*text, "0HCgayuFvMs");
    assert_eq!(format!("{text:?}"), "\"0HCgayuFvMs\"");
    assert_eq!(UserId::decode(&text).unwrap(), id);
    assert_eq!(UserId::decode("0HCgayuFvMs").unwrap(), id);

    // The decimal form is unchanged.
    assert_eq!(id.to_string(), RAW.to_string());
}

#[test]
fn every_id_of_a_type_has_the_same_length() {
    assert_eq!(
        UserId::try_from(0_i64).unwrap().encode().as_str(),
        "00000000000"
    );
    assert_eq!(
        UserId::try_from(i64::MAX).unwrap().encode().as_str(),
        "AzL8n0Y58m7"
    );
    assert_eq!(
        UnsignedId::try_from(u64::MAX).unwrap().encode().as_str(),
        "LygHa16AHYF"
    );

    // 45 bits in base 36, and 2 bits in base 58.
    assert_eq!(
        ReducedId::try_from(1_i64).unwrap().encode().as_str(),
        "000000001"
    );
    assert_eq!(MinimalId::try_from(3_i64).unwrap().encode().as_str(), "4");

    assert_eq!(InviteId::try_from(RAW).unwrap().encode().len(), 13);
}

#[test]
fn the_url_alphabet_pads_with_a_hyphen() {
    assert_eq!(
        UrlId::try_from(0_i64).unwrap().encode().as_str(),
        "-----------"
    );
    let id = UrlId::try_from(RAW).unwrap();
    assert!(id.encode().starts_with('-'));
    assert_eq!(UrlId::decode(&id.encode()).unwrap(), id);
}

#[test]
fn decoding_accepts_only_the_exact_encoding() {
    assert_eq!(
        UserId::decode("HCPqSdBRrg"),
        Err(DecodeIdError::Length {
            expected: 11,
            found: 10
        })
    );
    assert_eq!(
        UserId::decode("00HCgayuFvMs"),
        Err(DecodeIdError::Length {
            expected: 11,
            found: 12
        })
    );
    assert_eq!(
        UserId::decode("0HCga-uFvMs"),
        Err(DecodeIdError::Character { index: 5 })
    );
    assert_eq!(UserId::decode("zzzzzzzzzzz"), Err(DecodeIdError::Overflow));
    assert_eq!(
        UserId::decode(""),
        Err(DecodeIdError::Length {
            expected: 11,
            found: 0
        })
    );

    // A character of another alphabet.
    assert_eq!(
        InviteId::decode("2b76swcw35eg0"),
        Err(DecodeIdError::Character { index: 12 })
    );
}

#[test]
fn decoded_integers_are_validated_against_the_format() {
    // One above `i64::MAX`: fits 64 bits, but not a signed ID.
    let too_large = UnsignedId::try_from(i64::MAX as u64 + 1).unwrap().encode();
    assert!(matches!(
        UserId::decode(&too_large),
        Err(DecodeIdError::Invalid(InvalidId::ReservedBits { .. }))
    ));

    // Reserved bits of a reduced format.
    assert!(matches!(
        ReducedId::decode("zzzzzzzzz"),
        Err(DecodeIdError::Invalid(InvalidId::ReservedBits { .. }))
    ));

    let error = ReducedId::decode("zzzzzzzzz").unwrap_err();
    assert_eq!(error.to_string(), "decoded integer is not a valid ID");
    assert!(std::error::Error::source(&error).is_some());
}

#[test]
fn generic_code_reaches_encoding_through_the_trait() {
    fn round_trip<I: EncodedId + PartialEq>(id: I) -> bool {
        I::decode(&id.encode()) == Ok(id)
    }

    assert_eq!(UserId::ALPHABET, Alphabet::BASE62);
    assert_eq!(
        InviteId::ALPHABET.as_str(),
        "23456789abcdefghjkmnpqrstuvwxyz"
    );
    assert!(round_trip(UserId::try_from(RAW).unwrap()));
    assert!(round_trip(InviteId::try_from(RAW).unwrap()));
}

proptest! {
    #[test]
    fn valid_ids_round_trip(raw in 0..=i64::MAX, wide in any::<u64>()) {
        let id = UserId::try_from(raw).unwrap();
        prop_assert_eq!(UserId::decode(&id.encode()), Ok(id));
        let id = UrlId::try_from(raw).unwrap();
        prop_assert_eq!(UrlId::decode(&id.encode()), Ok(id));
        let id = InviteId::try_from(raw).unwrap();
        prop_assert_eq!(InviteId::decode(&id.encode()), Ok(id));
        let id = UnsignedId::try_from(wide).unwrap();
        prop_assert_eq!(UnsignedId::decode(&id.encode()), Ok(id));
    }

    #[test]
    fn encodings_sort_like_their_ids(a in 0..=i64::MAX, b in 0..=i64::MAX) {
        let (first, second) = (UserId::try_from(a).unwrap(), UserId::try_from(b).unwrap());
        prop_assert_eq!(first.cmp(&second), first.encode().as_str().cmp(second.encode().as_str()));

        let (first, second) = (UrlId::try_from(a).unwrap(), UrlId::try_from(b).unwrap());
        prop_assert_eq!(first.cmp(&second), first.encode().as_str().cmp(second.encode().as_str()));
    }

    #[test]
    fn arbitrary_text_never_panics(text in "\\PC{0,16}") {
        let _ = UserId::decode(&text);
        let _ = UrlId::decode(&text);
    }
}
