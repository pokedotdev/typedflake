//! Serde support: decimal strings out, strings or integers in.

use core::fmt;
use core::marker::PhantomData;

use ::serde::de::{Error, Visitor};
pub use ::serde::{Deserialize, Deserializer, Serialize, Serializer};

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
