//! `postgres-types` support for `i64` IDs, stored as `BIGINT`.
//!
//! Works with both `tokio-postgres` and the synchronous `postgres` client.

pub use ::postgres_types::private::BytesMut;
pub use ::postgres_types::{FromSql, IsNull, ToSql, Type, to_sql_checked};

use crate::id::{self, Id};

pub type BoxError = Box<dyn std::error::Error + Sync + Send>;

pub fn accepts(ty: &Type) -> bool {
    <i64 as ToSql>::accepts(ty)
}

pub fn to_sql<I: Id<Repr = i64>>(
    id: &I,
    ty: &Type,
    out: &mut BytesMut,
) -> Result<IsNull, BoxError> {
    id.get().to_sql(ty, out)
}

/// Decodes through ID validation, so a negative or out-of-range `BIGINT` is an
/// error instead of an invalid ID.
pub fn from_sql<I: Id<Repr = i64>>(ty: &Type, raw: &[u8]) -> Result<I, BoxError> {
    let raw = <i64 as FromSql>::from_sql(ty, raw)?;
    Ok(id::from_i64(raw)?)
}
