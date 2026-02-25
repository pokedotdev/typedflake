# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Structure

TypedFlake is a **3-crate Rust workspace** for Snowflake-style ID generation:

- **`typedflake/`** - Public API facade crate (re-exports core + derive macro)
- **`typedflake-core/`** - Core runtime logic (config, context, generator, global, state)
- **`typedflake-macros/`** - Proc-macro crate (`#[derive(TypedFlake)]`)
- **`docs/`** - Additional documentation (newtype philosophy, etc.)

### Workspace Layout

```
typedflake/                          # repo root (virtual workspace)
├── Cargo.toml                       # [workspace] manifest
├── typedflake/                      # public API facade
│   ├── Cargo.toml
│   ├── src/lib.rs                   # re-exports core + macros
│   ├── tests/                       # integration tests
│   ├── examples/                    # example programs
│   └── benches/                     # performance benchmarks
├── typedflake-core/                 # core runtime logic
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── config.rs
│       ├── context.rs
│       ├── generator.rs
│       ├── global.rs
│       └── state.rs
├── typedflake-macros/               # proc-macro crate
│   ├── Cargo.toml
│   └── src/lib.rs
├── docs/
├── CLAUDE.md
└── README.md
```

## Common Commands

### Building and Testing

```bash
# Run all tests (unit tests + doctests + integration tests)
cargo test --workspace --all-targets
cargo test --workspace --doc

# Run with serde feature
cargo test --workspace --all-targets --all-features

# Run specific test file
cargo test -p typedflake --test integration
cargo test -p typedflake --test concurrency
cargo test -p typedflake --test validation
cargo test -p typedflake --test derive
cargo test -p typedflake --test serde --features serde

# Run specific test module in core
cargo test -p typedflake-core config::tests

# Build the workspace
cargo build --workspace

# Check for errors without building
cargo check --workspace

# Check for linting issues
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Format code
cargo fmt --all
```

### Examples

```bash
# Run examples
cargo run -p typedflake --example demo
cargo run -p typedflake --example distributed
cargo run -p typedflake --example override_defaults
cargo run -p typedflake --example serde --features serde
```

### Benchmarks

```bash
# Run performance benchmarks
cargo bench -p typedflake
```

## Code Architecture

TypedFlake is a Snowflake-style ID generator library built around a **newtype-driven design** where each ID type has its own independent generator state and configuration.

For comprehensive guidance on the newtype pattern philosophy and best practices used throughout this codebase, see [`docs/newtype-philosophy.md`](docs/newtype-philosophy.md).

### Crate Architecture

**`typedflake` (facade)** - Public API crate that users depend on. Re-exports all types from `typedflake-core` and the `TypedFlake` derive macro from `typedflake-macros`. Also re-exports `serde` as `__serde` for macro hygiene when the serde feature is enabled.

**`typedflake-core` (runtime)** - Contains all runtime types and logic. No dependency on the proc macro crate.

**`typedflake-macros` (proc macro)** - Contains the `#[derive(TypedFlake)]` proc macro. Generates code referencing `::typedflake::` paths (the facade). Has a marker `serde` feature (no deps) that controls whether serde impls are generated.

### Dependency Graph

```
typedflake (facade)
├── typedflake-core (runtime)
│   ├── dashmap
│   ├── derive_more
│   └── serde (optional)
├── typedflake-macros (proc macro)
│   ├── proc-macro2
│   ├── quote
│   └── syn
└── serde (optional, re-exported as __serde)
```

### Core Architecture

**`typedflake-macros/src/lib.rs`** - The `#[derive(TypedFlake)]` proc macro is the primary API entry point. It generates distinct newtype functionality with inherent methods (no trait required). Each generated type maintains its own static `IdContext` using `OnceLock`, ensuring thread-safe per-type state isolation. The macro also generates a typed wrapper struct `<Name>Generator` for each ID type. Supports `#[typedflake(config = EXPR)]` for custom configuration.

**`typedflake-core/src/config.rs`** - Provides compile-time configuration with three main types:

- `BitLayout`: Struct containing `timestamp`, `worker`, `process`, `sequence` bit allocations (must sum to 64). Includes industry-standard presets (`TWITTER`, `DISCORD`, `DEFAULT`) and capacity calculation methods.
- `Config`: Contains `BitLayout` and epoch timestamp. Pre-calculates shifts, masks, and limits for optimal bit manipulation performance.
- `ValidationError` and `BitLayoutError`: Error types for configuration validation.

**`typedflake-core/src/context.rs`** - Manages ID type context with three responsibilities:

- Holds the `Config` for an ID type
- Owns a `StatePool` that lazily initializes atomic states on-demand for (worker_id, process_id) instances
- Provides a default generator via lazy `OnceLock` initialization
- Factory methods: `create_generator()`, `create_worker()`, `create_process()`

**`typedflake-core/src/state.rs`** - State management with two key types:

- `State`: Atomic state for a single (worker_id, process_id) instance. Uses packed u64 for timestamp + sequence.
- `StatePool`: DashMap-based lazy state pool that creates states on-demand. Uses mathematical key packing `(worker_id << process_bits) | process_id` for O(1) lock-free concurrent access.

**`typedflake-core/src/generator.rs`** - Core ID generation logic:

- `Generator`: Bound to specific worker_id and process_id with pre-injected `Arc<State>`
- Zero-lookup ID generation using compare-and-swap operations on packed atomic state
- Provides both `generate()` (blocks) and `generate_internal()` (returns error on exhaustion)
- Component extraction and composition methods using pre-calculated config masks/shifts

**`typedflake-core/src/global.rs`** - Global configuration management using `OnceLock`:

- `set_defaults()`, `set_default_config()`, `set_default_instance()` for one-time initialization
- `get_default_config()` and `get_default_instance()` with fallback to hardcoded defaults

### Key Design Patterns

**Newtype Isolation**: Each `#[derive(TypedFlake)] struct TypeName(u64)` creates a completely independent type with its own `IdContext`. `UserId` and `OrderId` can never interfere with each other.

**Zero-Cost Abstractions**: Bit operations use pre-calculated shifts and masks stored in the `Config` struct, avoiding runtime calculations.

**No Trait Required**: All methods are inherent methods - users never need to import traits.

**Lazy State Initialization**: `StatePool` uses DashMap for on-demand state creation with mathematical key packing, providing lock-free concurrent access with minimal memory footprint.

**Packed Atomic State**: Each `State` uses a single `AtomicU64` packing both timestamp and sequence for efficient compare-and-swap operations.

### Derive Macro API

The `#[derive(TypedFlake)]` macro generates types that:

- Provide all methods as inherent methods (no trait imports needed)
- Include standard traits: `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `PartialOrd`, `Ord`
- Provide conversion methods: `as_u64()`, `from_u64_unchecked()`, `try_from_u64()`, `From<Name> for u64`, `TryFrom<u64>`
- Support string parsing via `FromStr` and `Display`
- Maintain independent static `IdContext` per type
- Factory methods: `instance()`, `worker()`, `process()` returning typed `<Name>Generator`
- Static methods: `generate()`, `compose()`, `compose_unchecked()`, `compose_custom()`, `compose_custom_unchecked()`
- Instance methods: `decompose()`, `components()`, `timestamp()`, `worker_id()`, `process_id()`, `sequence()`
- Conditional serde support: `Serialize`/`Deserialize` when the `serde` feature is enabled

### Thread Safety Model

- Each generated ID type has its own `IdContext` with `StatePool`
- `StatePool` lazily initializes states on first access using DashMap's lock-free concurrent HashMap
- Per-(worker_id, process_id) atomic state using compare-and-swap operations
- `Generator` provides lock-free ID generation with pre-injected `Arc<State>`
- No shared global state between different ID types or instances

### API Philosophy

The library prioritizes **real-world usage patterns** over theoretical abstractions:

- **Zero imports**: `#[derive(TypedFlake)] pub struct UserId(u64);` then `UserId::generate()` works immediately
- **Type safety**: `UserId` and `OrderId` are distinct types that cannot be mixed
- **Extensible**: Users can freely add their own derives and attributes alongside `TypedFlake`
- **Performance**: Direct method calls with no trait dispatch overhead, lock-free concurrent access
- **Simplicity**: Inherent methods eliminate the need for trait knowledge
- Backward compatibility is not a priority, as the project is in early experimental development

## Code Style

**Comments**: Be practical and minimal. Code should be self-explanatory through clear naming and structure. Only add comments when:
- Explaining complex algorithms or non-obvious logic
- Documenting public API with doc comments (`///`)
- Clarifying "why" rather than "what"

Avoid verbose or redundant comments. Let the code speak for itself.
