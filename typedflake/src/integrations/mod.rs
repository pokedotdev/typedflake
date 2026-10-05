//! Shared implementations behind the opt-in integration derives.

#![allow(missing_docs)]

#[cfg(feature = "postgres")]
pub mod postgres;
#[cfg(feature = "serde")]
pub mod serde;
#[cfg(feature = "sqlx-postgres")]
pub mod sqlx_postgres;
