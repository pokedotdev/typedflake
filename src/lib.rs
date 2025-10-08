//! # TypedFlake
//!
//! A Snowflake-style ID generator library with newtype-driven design.
//!
//! TypedFlake provides a flexible and type-safe way to generate distributed unique IDs
//! similar to Twitter's Snowflake algorithm. Each ID type has its own independent generator
//! state and configuration, ensuring thread-safety and preventing ID collisions between
//! different entity types.
//!
//! ## Features
//!
//! - **Type-safe ID generation**: Each ID type is a distinct newtype
//! - **Validated by default**: All constructors validate component ranges following newtype best practices
//! - **Customizable bit allocation**: Configure timestamp, worker, process, and sequence bits
//! - **Custom epoch support**: Set your own epoch for timestamp calculation
//! - **Thread-safe**: Each ID type maintains its own independent generator state
//! - **Zero-cost abstractions**: Pre-calculated shifts and masks for optimal performance
//! - **Flexible configuration**: Runtime builder pattern or compile-time const creation
//!
//! ## Validation Philosophy
//!
//! TypedFlake follows the newtype pattern principle: "design datatypes that are always valid."
//! All ID construction methods validate that components are within configured bit limits:
//!
//! ```rust
//! # typedflake::id!(UserId);
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! # let raw_value = UserId::generate().as_u64();
//! # let (ts, _, _, seq) = UserId::generate().decompose();
//! # let (ts2, worker, process, seq2) = UserId::generate().decompose();
//! // ✅ Validated constructors (preferred)
//! let id = UserId::try_from_u64(raw_value)?;                          // From raw u64
//! let id: UserId = raw_value.try_into()?;                             // Via TryFrom
//! let id = UserId::compose(ts, seq)?;                                 // Timestamp + sequence
//! let id = UserId::compose_custom(ts2, worker, process, seq2)?;       // All components
//! let id: UserId = "12345".parse()?;                                  // From string
//!
//! // ⚠️ Unchecked constructors (performance-critical paths only)
//! let id = UserId::from_u64_unchecked(raw_value);                     // No validation
//! let id = UserId::compose_unchecked(ts, seq);                        // No validation
//! let id = UserId::compose_custom_unchecked(ts2, worker, process, seq2); // No validation
//! # Ok(())
//! # }
//! ```
//!
//! ## Quick Start
//!
//! ```rust
//! // Create an ID type with default configuration
//! typedflake::id!(UserId);
//!
//! // Generate IDs (using default instance)
//! let user_id = UserId::generate();
//! println!("Generated user ID: {}", user_id);
//!
//! // Generate with specific instance
//! let user_instance = UserId::instance(15, 7).unwrap();
//! let custom_id = user_instance.generate();
//!
//! // Access components
//! let timestamp = user_id.timestamp();
//! let (timestamp, worker_id, process_id, sequence) = user_id.decompose();
//! ```
//!
//! ## Custom Configuration
//!
//! ```rust
//! use typedflake::{BitLayout, Config, Epoch};
//!
//! // Custom algorithm configuration
//! const SESSION_ALGORITHM: Config = Config::new_unchecked(
//!     BitLayout::new(42, 10, 0, 12),   // bits: timestamp, worker, process, sequence
//!     Epoch::new(1_600_000_000_000), // epoch
//! );
//!
//! typedflake::id!(SessionId, SESSION_ALGORITHM);
//!
//! // Another ID type with different algorithm
//! const USER_ALGORITHM: Config = Config::new_unchecked(
//!     BitLayout::new(42, 10, 5, 7),
//!     Epoch::new(1_600_000_000_000),
//! );
//!
//! typedflake::id!(UserId, USER_ALGORITHM);
//!
//! let session_id = SessionId::generate();
//! let user_id = UserId::generate();
//! println!("Generated session ID: {}", session_id);
//! println!("Generated user ID: {}", user_id);
//! ```
//!
//! ## Shared Algorithm with Dynamic Instances
//!
//! ```rust
//! use typedflake::{BitLayout, Config, Epoch};
//!
//! // Shared algorithm configuration across services
//! const SHARED_ALGORITHM: Config = Config::new_unchecked(
//!     BitLayout::new(42, 6, 4, 12),   // bits: timestamp, worker, process, sequence
//!     Epoch::new(1_640_000_000_000),  // 2022 epoch
//! );
//!
//! // Service-specific ID types
//! typedflake::id!(AuthServiceId, SHARED_ALGORITHM);
//! typedflake::id!(PaymentServiceId, SHARED_ALGORITHM);
//!
//! // Generate IDs with dynamic instances
//! let auth_instance = AuthServiceId::instance(1, 0).unwrap();
//! let payment_instance = PaymentServiceId::instance(2, 1).unwrap();
//!
//! let auth_id = auth_instance.generate();
//! let payment_id = payment_instance.generate();
//! ```
//!
//! ## ID Operations
//!
//! ```rust
//! # typedflake::id!(ExampleId);
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let id = ExampleId::generate();
//!
//! // Convert to/from u64
//! let raw: u64 = id.as_u64();
//! let id2 = ExampleId::from_u64_unchecked(raw); // Unchecked for trusted data
//! let id3 = ExampleId::try_from_u64(raw).unwrap(); // Validated for external data
//!
//! // Decompose into components
//! let (timestamp, worker_id, process_id, sequence) = id.decompose();
//! let components = id.components();
//!
//! // Access individual components
//! let timestamp = id.timestamp();
//! let worker_id = id.worker_id();
//! let process_id = id.process_id();
//! let sequence = id.sequence();
//!
//! // Compose from components
//! let composed_id = ExampleId::compose_custom(timestamp, worker_id, process_id, sequence)?;
//! # Ok(())
//! # }
//! ```

pub mod config;
pub mod context;
pub mod generator;
pub mod global;
pub mod state;
pub mod traits;

mod macros; // Keep macros private, they're exported via the macro itself

// Re-export main types and the macro
pub use config::{
    BitLayout, BitLayoutError, Config, ConfigError, Epoch, EpochError, ValidationError,
};
pub use context::IdContext;
pub use generator::{Generator, GeneratorError, IdComponents};
