# TypedFlake

[![Crates.io](https://img.shields.io/crates/v/typedflake.svg)](https://crates.io/crates/typedflake)
[![Documentation](https://docs.rs/typedflake/badge.svg)](https://docs.rs/typedflake)

Snowflake-style IDs for Rust, where every kind of ID is its own type.

- **Distinct types.** `UserId` and `OrderId` cannot be mixed up, and each has its own generation state.
- **Validated values.** Every way into an ID (integers, strings, JSON, database rows) applies the same checks. There is no unchecked constructor.
- **Database-friendly.** IDs are `i64` by default and never negative, so they fit a `BIGINT` column as they are.
- **Declarative formats.** Epoch and bit allocation are constants checked at compile time.
- **Opt-in integrations.** Serde, SQLx (PostgreSQL), and `postgres-types` are separate derives behind Cargo features.

## Quick start

```toml
[dependencies]
typedflake = "0.2"
```

```rust
use typedflake::typedflake;

#[typedflake(epoch = "2025-01-01")]
pub struct UserId(i64);

#[typedflake(epoch = "2025-01-01")]
pub struct OrderId(i64);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Once, during application startup.
    typedflake::init(17)?;

    // Anywhere in the application.
    let user_id = UserId::generate()?;
    let order_id = OrderId::generate()?;

    println!("{user_id} {order_id}");
    Ok(())
}
```

`17` is this process's **node**: a number that must be unique among the running processes that generate the same ID types. TypedFlake does not assign nodes; take them from your deployment (a replica index, a configured value, a lease).

## Declaring IDs

`#[typedflake(...)]` goes on a tuple struct with one private `i64` or `u64` field, above any `#[derive]`. It implements `Debug`, `Display`, `FromStr`, `Clone`, `Copy`, equality, ordering, and hashing, so those must not be derived again. Other derives and attributes are fine.

An ID packs three fields, from most to least significant:

```text
[ timestamp ][ node ][ sequence ]
```

- **timestamp**: milliseconds since the epoch you choose.
- **node**: which process generated the ID.
- **sequence**: a counter within one millisecond.

### Epoch

The epoch is required and is part of the stored format: changing it changes the meaning of every existing ID. Pick a date shortly before your first ID and never change it.

```rust
use typedflake::typedflake;

#[typedflake(epoch = "2025-01-01")]
pub struct UserId(i64);
```

The string is a `YYYY-MM-DD` date at midnight UTC. For any other instant, use a shared format with `Epoch::new(unix_millis)`.

### Bits

Without `bits`, an ID has 10 node bits (1024 nodes), 12 sequence bits (4096 IDs per millisecond per node), and the rest for the timestamp.

| Integer | Timestamp bits | Lasts | Range |
| --- | --- | --- | --- |
| `i64` | 41 | about 69.7 years | `0..=i64::MAX` |
| `u64` | 42 | about 139.4 years | `0..=u64::MAX` |

Prefer `i64` unless you need the extra bit: most databases only have signed 64-bit integers.

To choose the widths yourself, give all three:

```rust
use typedflake::typedflake;

#[typedflake(
    epoch = "2025-01-01",
    bits(timestamp = 43, node = 8, sequence = 12),
)]
pub struct EventId(i64);
```

Timestamp and sequence need at least one bit; node may have zero. The total may be less than the integer holds, which gives smaller numbers in exchange for capacity. The unused upper bits are reserved: an ID with any of them set is rejected.

### Shared formats

Several ID types can share one format constant:

```rust
use typedflake::{BitLayout, Epoch, Format, typedflake};

pub const APP_IDS: Format = Format {
    epoch: Epoch::from_date(2025, 1, 1),
    bits: BitLayout {
        timestamp: 41,
        node: 10,
        sequence: 12,
    },
};

#[typedflake(format = APP_IDS)]
pub struct UserId(i64);

#[typedflake(format = APP_IDS)]
pub struct OrderId(i64);
```

Formats are validated when an ID type uses them. An invalid one, such as 64 bits for an `i64`, is a compile error at the declaration.

## Nodes

A node is either a plain number or a struct that splits the node bits into named fields.

```rust
use typedflake::{TypedNode, typedflake};

#[derive(Debug, Clone, Copy, TypedNode)]
pub struct AppNode {
    #[node(bits = 5)]
    pub worker: u8,
    #[node(bits = 5)]
    pub process: u8,
}

#[typedflake(epoch = "2025-01-01", node = AppNode)]
pub struct UserId(i64);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    typedflake::init(AppNode {
        worker: 17,
        process: 1,
    })?;

    let id = UserId::generate()?;
    assert_eq!(id.parts().node.worker, 17);
    Ok(())
}
```

Fields are `u8`, `u16`, or `u32`, pack in declaration order (most significant first), and must add up to the format's node bits. Their order is part of the stored format.

Node values are checked when they are used: a number or field too large for its bits is an error, never truncated.

## Generating IDs

### The default node

`typedflake::init(node)` installs the node used by every `Id::generate()` call. Call it once at startup; a second call returns `InitError::AlreadyInitialized`, and generating before it returns `GenerateError::NotInitialized`.

One default serves all ID types with the same kind of node. An ID declared with a different node type reports a mismatch instead of silently using it; give that ID an explicit generator.

In tests, where many tests share a process, ignore the repeat error: `let _ = typedflake::init(0);`.

### Explicit generators

A generator is bound to one node and needs no global initialization:

```rust
use typedflake::typedflake;

#[typedflake(epoch = "2025-01-01")]
pub struct UserId(i64);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let generator = UserId::generator(17)?;

    let first = generator.generate()?;
    let second = generator.clone().generate()?;
    assert!(first < second);
    Ok(())
}
```

Generators for the same ID type and node always share their state, whether cloned, created again, or reached through `UserId::generate()`. There is no way to get two that could issue the same ID.

### Errors and waiting

`generate()` never waits. It fails when:

- the millisecond's sequence is used up (`SequenceExhausted`): retry on the next millisecond;
- the clock is behind the last ID generated (`ClockRollback`): generation resumes by itself when the clock catches up;
- the clock is before the epoch, or past what the timestamp bits can hold.

To wait for sequence capacity instead of handling it:

```rust
use std::time::Duration;
use typedflake::typedflake;

#[typedflake(epoch = "2025-01-01")]
pub struct UserId(i64);

async fn create_ids() -> Result<(), Box<dyn std::error::Error>> {
    // Blocking, with a time budget.
    let id = UserId::generate_blocking(Duration::from_millis(20))?;

    // On a Tokio timer (feature `tokio`). Bound it with `tokio::time::timeout`.
    let id = UserId::generate_async().await?;
    Ok(())
}
```

Both retry only sequence exhaustion. Generators have the same two methods.

## Working with IDs

```rust
use typedflake::{Parts, typedflake};

#[typedflake(epoch = "2025-01-01")]
pub struct UserId(i64);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let id = UserId::generator(17)?.generate()?;

    // Integers.
    let raw: i64 = id.get();
    let raw: i64 = id.into();
    let restored = UserId::try_from(raw)?;

    // Decimal strings.
    let text = id.to_string();
    let parsed: UserId = text.parse()?;

    // Parts.
    let parts: Parts<u32> = id.parts();
    println!("{} {} {}", parts.elapsed_millis, parts.node, parts.sequence);
    let rebuilt = UserId::from_parts(parts)?;
    let created_at: u64 = id.unix_millis()?;

    assert!(restored == id && parsed == id && rebuilt == id);
    Ok(())
}
```

`TryFrom<i64>` and `TryFrom<u64>` exist for every ID and reject negative values and reserved bits. There is no `From<i64>`, no arithmetic, and no `Deref`: an ID is not a number you compute with.

None of these operations touch the clock or need `init`.

Every method is inherent, so nothing needs importing. For code generic over ID types, the same methods are on the `typedflake::Id` trait.

## Integrations

Each integration is a Cargo feature plus a derive on the IDs that want it.

| Derive | Feature | Implements |
| --- | --- | --- |
| `typedflake::Serde` | `serde` | Serde `Serialize` and `Deserialize` |
| `typedflake::SqlxPostgres` | `sqlx-postgres` | SQLx 0.8 `Type`, `Encode`, `Decode`, and arrays for PostgreSQL |
| `typedflake::Postgres` | `postgres` | `postgres-types` 0.2 `ToSql` and `FromSql` |

The `tokio` feature adds `generate_async`.

Use these instead of the libraries' own derives: a derived `Deserialize` or `sqlx::Type` would build an ID from any integer and skip validation, so `#[typedflake]` rejects them.

### Serde

```rust
use serde::{Deserialize, Serialize};
use typedflake::typedflake;

#[typedflake(epoch = "2025-01-01")]
#[derive(typedflake::Serde)]
pub struct UserId(i64);

#[derive(Serialize, Deserialize)]
struct User {
    id: UserId,
    name: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let user = User {
        id: UserId::try_from(9_007_199_254_740_993_i64)?,
        name: "Alice".to_owned(),
    };
    assert_eq!(
        serde_json::to_string(&user)?,
        r#"{"id":"9007199254740993","name":"Alice"}"#
    );

    // Strings and integers are both accepted on input.
    let user: User = serde_json::from_str(r#"{"id":42,"name":"Bob"}"#)?;
    assert_eq!(user.id.get(), 42);
    Ok(())
}
```

IDs are written as strings because JSON numbers lose precision above 2^53.

### SQLx

```rust
use typedflake::typedflake;

#[typedflake(epoch = "2025-01-01")]
#[derive(typedflake::SqlxPostgres)]
pub struct UserId(i64);

#[derive(sqlx::FromRow)]
struct User {
    id: UserId,
    name: String,
}

async fn create_user(pool: &sqlx::PgPool, name: &str) -> Result<User, Box<dyn std::error::Error>> {
    let id = UserId::generate()?;

    sqlx::query("INSERT INTO users (id, name) VALUES ($1, $2)")
        .bind(id)
        .bind(name)
        .execute(pool)
        .await?;

    let user = sqlx::query_as("SELECT id, name FROM users WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await?;
    Ok(user)
}
```

IDs are stored as `BIGINT`. Reading a row validates the value, so a negative or out-of-range number in the column is a decode error. SQLx's compile-time `query!` macros need their usual type override (`id AS "id: UserId"`).

### postgres-types

```rust
use typedflake::typedflake;

#[typedflake(epoch = "2025-01-01")]
#[derive(typedflake::Postgres)]
pub struct UserId(i64);

async fn create_user(
    client: &tokio_postgres::Client,
    name: &str,
) -> Result<UserId, Box<dyn std::error::Error>> {
    let id = UserId::generate()?;

    client
        .execute("INSERT INTO users (id, name) VALUES ($1, $2)", &[&id, &name])
        .await?;

    let row = client
        .query_one("SELECT id FROM users WHERE id = $1", &[&id])
        .await?;
    Ok(row.try_get("id")?)
}
```

This works with `tokio-postgres` and the synchronous `postgres` client.

Both database derives support `i64` IDs only. A `u64` ID has no PostgreSQL integer that holds all its values; convert it yourself if you need to store one.

[`examples/axum-sqlx`](examples/axum-sqlx) is a small HTTP service that uses Serde and SQLx together.

## What is and is not guaranteed

- **Uniqueness needs unique nodes.** Two processes generating the same ID type with the same node can produce the same ID.
- **Uniqueness does not survive a clock that goes backwards across a restart.** State is in memory: within a process a rollback is detected and reported, but a restarted process cannot know the clock was once ahead.
- **Different ID types can have equal numbers.** The type is not encoded in the value.
- **Ordering is approximate.** IDs from one node increase over time. IDs from different nodes are ordered by millisecond, then by node.
- **A valid ID is not proof of anything.** Parsing checks the format, not that the ID was ever generated or exists in your database.

## Migrating from 0.1

0.2 replaces `#[derive(TypedFlake)]` and `id!` with `#[typedflake]`, and the worker/process pair with a single node. See [`MIGRATION.md`](MIGRATION.md) for the API changes and for declarations that keep reading existing 0.1 IDs.

## Performance

Measured with `cargo bench -p typedflake` on an AMD Ryzen 7 5700G with Rust 1.99. Treat them as orders of magnitude, not promises.

| Operation | Time |
| --- | --- |
| `UserId::generate()` | 38 ns |
| `generator.generate()` | 38 ns |
| `generator.generate()`, 4 threads sharing one node | 63 ns |
| `try_from(i64)` | under 1 ns |
| `parts()` / `from_parts()` | about 1 ns |
| `to_string()` | 25 ns |
| `parse()` | 12 ns |

Generation is one clock read and one atomic compare-and-swap, with no allocation or lookup per ID. The default layout caps a node at 4096 IDs per millisecond for each ID type, which is far below what the CPU can produce: under sustained load the limit you meet is the sequence, not the generator.

## Minimum supported Rust version

Rust 1.99.

## License

[MIT License](LICENSE)

## Acknowledgments

**Algorithm inspirations:**

- [Twitter Snowflake](https://github.com/twitter-archive/snowflake/tree/snowflake-2010) (original algorithm)
- [Discord's Snowflake implementation](https://discord.com/developers/docs/reference#snowflakes)

**Design philosophy:**

- [The Ultimate Guide to Rust Newtypes](https://www.howtocodeit.com/articles/ultimate-guide-rust-newtypes)
