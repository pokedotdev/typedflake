//! SQLx PostgreSQL support for `i64` IDs, stored as `BIGINT`.

pub use ::sqlx::encode::IsNull;
pub use ::sqlx::error::BoxDynError;
pub use ::sqlx::postgres::{PgArgumentBuffer, PgHasArrayType, PgTypeInfo, PgValueRef, Postgres};
pub use ::sqlx::{Decode, Encode, Type};

use crate::id::{self, Id};

pub fn type_info() -> PgTypeInfo {
    <i64 as Type<Postgres>>::type_info()
}

pub fn compatible(ty: &PgTypeInfo) -> bool {
    <i64 as Type<Postgres>>::compatible(ty)
}

pub fn array_type_info() -> PgTypeInfo {
    <i64 as PgHasArrayType>::array_type_info()
}

pub fn encode<I: Id<Repr = i64>>(
    id: &I,
    buffer: &mut PgArgumentBuffer,
) -> Result<IsNull, BoxDynError> {
    <i64 as Encode<'_, Postgres>>::encode_by_ref(&id.get(), buffer)
}

/// Decodes through ID validation, so a negative or out-of-range `BIGINT` is an
/// error instead of an invalid ID.
pub fn decode<I: Id<Repr = i64>>(value: PgValueRef<'_>) -> Result<I, BoxDynError> {
    let raw = <i64 as Decode<'_, Postgres>>::decode(value)?;
    Ok(id::from_i64(raw)?)
}
