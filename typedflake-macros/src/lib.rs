//! Procedural macros for [TypedFlake](https://docs.rs/typedflake).
//!
//! Use them through the `typedflake` crate, which re-exports and documents
//! them.

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

mod id;
mod integrations;
mod node;
mod util;

/// Declares a Snowflake-style ID type.
///
/// Apply it to a tuple struct with one private `i64` or `u64` field, above any
/// `#[derive]`:
///
/// ```ignore
/// #[typedflake(epoch = "2025-01-01")]
/// pub struct UserId(i64);
/// ```
///
/// # Options
///
/// - `epoch = "YYYY-MM-DD"`: instant timestamps are measured from, at midnight
///   UTC. Required unless `format` is given.
/// - `bits(timestamp = N, node = N, sequence = N)`: bit allocation. Defaults to
///   10 node bits, 12 sequence bits, and the remaining bits for the timestamp
///   (41 for `i64`, 42 for `u64`). The widths may add up to less than the
///   integer's capacity.
/// - `format = CONSTANT`: a shared `Format` constant, instead of `epoch` and
///   `bits`.
/// - `node = Type`: a `TypedNode` struct that splits the node field into named
///   parts. Defaults to a plain `u32` node number.
/// - `alphabet = CONSTANT`: an `Alphabet` for a compact text form. Adds
///   `encode` and `decode`, and implements `EncodedId`.
///
/// # Generated items
///
/// The type implements `Id`, `Debug`, `Display`, `FromStr`, `Clone`, `Copy`,
/// `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash`, `TryFrom<i64>`,
/// `TryFrom<u64>`, and `From<Self>` for its integer, and is
/// `#[repr(transparent)]`. Every `Id` method is also available as an inherent
/// method, so no trait import is needed.
#[proc_macro_attribute]
pub fn typedflake(args: TokenStream, input: TokenStream) -> TokenStream {
    id::expand(args.into(), input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Splits the node field of an ID into named parts.
///
/// Apply it to a struct with named `u8`, `u16`, or `u32` fields, each marked
/// with `#[node(bits = N)]`. Fields pack in declaration order, most
/// significant first, and their widths must add up to the node bits of the ID
/// format.
///
/// ```ignore
/// #[derive(Debug, Clone, Copy, TypedNode)]
/// pub struct AppNode {
///     #[node(bits = 5)]
///     pub worker: u8,
///     #[node(bits = 5)]
///     pub process: u8,
/// }
///
/// #[typedflake(epoch = "2025-01-01", node = AppNode)]
/// pub struct UserId(i64);
/// ```
///
/// The field order is part of the persisted ID format.
#[proc_macro_derive(TypedNode, attributes(node))]
pub fn derive_typed_node(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    node::expand(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Implements Serde's `Serialize` and `Deserialize` for an ID type.
///
/// IDs serialize as decimal strings and deserialize from strings or integers,
/// with the same validation as `TryFrom`. Requires the `serde` feature.
#[proc_macro_derive(Serde)]
pub fn derive_serde(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    integrations::serde(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Implements Serde's `Serialize` and `Deserialize` for an ID type, using its
/// encoded text.
///
/// IDs serialize as `encode()` writes them and deserialize only from that
/// exact text, with the same validation as `decode()`. The ID type needs the
/// `alphabet` option. Use this or `Serde` on a type, not both. Requires the
/// `serde` feature.
#[proc_macro_derive(SerdeEncoded)]
pub fn derive_serde_encoded(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    integrations::serde_encoded(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Implements SQLx's PostgreSQL `Type`, `Encode`, `Decode`, and array support
/// for an `i64` ID type, stored as `BIGINT`.
///
/// Decoding validates the value. Requires the `sqlx-postgres` feature.
#[proc_macro_derive(SqlxPostgres)]
pub fn derive_sqlx_postgres(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    integrations::sqlx_postgres(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Implements `postgres-types`' `ToSql` and `FromSql` for an `i64` ID type,
/// stored as `BIGINT`.
///
/// Works with `tokio-postgres` and the synchronous `postgres` client. Decoding
/// validates the value. Requires the `postgres` feature.
#[proc_macro_derive(Postgres)]
pub fn derive_postgres(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    integrations::postgres(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
