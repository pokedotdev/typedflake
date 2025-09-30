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
//! - **Customizable bit allocation**: Configure timestamp, worker, process, and sequence bits
//! - **Custom epoch support**: Set your own epoch for timestamp calculation
//! - **Thread-safe**: Each ID type maintains its own independent generator state
//! - **Zero-cost abstractions**: Pre-calculated shifts and masks for optimal performance
//! - **Flexible configuration**: Runtime builder pattern or compile-time const creation
//!
//! ## Quick Start
//!
//! ```rust
//! // Create an ID type with default configuration
//! typedflake::id!(UserId);
//!
//! // Generate IDs (default instance with worker_id=0, process_id=0)
//! let user_id = UserId::generate().unwrap();
//! println!("Generated user ID: {}", user_id);
//!
//! // Generate with specific instance
//! let user_instance = UserId::instance(15, 7).unwrap();
//! let custom_id = user_instance.generate().unwrap();
//!
//! // Access components
//! let timestamp = user_id.timestamp();
//! let (timestamp, worker_id, process_id, sequence) = user_id.decompose();
//! ```
//!
//! ## Custom Configuration
//!
//! ```rust
//! use typedflake::Config;
//!
//! // Custom algorithm configuration
//! const SESSION_ALGORITHM: Config = Config::new(
//!     (42, 10, 0, 12),   // bits: timestamp, worker, process, sequence
//!     1_600_000_000_000, // epoch
//! );
//!
//! typedflake::id!(SessionId, SESSION_ALGORITHM);
//!
//! // Another ID type with different algorithm
//! const USER_ALGORITHM: Config = Config::new(
//!     (42, 10, 5, 7),
//!     1_600_000_000_000,
//! );
//!
//! typedflake::id!(UserId, USER_ALGORITHM);
//!
//! let session_id = SessionId::generate().unwrap();
//! let user_id = UserId::generate().unwrap();
//! println!("Generated session ID: {}", session_id);
//! println!("Generated user ID: {}", user_id);
//! ```
//!
//! ## Shared Algorithm with Dynamic Instances
//!
//! ```rust
//! use typedflake::Config;
//!
//! // Shared algorithm configuration across services
//! const SHARED_ALGORITHM: Config = Config::new(
//!     (42, 6, 4, 12),        // bits: timestamp, worker, process, sequence
//!     1_640_000_000_000,     // 2022 epoch
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
//! let auth_id = auth_instance.generate().unwrap();
//! let payment_id = payment_instance.generate().unwrap();
//! ```
//!
//! ## ID Operations
//!
//! ```rust
//! # use typedflake::Config;
//! # typedflake::id!(ExampleId);
//! let id = ExampleId::generate().unwrap();
//!
//! // Convert to/from u64
//! let raw: u64 = id.as_u64();
//! let id2 = ExampleId::from_u64(raw);
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
//! let composed_id = ExampleId::compose(timestamp, worker_id, process_id, sequence);
//! ```

pub mod config;
pub mod context;
pub mod generator;
pub mod global;
pub mod state;
pub mod traits;

mod macros; // Keep macros private, they're exported via the macro itself

// Re-export main types and the macro
pub use config::{Config, ValidationError};
pub use context::IdContext;
pub use generator::{Generator, GeneratorError, IdComponents};
