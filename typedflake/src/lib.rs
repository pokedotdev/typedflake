//! Snowflake-style IDs as distinct, validated newtypes.
//!
//! Declare an ID type with [`#[typedflake]`](macro@typedflake), install a node
//! once with [`init`], and generate IDs anywhere:
//!
//! ```
//! use typedflake::typedflake;
//!
//! #[typedflake(epoch = "2025-01-01")]
//! pub struct UserId(i64);
//!
//! #[typedflake(epoch = "2025-01-01")]
//! pub struct OrderId(i64);
//!
//! // Once, during application startup.
//! typedflake::init(17)?;
//!
//! // Anywhere in the application.
//! let user_id = UserId::generate()?;
//! let order_id = OrderId::generate()?;
//!
//! assert_eq!(user_id.parts().node, 17);
//! assert_eq!(order_id.to_string().parse::<OrderId>()?, order_id);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! `UserId` and `OrderId` are separate types: they cannot be mixed up, and each
//! keeps its own generation state.
//!
//! # Uniqueness
//!
//! IDs of one type are unique as long as no two running processes use the same
//! node for it, and the system clock does not move backwards across a restart.
//! This crate does not assign nodes or coordinate deployments.
//!
//! # Features
//!
//! | Feature | Enables |
//! | --- | --- |
//! | `serde` | [`Serde`](macro@Serde) and [`SerdeEncoded`](macro@SerdeEncoded) derives |
//! | `sqlx-postgres` | [`SqlxPostgres`](macro@SqlxPostgres) derive |
//! | `postgres` | [`Postgres`](macro@Postgres) derive for `postgres-types` |
//! | `tokio` | `generate_async` methods |

#![forbid(unsafe_code)]
#![warn(missing_docs)]

#[cfg(not(target_has_atomic = "64"))]
compile_error!("typedflake requires a target with 64-bit atomics");

// Generated code refers to `::typedflake`, including inside this crate.
extern crate self as typedflake;

mod clock;
mod encoding;
mod format;
mod generator;
mod global;
mod id;
mod integrations;
mod node;
mod state;

pub use encoding::{Alphabet, DecodeIdError, Encoded, EncodedId};
pub use format::{BitLayout, Epoch, Format, FormatError};
pub use generator::{GenerateError, Generator, GeneratorError};
pub use global::{InitError, init};
pub use id::{Id, InvalidId, ParseIdError, Parts, Repr, TimestampError};
pub use node::{Node, NodeError};
pub use typedflake_macros::{Postgres, Serde, SerdeEncoded, SqlxPostgres, TypedNode, typedflake};

/// Support for generated code. Not part of the public interface.
#[doc(hidden)]
pub mod __private {
    pub use std::sync::OnceLock;

    pub use crate::format::{Layout, default_bits, format_error_message};
    pub use crate::id::{from_i64, from_u64, parse};
    pub use crate::node::pack_field;

    #[cfg(feature = "postgres")]
    pub use crate::integrations::postgres;
    #[cfg(feature = "serde")]
    pub use crate::integrations::serde;
    #[cfg(feature = "sqlx-postgres")]
    pub use crate::integrations::sqlx_postgres;
}

/// Compiles the README's examples as doctests. They use every integration, so
/// they only run with all features enabled.
#[cfg(all(
    doctest,
    feature = "serde",
    feature = "sqlx-postgres",
    feature = "postgres",
    feature = "tokio",
))]
#[doc = include_str!("../../README.md")]
struct ReadmeDoctests;

#[cfg(doctest)]
#[doc = include_str!("../../MIGRATION.md")]
struct MigrationDoctests;

/// Rejections whose compiler output depends on whether the standard library's
/// sources are installed, so they are checked here instead of in `tests/ui`.
///
/// IDs have no arithmetic:
///
/// ```compile_fail
/// use typedflake::typedflake;
///
/// #[typedflake(epoch = "2025-01-01")]
/// struct UserId(i64);
///
/// let id = UserId::try_from(1_i64).unwrap();
/// let next = id + 1;
/// ```
///
/// A date that does not exist fails a const epoch:
///
/// ```compile_fail
/// const EPOCH: typedflake::Epoch = typedflake::Epoch::from_date(2025, 2, 30);
/// ```
///
/// An alphabet with a repeated character fails at the declaration that uses
/// it, even if nothing is ever encoded:
///
/// ```compile_fail
/// use typedflake::{Alphabet, typedflake};
///
/// #[typedflake(epoch = "2025-01-01", alphabet = Alphabet::new("abca"))]
/// struct LinkId(i64);
/// ```
///
/// The same declarations compile once corrected:
///
/// ```
/// use typedflake::{Alphabet, typedflake};
///
/// #[typedflake(epoch = "2025-01-01")]
/// struct UserId(i64);
///
/// let id = UserId::try_from(1_i64).unwrap();
/// let next = id.get() + 1;
/// const EPOCH: typedflake::Epoch = typedflake::Epoch::from_date(2025, 2, 28);
///
/// #[typedflake(epoch = "2025-01-01", alphabet = Alphabet::new("abc"))]
/// struct LinkId(i64);
/// # let _ = (next, EPOCH);
/// ```
#[cfg(doctest)]
struct CompileFailDoctests;
