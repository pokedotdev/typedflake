# Repository Guidelines

## Workspace Map

TypedFlake is a Rust 2024 workspace with MSRV 1.99. Keep changes within the appropriate crate:

- `typedflake/`: runtime and public interface; integration and UI tests, examples, and Criterion benchmarks live under `tests/`, `examples/`, and `benches/`.
- `typedflake-macros/`: `#[typedflake]`, `TypedNode`, and the integration derives.
- `examples/axum-sqlx/`: non-published integration application.
- `tests/renamed-dependency/`: non-published crate that checks generated paths under a renamed dependency.

Document public usage in `README.md`; record release-facing behavior changes in `CHANGELOG.md` and upgrade steps in `MIGRATION.md`. `PROPOSAL.md` records the design decisions behind the 0.2 interface.

## Development Commands

- `cargo check --workspace --all-targets --all-features` — fast workspace validation.
- `cargo build --workspace --examples --all-features` — build every library and example.
- `cargo test --workspace --all-targets --all-features` — run unit and integration tests.
- `cargo test --workspace --doc --all-features` — run doctests, which CI checks separately.
- `cargo fmt --all --check` — verify formatting; use `cargo fmt --all` to fix it.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — apply the CI lint policy.
- `cargo run -p typedflake --example demo` — exercise the basic API locally.
- `cargo bench -p typedflake --bench performance` — run Criterion benchmarks when performance-sensitive code changes.
- `TRYBUILD=overwrite cargo test -p typedflake --test ui` — regenerate pinned compiler diagnostics; run it without features and with `--all-features`.

Database round-trip tests are skipped unless `TYPEDFLAKE_TEST_POSTGRES_URL` points at a PostgreSQL server.

## Coding Style & Naming Conventions

Let rustfmt control layout (four-space indentation). Use `snake_case` for modules, functions, and tests; `PascalCase` for types and generated ID newtypes; and `SCREAMING_SNAKE_CASE` for constants. Keep macros thin: token parsing and syntax errors belong in `typedflake-macros`, while behavior belongs in generic runtime code in `typedflake` that the macros forward to. Add `///` documentation to public APIs. Comments should explain constraints or reasoning, not restate code.

## Testing Guidelines

Place unit tests beside implementation code and public behavior tests in `typedflake/tests/<area>.rs`. Name tests after observable behavior, for example `sequence_exhaustion_and_recovery`. Cover the no-feature and all-feature configurations. Macro changes should test accepted syntax, generated behavior, and rejection cases under `tests/ui/`. Tests that call `typedflake::init` need their own test file, because the default node is process-wide. No coverage threshold is enforced; prioritize configuration validation, ID boundaries, concurrency, and serialization. Before handing off, run formatting, Clippy, workspace tests, and doctests.

## Commit & Pull Request Guidelines

Use Conventional Commits (`feat:`, `fix:`, `refactor:`, `chore:`), optional scopes such as `fix(macro):`, and `!` for breaking changes. Keep commits focused; change `Cargo.lock` only when dependencies change. Pull requests should explain motivation and API impact, link relevant issues, list verification commands, and update tests, `README.md`, or `CHANGELOG.md` when behavior changes. Screenshots are only relevant to rendered documentation or UI examples.
