//! Serde support: decimal strings out and strings or integers in, or the
//! encoded text both ways.

use core::fmt;
use core::marker::PhantomData;

use ::serde::de::{Error, Visitor};
pub use ::serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::encoding::{DecodeIdError, EncodedId};
use crate::id::{self, Id, ParseIdError};

/// Serializes an ID as a decimal string, so it survives formats whose numbers
/// are IEEE 754 doubles.
pub fn serialize<I: Id, S: Serializer>(id: &I, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_str(&id.get())
}

/// Deserializes an ID through the same validation as `TryFrom` and `FromStr`.
pub fn deserialize<'de, I: Id, D: Deserializer<'de>>(deserializer: D) -> Result<I, D::Error> {
    // Formats that are not self-describing cannot answer `deserialize_any`,
    // and they always carry the string this crate serialized.
    if deserializer.is_human_readable() {
        deserializer.deserialize_any(IdVisitor(PhantomData))
    } else {
        deserializer.deserialize_str(IdVisitor(PhantomData))
    }
}

struct IdVisitor<I>(PhantomData<I>);

impl<I: Id> Visitor<'_> for IdVisitor<I> {
    type Value = I;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an ID as a decimal string or an integer")
    }

    fn visit_str<E: Error>(self, value: &str) -> Result<I, E> {
        // Serde errors carry no source chain, so report the underlying cause.
        id::parse(value).map_err(|error| match error {
            ParseIdError::Int(error) => E::custom(error),
            ParseIdError::Invalid(error) => E::custom(error),
        })
    }

    fn visit_u64<E: Error>(self, value: u64) -> Result<I, E> {
        id::from_u64(value).map_err(E::custom)
    }

    fn visit_i64<E: Error>(self, value: i64) -> Result<I, E> {
        id::from_i64(value).map_err(E::custom)
    }
}

/// Serializes an ID as its encoded text.
pub fn serialize_encoded<I: EncodedId, S: Serializer>(
    id: &I,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(id.encode().as_str())
}

/// Deserializes an ID from its encoded text only. A decimal string can also
/// be valid text in the alphabet, for a different ID, so accepting both would
/// be ambiguous.
pub fn deserialize_encoded<'de, I: EncodedId, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<I, D::Error> {
    deserializer.deserialize_str(EncodedVisitor(PhantomData))
}

struct EncodedVisitor<I>(PhantomData<I>);

impl<I: EncodedId> Visitor<'_> for EncodedVisitor<I> {
    type Value = I;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an ID encoded in its alphabet")
    }

    fn visit_str<E: Error>(self, value: &str) -> Result<I, E> {
        I::decode(value).map_err(|error| match error {
            DecodeIdError::Invalid(error) => E::custom(error),
            error => E::custom(error),
        })
    }
}
