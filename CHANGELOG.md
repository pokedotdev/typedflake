# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->

## [Unreleased] - ReleaseDate

## [0.2.0](https://github.com/pokedotdev/typedflake/compare/v0.1.3...v0.2.0) - 2026-10-04

A redesign of the whole interface. See [MIGRATION.md](MIGRATION.md) for how to
move from 0.1 and keep reading existing IDs.

### Added

- `#[typedflake(...)]` attribute for declaring ID types on `i64` or `u64` newtypes.
- `i64` IDs, which are never negative and fit signed database columns.
- Formats that use fewer bits than the integer holds; the unused upper bits are reserved and validated.
- `Format` and `BitLayout` as plain struct constants, validated at compile time against each ID type.
- `TypedNode` derive for splitting the node into named fields.
- `typedflake::init(node)` for installing a default node, once per node type.
- `Generator<Id>`, obtained from `Id::generator(node)`; generators for one ID type and node always share state.
- `generate_blocking()` and, with the `tokio` feature, `generate_async()`.
- `Parts`, `parts()`, `from_parts()`, and `unix_millis()` for inspecting and rebuilding IDs.
- `TryFrom<i64>` and `TryFrom<u64>` for every ID type.
- `Id` trait for code generic over ID types.
- `typedflake::SqlxPostgres` derive (feature `sqlx-postgres`) for SQLx 0.8, including arrays.
- `typedflake::Postgres` derive (feature `postgres`) for `postgres-types` 0.2.
- Typed errors: `FormatError`, `NodeError`, `InitError`, `GeneratorError`, `GenerateError`, `InvalidId`, `ParseIdError`, `TimestampError`.

### Changed

- **Breaking:** `id!` and `#[derive(TypedFlake)]` are replaced by `#[typedflake(...)]`.
- **Breaking:** the epoch is required; there is no default epoch.
- **Breaking:** worker and process are replaced by a single node field.
- **Breaking:** `generate()` returns a `Result` and never waits. Sequence exhaustion and clock rollback are errors.
- **Breaking:** generating without `typedflake::init` is an error instead of using worker 0 and process 0.
- **Breaking:** Serde support is the opt-in `typedflake::Serde` derive instead of automatic. IDs still serialize as strings; deserialization now also accepts integers.
- **Breaking:** the minimum supported Rust version is 1.99.
- A clock that moves backwards is reported, and generation resumes once it catches up.

### Removed

- **Breaking:** `Config`, the `TWITTER`, `DISCORD`, and `INSTAGRAM` presets, and the global configuration functions.
- **Breaking:** unchecked constructors (`from_u64_unchecked`, `compose_unchecked`, `compose_custom_unchecked`).
- **Breaking:** `as_u64`, `decompose`, `components`, `compose`, and the per-component accessors, replaced by `get`, `parts`, and `from_parts`.
- **Breaking:** the per-type `<Name>Generator` structs and the `instance`, `worker`, and `process` constructors.

## [0.1.3](https://github.com/pokedotdev/typedflake/compare/v0.1.2...v0.1.3) - 2025-10-10

### Fixed

- *(macro)* resolve hygiene issues with paste and serde cfg checks

### Other

- add GitHub Actions workflow for tests and linting

## [0.1.2](https://github.com/pokedotdev/typedflake/compare/v0.1.1...v0.1.2) - 2025-10-10

### Fixed

- add missing serde feature definition to Cargo.toml

### Other

- *(readme)* correct blockquote alert syntax
- Merge branch 'main' of github.com:pokedotdev/typedflake

## [0.1.1](https://github.com/pokedotdev/typedflake/compare/v0.1.0...v0.1.1) - 2025-10-10

### Added

- add optional serde support for IEEE 754-safe JSON serialization

### Other

- remove API Guide wrapper and flatten heading hierarchy
- *(ci)* use YAML anchors to deduplicate workflow steps
- release v0.1.0

## [0.1.0](https://github.com/pokedotdev/typedflake/releases/tag/v0.1.0) - 2025-10-09

### Added

- *(config)* add config presets
- *(state)* implement lazy state initialization with DashMap
- *(config)* add runtime config validation API
- *(config)* map error variants to static strings for const diagnostics
- [**breaking**] add Epoch type for configuration presets
- [**breaking**] encapsulate BitLayout fields with accessors
- add structured error handling with instance validation and context
- simplify global config getter

### Other

- add release-plz automation for automated releases
- add crates.io publishing metadata to Cargo.toml
- add README with API guide and examples
- *(examples)* simplify demo example
- *(macros)* inline trait implementations into id! macro
- improve module-level documentation across codebase
- add inline annotations to hot path methods
- *(generator)* add spin-wait before sleep in sequence exhaustion
- *(state)* add cache-line alignment to prevent false sharing
- *(generator)* fix stale timestamp in CAS retry loop
- simplify verbose comments and remove redundant ones
- [**breaking**] make generate() blocking and error-free, rename error-returning variant to generate_internal()
- remove hardcoded (0,0) references from default instance documentation
- *(context)* add tests for state sharing across generator instance
- *(deps)* upgrade criterion to 0.7 and narrow derive_more features
- *(config)* [**breaking**] rename Config::new to Config::new_unchecked
- swap thiserror for derive_more error derives
- expose Config getters instead of public fields
- > feat: add validated ID constructors and component checks
- introduce BitLayout struct with industry presets
- clean tests
- move integration tests
- rename StateVec->StatePool and IdManager->IdContext for clarity
- simplify test configs using Config::default() and rational bit allocations
- simplify architecture by merging factory into manager and
- simplify id macro
- remove convenience methods and redundant fields
- update README and simplify example code
- optimize state management using config with pre-calculated
- init
