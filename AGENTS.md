# Repository Guidelines

TypedFlake generates Snowflake-style IDs where each kind of ID is its own type. It is a Rust workspace with MSRV 1.99:

- `typedflake/`: runtime and public interface.
- `typedflake-macros/`: `#[typedflake]`, `TypedNode`, and the integration derives.
- `examples/axum-sqlx/`: example application, not published.
- `tests/renamed-dependency/`: checks generated paths under a renamed dependency, not published.

## Commands

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo test --workspace --doc --all-features
cargo test -p typedflake --all-targets          # without optional features
cargo bench -p typedflake --bench performance
```

Run the first four before handing off; CI also runs Clippy with each feature alone.

Database tests are skipped unless `TYPEDFLAKE_TEST_POSTGRES_URL` is set:

```bash
docker run -d --rm --name typedflake-test-pg -e POSTGRES_PASSWORD=postgres \
    -p 127.0.0.1:54329:5432 postgres:17-alpine
export TYPEDFLAKE_TEST_POSTGRES_URL=postgres://postgres:postgres@127.0.0.1:54329/postgres
```

## Architecture

Each ID is a newtype declared with `#[typedflake(...)]`. The attribute implements the `Id` trait, and almost all logic lives in generic runtime code that reaches the type through that trait.

Runtime modules in `typedflake/src/`:

- `format.rs`: `Epoch`, `BitLayout`, and `Format`. Each ID declaration validates its format in a const, so an invalid format is a compile error at the declaration.
- `node.rs`: the `Node` trait. `u32` is the only integer node, which lets `init(17)` infer its type; `TypedNode` structs have a fixed width.
- `id.rs`: the `Id` trait and the single validation path used by every constructor and integration.
- `state.rs`: the last timestamp and sequence in one `AtomicU64`, advanced with compare-and-swap. The clock is read after each load of the state, so another thread's newer millisecond is not mistaken for a rollback.
- `generator.rs`: `Generator<I>` and the cached generator behind `Id::generate()`.
- `global.rs`: `init`, with one default node per node type.
- `clock.rs`: wall-clock access, with a thread-local mock for unit tests.
- `integrations/`: shared code for the integration derives, each behind its Cargo feature.

Macros only parse, report syntax errors, and forward to the runtime through `::typedflake::__private`. Behavior belongs in the runtime.

## Design rules

- **No unchecked constructors.** Every path into an ID validates sign and reserved bits.
- **Format belongs to the ID type.** Deployment only selects a node.
- **One state per ID type and node.** No public path creates a second, uncoordinated one.
- **Pure operations stay pure.** Parsing, conversion, and decomposition never touch globals or the clock.
- **Inherent methods.** Users never need to import a trait; `Id` exists for generic code.
- **Backward compatibility is not a priority.** The project is in early development.

## Testing

- Public behavior tests go in `typedflake/tests/<area>.rs`; unit tests sit beside the code.
- A test that calls `typedflake::init` needs its own test file, because default nodes are process-wide.
- Macro changes need accepted and rejected cases under `typedflake/tests/ui/`.
- UI tests pin compiler diagnostics for Rust 1.99. Regenerate them with `TRYBUILD=overwrite cargo test -p typedflake --test ui`, once without features and once with `--all-features`. Set `TYPEDFLAKE_SKIP_UI=1` to skip them on another compiler.
- Diagnostics that quote standard library sources depend on `rust-src` being installed. Keep those cases out of the UI tests and use `compile_fail` doctests in `lib.rs`.
- `README.md` and `MIGRATION.md` are compiled as doctests, so their examples must build.

## Conventions

- Comments explain why, not what. Public items get `///` docs.
- Use Conventional Commits (`feat:`, `fix:`, `docs:`, `chore:`), with `!` for breaking changes.
- When behavior changes, update `README.md`, `CHANGELOG.md`, and `MIGRATION.md` as needed.
