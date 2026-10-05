# TypedFlake

[![Crates.io](https://img.shields.io/crates/v/typedflake?style=flat-square&logo=rust)](https://crates.io/crates/typedflake)
[![Documentation](https://img.shields.io/docsrs/typedflake?style=flat-square&logo=docs.rs)](https://docs.rs/typedflake)
[![MSRV](https://img.shields.io/crates/msrv/typedflake?style=flat-square)](https://crates.io/crates/typedflake)

Snowflake-style IDs for Rust, where every kind of ID is its own type.

- **One type per ID.** `UserId` and `OrderId` cannot be mixed up.
- **Sortable by time.** Newer IDs are larger.
- **Fits your database.** IDs are `i64` and never negative, ready for a `BIGINT` column.
- **Works across servers.** Each process generates its own IDs, with no coordination.
- **Custom formats.** Choose the epoch and the bit layout, checked at compile time.
- **Text encoding.** Base 62 for URLs, or any alphabet you define.
- **Optional integrations.** Serde, SQLx (PostgreSQL), and `postgres-types`.

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

`17` is the **node**: a number that identifies this process. Give each running instance of your application a different one (a replica index, a configured value), so two instances never produce the same ID.

## Declaring IDs

Put `#[typedflake(...)]` on a tuple struct with one private `i64` or `u64` field, above any `#[derive]`. The type already comes with `Debug`, `Display`, `FromStr`, `Clone`, `Copy`, comparison, and hashing, so do not derive those again.

An ID packs three fields into one integer:

```text
[ timestamp ][ node ][ sequence ]
```

- **timestamp**: milliseconds since your epoch.
- **node**: which process generated the ID.
- **sequence**: a counter within one millisecond.

### Epoch

The epoch is the date your timestamps start counting from. Pick a date shortly before your first ID and never change it, because existing IDs depend on it.

It is written as a `YYYY-MM-DD` date, at midnight UTC.

### Bits

By default an ID has 10 node bits (1024 nodes), 12 sequence bits (4096 IDs per millisecond per node), and the rest for the timestamp:

| Integer | Timestamp bits | Lasts |
| --- | --- | --- |
| `i64` | 41 | about 69 years |
| `u64` | 42 | about 139 years |

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

The total may be less than the integer holds, which gives shorter numbers in exchange for capacity.

### Shared formats

Several ID types can share one format:

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

A format that does not fit, such as 64 bits in an `i64`, is a compile error.

## Nodes

A node can be a plain number, as in the quick start, or a struct that splits the node bits into named fields:

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

Fields are `u8`, `u16`, or `u32`, and their bits must add up to the format's node bits. A value too large for its bits is an error.

## Generating IDs

### With a default node

Call `typedflake::init(node)` once at startup, then use `generate()` anywhere:

```rust
use typedflake::typedflake;

#[typedflake(epoch = "2025-01-01")]
pub struct UserId(i64);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    typedflake::init(17)?;

    let id = UserId::generate()?;
    Ok(())
}
```

If some IDs use a plain number and others a `TypedNode` struct, call `init` once for each kind of node.

Calling `init` twice with the same kind of node is an error. Tests that share a process can ignore it: `let _ = typedflake::init(0);`.

### With an explicit generator

A generator carries its own node and does not need `init`:

```rust
use typedflake::typedflake;

#[typedflake(epoch = "2025-01-01")]
pub struct UserId(i64);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let generator = UserId::generator(17)?;

    let first = generator.generate()?;
    let second = generator.generate()?;
    assert!(first < second);
    Ok(())
}
```

Generators are cheap to clone and safe to share between threads.

### When generation fails

`generate()` returns immediately and never waits. It returns an error when:

- all the IDs for the current millisecond are used (`SequenceExhausted`);
- the system clock went backwards (`ClockRollback`); it works again once the clock catches up;
- the clock is before the epoch, or the timestamp bits have run out.

To wait for the next millisecond instead of handling `SequenceExhausted` yourself:

```rust
use typedflake::typedflake;

#[typedflake(epoch = "2025-01-01")]
pub struct UserId(i64);

async fn create_ids() -> Result<(), Box<dyn std::error::Error>> {
    // Blocks the thread until the next millisecond.
    let id = UserId::generate_blocking()?;

    // Waits on a Tokio timer (feature `tokio`).
    let id = UserId::generate_async().await?;
    Ok(())
}
```

## Working with IDs

```rust
use typedflake::typedflake;

#[typedflake(epoch = "2025-01-01")]
pub struct UserId(i64);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let id = UserId::generator(17)?.generate()?;

    // To and from integers.
    let raw: i64 = id.get();
    let raw: i64 = id.into();
    let restored = UserId::try_from(raw)?;

    // To and from strings.
    let text = id.to_string();
    let parsed: UserId = text.parse()?;

    // Look inside.
    let parts = id.parts();
    println!("{} {} {}", parts.elapsed_millis, parts.node, parts.sequence);
    let rebuilt = UserId::from_parts(parts)?;

    // When it was created, as Unix milliseconds.
    let created_at: u64 = id.unix_millis()?;

    assert!(restored == id && parsed == id && rebuilt == id);
    Ok(())
}
```

Converting from an integer or a string checks that the value is a valid ID for that type, so it returns a `Result`.

To write code that is generic over ID types, use the `typedflake::Id` trait.

## Encoding IDs as text

An ID is a number, and `to_string()` writes it in decimal: up to 19 digits. For URLs, share links, or codes people type, give the type an **alphabet** and it gains `encode()` and `decode()`:

```rust
use typedflake::{Alphabet, typedflake};

#[typedflake(epoch = "2025-01-01", alphabet = Alphabet::BASE62)]
pub struct UserId(i64);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let id = UserId::try_from(232_900_560_974_681_078_i64)?;

    // No allocation; use it as a `&str` or print it.
    let text = id.encode();
    assert_eq!(text.as_str(), "0HCgayuFvMs");
    println!("https://example.com/users/{text}");

    // Checked like every other way of building an ID.
    assert_eq!(UserId::decode("0HCgayuFvMs")?, id);
    assert!(UserId::decode("not-an-id").is_err());
    Ok(())
}
```

- **Fixed length.** Every ID of a type encodes to the same number of characters.
- **One text per ID.** `decode()` only accepts exactly what `encode()` writes.
- **Only for text.** `to_string()`, Serde, and the database keep using the number.

### Built-in alphabets

| Alphabet | Characters | Length of an `i64` ID |
| --- | --- | --- |
| `Alphabet::BASE36` | `0-9 a-z` | 13 |
| `Alphabet::BASE58` | Bitcoin's: no `0`, `O`, `I`, or `l` | 11 |
| `Alphabet::BASE62` | `0-9 A-Z a-z` | 11 |
| `Alphabet::BASE64_URL` | `- 0-9 A-Z _ a-z` | 11 |

With these, encoded IDs sort the same way the IDs do. `BASE64_URL` has the characters of base64url in a different order: it is not base64, and its IDs can start with `-`.

### Your own alphabet

Any 2 to 94 different ASCII characters work. The first one is the digit zero:

```rust
use typedflake::{Alphabet, typedflake};

// Digits and lowercase letters, without the easily confused `0`, `1`, `i`, `l`, and `o`.
pub const FRIENDLY: Alphabet = Alphabet::new("23456789abcdefghjkmnpqrstuvwxyz");

#[typedflake(epoch = "2025-01-01", alphabet = FRIENDLY)]
pub struct InviteId(i64);
```

A repeated or unsupported character is a compile error. Fewer characters make longer IDs, and encoded IDs sort like the IDs only if the characters are listed in ASCII order.

To write code that is generic over encoded IDs, use the `typedflake::EncodedId` trait.

## Integrations

Each integration is a Cargo feature plus a derive on the IDs that need it:

| Derive | Feature | For |
| --- | --- | --- |
| `typedflake::Serde` | `serde` | Serde |
| `typedflake::SqlxPostgres` | `sqlx-postgres` | SQLx 0.8 with PostgreSQL |
| `typedflake::Postgres` | `postgres` | `postgres-types` 0.2 (`tokio-postgres`, `postgres`) |

The `tokio` feature adds `generate_async`.

Use these derives instead of `serde::Deserialize`, `sqlx::Type`, and similar: they check every value they read.

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

IDs are written as strings because JavaScript and many JSON parsers lose precision on large numbers.

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

IDs are stored as `BIGINT`. With the `query!` macros, use SQLx's usual type override: `id AS "id: UserId"`.

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

Both database derives work with `i64` IDs only, since PostgreSQL has no unsigned 64-bit integer.

[`examples/axum-sqlx`](examples/axum-sqlx) is a small HTTP service that uses Serde and SQLx together.

## Good to know

- **Each process needs its own node.** Two processes generating the same ID type with the same node can produce the same ID.
- **Keep the system clock in sync.** A clock that jumps backwards while the process is stopped can lead to repeated IDs after it restarts.
- **IDs are unique per type.** A `UserId` and an `OrderId` can have the same number.
- **IDs sort roughly by creation time.** Exactly within one node, and to the millisecond across nodes.

## Migrating from 0.1

See [`MIGRATION.md`](MIGRATION.md) for the API changes and for how to keep reading existing 0.1 IDs.

## Performance

Measured with `cargo bench -p typedflake --bench performance` on an AMD Ryzen 7 5700G:

| Operation | Time |
| --- | --- |
| `generate()` | 35 ns |
| `generate()`, 4 threads sharing one node | 18 ns |
| `try_from(i64)` | under 1 ns |
| `to_string()` | 25 ns |
| `parse()` | 12 ns |
| `encode()`, base 62 | 8 ns |
| `decode()`, base 62 | 14 ns |

## License

[MIT License](LICENSE)

## Acknowledgments

**Algorithm inspirations:**

- [Twitter Snowflake](https://github.com/twitter-archive/snowflake/tree/snowflake-2010) (original algorithm)
- [Discord's Snowflake implementation](https://discord.com/developers/docs/reference#snowflakes)

**Design philosophy:**

- [The Ultimate Guide to Rust Newtypes](https://www.howtocodeit.com/articles/ultimate-guide-rust-newtypes)
