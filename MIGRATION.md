# Migrating from 0.1 to 0.2

0.2 is a redesign. IDs declared one way in 0.1 must be declared again in 0.2, but **IDs already stored keep their values and meaning** if you follow the [format section](#keep-reading-existing-ids) below.

## What changed

| 0.1 | 0.2 |
| --- | --- |
| `typedflake::id!(UserId);` | `#[typedflake(epoch = "...")] pub struct UserId(i64);` |
| Always `u64` | `i64` (recommended) or `u64` |
| Default epoch of 2025-01-01 | Epoch is required |
| `Config::new_unchecked(BitLayout::new(t, w, p, s), epoch)` | `Format { epoch, bits: BitLayout { timestamp, node, sequence } }` |
| `id!(UserId, CONFIG)` | `#[typedflake(format = CONSTANT)]` |
| Worker and process fields | One node field: a `u32`, or a `TypedNode` struct with named fields |
| `Config::TWITTER`, `Config::DISCORD`, and their layouts and epochs | Removed; declare the format yourself |
| `global::set_defaults(config, worker, process)` | `typedflake::init(node)`; the format always belongs to the ID type |
| `UserId::generate()` returns an ID and waits when needed | Returns `Result`; never waits |
| | `generate_blocking(timeout)` and `generate_async()` wait for capacity |
| `UserId::instance(w, p)`, `worker(w)`, `process(p)` | `UserId::generator(node)` returning `Generator<UserId>` |
| `id.as_u64()` | `id.get()` or `i64::from(id)` |
| `UserId::try_from_u64(raw)`, `TryFrom<u64>` | `UserId::try_from(raw)` for `i64` and `u64` |
| `UserId::from_u64_unchecked(raw)` | Removed; every constructor validates |
| `id.decompose()`, `id.components()` | `id.parts()` returning `Parts { elapsed_millis, node, sequence }` |
| `id.timestamp()`, `worker_id()`, `process_id()`, `sequence()` | Fields of `id.parts()`; `id.unix_millis()` for the absolute time |
| `UserId::compose(...)`, `compose_custom(...)` | `UserId::from_parts(parts)` |
| `compose_unchecked`, `compose_custom_unchecked` | Removed |
| Serde implemented for every ID with the `serde` feature | `#[derive(typedflake::Serde)]` on the IDs that want it |
| | `typedflake::SqlxPostgres` and `typedflake::Postgres` derives |
| Three crates in the workspace | `typedflake` and `typedflake-macros` |

Serde output is unchanged: IDs are still decimal strings. Input now also accepts integers.

## Keep reading existing IDs

The format of an ID type is its epoch, its bit widths, and the order of its node fields. Keep those and stored IDs keep their meaning.

### Default 0.1 IDs

The 0.1 default was a `u64` with 42 timestamp bits, 5 worker bits, 5 process bits, 12 sequence bits, and the epoch 2025-01-01. Either declaration below reads those IDs unchanged.

**Keep `u64` and the worker/process split:**

```rust
use typedflake::{TypedNode, typedflake};

#[derive(Debug, Clone, Copy, TypedNode)]
pub struct LegacyNode {
    #[node(bits = 5)]
    pub worker: u8,
    #[node(bits = 5)]
    pub process: u8,
}

#[typedflake(
    epoch = "2025-01-01",
    bits(timestamp = 42, node = 10, sequence = 12),
    node = LegacyNode,
)]
pub struct UserId(u64);
```

**Move to `i64` with the new defaults:**

```rust
use typedflake::typedflake;

#[typedflake(epoch = "2025-01-01")]
pub struct UserId(i64);
```

The bit positions are the same, with the top bit reserved as the sign. Existing IDs are accepted as long as that bit is zero, which holds until about 69.7 years after the epoch. The node is one number, equal to `worker << 5 | process`: worker 17 and process 1 become node 545.

Moving the column from an unsigned to a signed type is a change in your database, not something this crate does for you.

### Custom 0.1 configurations

Translate the old `Config` field by field:

- `epoch`: the same instant. Use `Epoch::from_date(y, m, d)`, or `Epoch::new(unix_millis)` if it was not a midnight UTC date.
- `timestamp` and `sequence`: the same widths.
- `node`: the old worker width plus the old process width.
- If you used both worker and process, declare a `TypedNode` with those two fields, worker first, so they decode separately. If you only used one, a plain `u32` node is enough.

For the removed presets, that gives:

| Preset | Epoch | Bits | Node |
| --- | --- | --- | --- |
| `Config::TWITTER` | `Epoch::new(1_288_834_974_657)` | 42 / 10 / 12 | worker 5 bits, process 5 bits |
| `Config::DISCORD` | `Epoch::from_date(2015, 1, 1)` | 42 / 10 / 12 | worker 5 bits, process 5 bits |

All 0.1 layouts used the full 64 bits, so they need `u64` unless the top bit is still zero for every ID you have and will read.

## Behavior to review

- **Startup.** `UserId::generate()` no longer falls back to worker 0 and process 0. Without `typedflake::init`, it returns `GenerateError::NotInitialized`.
- **Error handling.** Generation returns `Result`. Decide per call site whether to propagate, retry, or use a waiting variant.
- **Sequence exhaustion.** 0.1 waited for the next millisecond inside `generate()`. To keep that, call `generate_blocking(timeout)` or, in async code, `generate_async()`.
- **Clock rollback.** A clock behind the last generated ID is now an error that clears on its own once the clock catches up.
- **Validation.** Negative `i64` values and values with reserved bits are rejected everywhere, including Serde and database decoding.
