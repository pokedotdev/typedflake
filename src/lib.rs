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
//! typedflake::id!(UserId);
//!
//! fn example() {
//!     let id = UserId::generate();
//!     let (timestamp, worker_id, process_id, sequence) = id.decompose();
//! }
//! ```
//!
//! ## Main Types
//!
//! - [`Config`] - Complete configuration (bit layout + epoch)
//! - [`BitLayout`] - Bit allocation for the 64-bit ID space
//! - [`Epoch`] - Custom epoch timestamps
//!
//! Use the [`id!`](crate::id) macro to create new ID types with their own independent state.

pub mod config;
pub mod context;
pub mod generator;
pub mod global;
pub mod state;

mod macros; // Keep macros private, they're exported via the macro itself

// Re-export main types and the macro
pub use config::{
    BitLayout, BitLayoutError, Config, ConfigError, Epoch, EpochError, ValidationError,
};
pub use context::IdContext;
pub use generator::{Generator, GeneratorError, IdComponents};
