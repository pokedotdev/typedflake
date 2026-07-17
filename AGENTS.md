# Repository Guidelines

## Workspace Map

TypedFlake is a Rust 2024 workspace with MSRV 1.85. Keep changes within the appropriate crate:

- `typedflake/`: public facade and re-exports; integration tests, examples, and Criterion benchmarks live under `tests/`, `examples/`, and `benches/`.
- `typedflake-core/`: runtime configuration, generation, context, and state management.
- `typedflake-macros/`: `#[derive(TypedFlake)]` parsing and code generation.
- `examples/axum-sqlx/`: non-published integration application.

Document public usage in `README.md`; record release-facing behavior changes in `CHANGELOG.md`.

## Development Commands

- `cargo check --workspace --all-targets --all-features` — fast workspace validation.
- `cargo build --workspace --examples --all-features` — build every library and example.
- `cargo test --workspace --all-targets --all-features` — run unit and integration tests.
- `cargo test --workspace --doc --all-features` — run doctests, which CI checks separately.
- `cargo fmt --all --check` — verify formatting; use `cargo fmt --all` to fix it.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — apply the CI lint policy.
- `cargo run -p typedflake --example demo` — exercise the basic API locally.
- `cargo bench -p typedflake` — run Criterion benchmarks when performance-sensitive code changes.

## Coding Style & Naming Conventions

Let rustfmt control layout (four-space indentation). Use `snake_case` for modules, functions, and tests; `PascalCase` for types and generated ID newtypes; and `SCREAMING_SNAKE_CASE` for constants. Keep `typedflake` thin: runtime behavior belongs in `typedflake-core`, while token parsing and emitted implementations belong in `typedflake-macros`. Add `///` documentation to public APIs. Comments should explain constraints or reasoning, not restate code.

## Testing Guidelines

Place unit tests beside implementation code and public behavior tests in `typedflake/tests/<area>.rs`. Name tests after observable behavior, for example `sequence_exhaustion_and_recovery`. Cover default and `serde`/all-feature configurations. Macro changes should test accepted syntax, generated behavior, and rejection cases. No coverage threshold is enforced; prioritize configuration validation, ID boundaries, concurrency, and serialization. Before handing off, run formatting, Clippy, workspace tests, and doctests.

## Commit & Pull Request Guidelines

Use Conventional Commits (`feat:`, `fix:`, `refactor:`, `chore:`), optional scopes such as `fix(macro):`, and `!` for breaking changes. Keep commits focused; change `Cargo.lock` only when dependencies change. Pull requests should explain motivation and API impact, link relevant issues, list verification commands, and update tests, `README.md`, or `CHANGELOG.md` when behavior changes. Screenshots are only relevant to rendered documentation or UI examples.
