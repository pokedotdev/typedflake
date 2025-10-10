# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->

## [Unreleased] - ReleaseDate

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
