//! # TypedFlake
//!
//! Distributed, type-safe Snowflake ID generation for Rust.
//!
//! Generate unique, time-ordered 64-bit IDs across distributed systems without coordination.
//! Each ID type is a distinct newtype that cannot be mixed with others at compile time.
//!
//! ## Quick Start
//!
//! ```rust
//! use typedflake::TypedFlake;
//!
//! #[derive(TypedFlake)]
//! pub struct UserId(u64);
//!
//! fn example() {
//!     let id = UserId::generate();
//!     let (timestamp, worker_id, process_id, sequence) = id.decompose();
//! }
//! ```
//!
//! ## Custom Configuration
//!
//! ```rust
//! use typedflake::TypedFlake;
//!
//! // Inline epoch
//! #[derive(TypedFlake)]
//! #[typedflake(epoch = "2025-01-01")]
//! pub struct UserId2(u64);
//!
//! // Inline layout + epoch
//! #[derive(TypedFlake)]
//! #[typedflake(layout = (42, 8, 4, 10), epoch = "2025-06-01")]
//! pub struct SessionId(u64);
//!
//! fn example() {
//!     let id = SessionId::generate();
//! }
//! ```
//!
//! ## Main Types
//!
//! - [`Config`] - Complete configuration (bit layout + epoch)
//! - [`BitLayout`] - Bit allocation for the 64-bit ID space
//! - [`Epoch`] - Custom epoch timestamps
//!
//! Use `#[derive(TypedFlake)]` to create new ID types with their own independent state.

pub use typedflake_macros::TypedFlake;

pub use typedflake_core::global::defaults;
pub use typedflake_core::{
    BitLayout, BitLayoutError, Config, ConfigError, Epoch, EpochError, Generator, GeneratorError,
    IdComponents, IdContext, ValidationError, config, context, generator, global, state,
};

#[cfg(feature = "serde")]
#[doc(hidden)]
pub use serde as __serde;
