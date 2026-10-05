# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Structure

TypedFlake is a **2-crate Rust workspace** for Snowflake-style ID generation:

- **`typedflake/`** - Runtime and public interface
- **`typedflake-macros/`** - Proc-macro crate (`#[typedflake]`, `TypedNode`, integration derives)

Two non-published members support it: `examples/axum-sqlx/` (an integration example) and `tests/renamed-dependency/` (checks generated paths when the dependency is renamed).

### Workspace Layout

```
typedflake/                          # repo root (virtual workspace)
├── Cargo.toml                       # [workspace] manifest
├── typedflake/                      # runtime and public interface
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs                   # re-exports, `__private` for generated code
│   │   ├── format.rs                # Epoch, BitLayout, Format, Layout
│   │   ├── node.rs                  # Node trait, NodeError
│   │   ├── id.rs                    # Id and Repr traits, Parts, value errors
│   │   ├── state.rs                 # atomic state and its registry
│   │   ├── generator.rs             # Generator, generation errors
│   │   ├── global.rs                # init and the default nodes
│   │   ├── clock.rs                 # clock seam
│   │   └── integrations/            # serde, sqlx_postgres, postgres
│   ├── tests/                       # integration and UI tests
│   ├── examples/
│   └── benches/
├── typedflake-macros/
│   └── src/
│       ├── lib.rs                   # macro entry points and their docs
│       ├── id.rs                    # #[typedflake]
│       ├── node.rs                  # TypedNode
│       ├── integrations.rs          # Serde, SqlxPostgres, Postgres
│       └── util.rs
├── examples/axum-sqlx/
├── tests/renamed-dependency/
├── MIGRATION.md
├── PROPOSAL.md                      # design record for the 0.2 redesign
└── README.md
```

## Common Commands

### Building and Testing

```bash
# Run all tests with every integration enabled
cargo test --workspace --all-targets --all-features
cargo test --workspace --doc --all-features

# Run the core tests without optional features
cargo test -p typedflake --all-targets

# Run a specific test file
cargo test -p typedflake --test values
cargo test -p typedflake --test generation
cargo test -p typedflake --test init_simple
cargo test -p typedflake --test init_mixed
cargo test -p typedflake --test ui
cargo test -p typedflake --test serde --features serde

# Run a specific unit test module
cargo test -p typedflake --lib state::tests

# Check for linting issues
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Format code
cargo fmt --all
```

Database round-trip tests are skipped unless `TYPEDFLAKE_TEST_POSTGRES_URL` points at a PostgreSQL server:

```bash
docker run -d --rm --name typedflake-test-pg -e POSTGRES_PASSWORD=postgres \
    -p 127.0.0.1:54329:5432 postgres:17-alpine
export TYPEDFLAKE_TEST_POSTGRES_URL=postgres://postgres:postgres@127.0.0.1:54329/postgres
```

UI tests (`tests/ui/`) pin compiler diagnostics for Rust 1.99. Keep cases whose output quotes standard library sources out of them (that output depends on `rust-src` being installed); use `compile_fail` doctests in `lib.rs` for those. Regenerate them with `TRYBUILD=overwrite cargo test -p typedflake --test ui`, once without features and once with `--all-features`. Set `TYPEDFLAKE_SKIP_UI=1` to skip them on another compiler.

### Examples and Benchmarks

```bash
cargo run -p typedflake --example demo
cargo run -p typedflake --example typed_node
cargo run -p typedflake --example serde --features serde
cargo run -p typedflake --example waiting --features tokio

cargo bench -p typedflake --bench performance
```

## Code Architecture

Each ID is a newtype declared with `#[typedflake(...)]`. The attribute implements the `Id` trait; almost all logic lives in generic runtime code that reaches the type through that trait.

### Runtime (`typedflake`)

**`format.rs`** - The persistent format. `Epoch`, `BitLayout`, and `Format` are plain copyable data. `Format::validate` is a const fn; `Layout::resolve` turns a valid format into precomputed shifts and limits. Each ID declaration checks its format in a const of its own and panics there with the reason, so an invalid format is a compile error that points at the declaration.

**`node.rs`** - The `Node` trait. `u32` is the plain node and the only integer one, which lets `init(17)` infer its type. `TypedNode` structs implement it with a fixed width. Nodes are validated when consumed, never truncated.

**`id.rs`** - The `Id` trait with the pure operations (`parts`, `from_parts`, `unix_millis`) and the generation entry points as provided methods, the sealed `Repr` trait for `i64`/`u64`, and the single validation path (`from_u64`, `from_i64`, `parse`) used by every constructor and integration.

**`state.rs`** - `State` packs the last timestamp and sequence in one `AtomicU64` and advances them with compare-and-swap. The clock is read after each load of the state, so another thread's newer millisecond is not mistaken for a rollback. A registry keyed by ID type and packed node hands out one shared `State` per pair.

**`generator.rs`** - `Generator<I>` holds an `Arc<State>` and the pre-shifted node. `generate` never waits; `generate_blocking` and `generate_async` retry sequence exhaustion only. `default_generator` resolves and caches the generator behind the static `Id::generate()`.

**`global.rs`** - `init` stores one default per node type, keyed by the type, so an ID only receives a node of its own node type.

**`clock.rs`** - Wall-clock access. In unit tests a thread-local mock freezes it; there is no public clock abstraction.

**`integrations/`** - Shared implementations for the integration derives, each behind its Cargo feature. They decode through the same validation as `TryFrom`.

### Macros (`typedflake-macros`)

Macros only parse, validate syntax, and forward to the runtime through `::typedflake::__private`. Business logic stays in the runtime. The crate path is found with `proc-macro-crate`, so a renamed dependency works.

Marker features (`serde`, `sqlx-postgres`, `postgres`, `tokio`) mirror the runtime's features and decide what the macros emit. A derive used without its feature produces a clear error.

### Key Design Rules

- **No unchecked constructors.** Every path into an ID validates sign and reserved bits.
- **Format belongs to the ID type.** Deployment only selects a node.
- **One state per ID type and node.** No public path creates a second, uncoordinated one.
- **Pure operations stay pure.** Parsing, conversion, and decomposition never touch globals or the clock.
- **Inherent methods.** Users never need to import a trait; `Id` exists for generic code.
- Backward compatibility is not a priority, as the project is in early experimental development.

## Code Style

**Comments**: Be practical and minimal. Code should be self-explanatory through clear naming and structure. Only add comments when:
- Explaining complex algorithms or non-obvious logic
- Documenting public API with doc comments (`///`)
- Clarifying "why" rather than "what"

Avoid verbose or redundant comments. Let the code speak for itself.
