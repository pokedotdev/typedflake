# TypedFlake

**Distributed, type-safe Snowflake ID generation for Rust.**

Generate unique, time-ordered 64-bit IDs across distributed systems without coordination. Inspired by Twitter's Snowflake algorithm, with strong type safety through Rust's newtype pattern.

## Features

- **🌍 Distributed**: Multi-server support via worker/process IDs
- **🔒 Thread-safe**: Lock-free atomic operations using compare-and-swap
- **⏱️ Time-ordered**: IDs are sortable by creation time
- **🛡️ Type-safe**: Each ID type is distinct (no mixing `UserId` with `OrderId`)
- **🎛️ Customizable**: Configure bit allocation and epoch per-type or globally
- **💾 Shared state pool**: Generators share lazy state per (worker, process) pair
- **📦 Serde support**: Optional JSON serialization as strings ([IEEE 754](https://en.wikipedia.org/wiki/Double-precision_floating-point_format) safe)

## Quick Start

```rust
use typedflake::TypedFlake;

#[derive(TypedFlake)]
pub struct UserId(u64);

#[derive(TypedFlake)]
pub struct OrderId(u64);

fn main() {
    // Generate IDs (thread-safe)
    let user_id = UserId::generate();
    let order_id = OrderId::generate();

    println!("Order: {order_id}");
    println!("User: {user_id}");

    // Access components by tuple
    let (timestamp, worker_id, process_id, sequence) = user_id.decompose();
}
```

---

## Basic Generation

Generate IDs using the default instance (worker=0, process=0):

```rust
use typedflake::TypedFlake;

#[derive(TypedFlake)]
pub struct UserId(u64);

let id = UserId::generate();
```

**Multiple ID types are completely independent:**

```rust
use typedflake::TypedFlake;

#[derive(TypedFlake)]
pub struct UserId(u64);

#[derive(TypedFlake)]
pub struct OrderId(u64);

let user_id = UserId::generate();
let order_id = OrderId::generate();

// ✅ Type-safe: These cannot be accidentally mixed
fn process_user(id: UserId) { }
process_user(order_id); // ❌ Compile error!
```

## Instance-Based Generation

Create generators bound to specific worker/process IDs for distributed systems:

```rust
use typedflake::TypedFlake;

#[derive(TypedFlake)]
pub struct UserId(u64);

// Server-based: worker ID represents physical/virtual server
let server_15 = UserId::worker(15)?;
let id = server_15.generate();

// Process-based: process ID for multi-process applications
let process_7 = UserId::process(7)?;
let id = process_7.generate();

// Full control: assign both worker and process IDs
// Example: worker=region, process=datacenter
let us_east_dc2 = UserId::instance(31, 15)?;
let id = us_east_dc2.generate();
```

> [!NOTE]
> **State Sharing**: Generator instances for the same (worker_id, process_id) pair share the same underlying atomic state. This makes it safe and efficient to create multiple generators for the same IDs across different threads or contexts—they coordinate through shared state without duplication.

> [!TIP]
> For containerized deployments (Kubernetes, Docker), use [**Global Defaults**](#global-defaults) to configure worker/process IDs from environment variables. This eliminates the need to pass IDs throughout your application.

## Configuration

### Inline Attributes

Configure epoch and/or bit layout directly on the type:

```rust
use typedflake::TypedFlake;

// Custom epoch only (uses default 42|5|5|12 layout)
#[derive(TypedFlake)]
#[typedflake(epoch = "2025-09-13")]
pub struct UserId(u64);

// Custom layout only (uses default epoch)
#[derive(TypedFlake)]
#[typedflake(layout = (42, 8, 4, 10))]
pub struct OrderId(u64);

// Both layout and epoch
#[derive(TypedFlake)]
#[typedflake(layout = (42, 8, 4, 10), epoch = "2025-09-13")]
pub struct SessionId(u64);

// Epoch from milliseconds
#[derive(TypedFlake)]
#[typedflake(epoch = 1600000000000)]
pub struct LegacyId(u64);
```

### Custom Configuration via Const

For shared configs used across multiple types:

```rust
use typedflake::{TypedFlake, BitLayout, Config, Epoch};

const CUSTOM_CONFIG: Config = Config::new(
    BitLayout::new(42, 5, 5, 12),     // timestamp, worker, process, sequence
    Epoch::from_date(2025, 9, 13)     // Custom epoch date
);

#[derive(TypedFlake)]
#[typedflake(config = CUSTOM_CONFIG)]
pub struct CustomId(u64);
```

### Choosing an Epoch

**Recommended for new projects:** Set your epoch near to your project's launch date.

```rust
use typedflake::TypedFlake;

// Recommended: Set epoch near to your actual launch date
#[derive(TypedFlake)]
#[typedflake(epoch = "2025-09-13")]
pub struct MyId(u64);
```

**Why this matters:**

- ✅ Maximizes your timestamp lifespan starting from when you actually need it
- ✅ Keeps timestamp values smaller during your project's early years
- ✅ Aligns IDs with your project timeline

> [!CAUTION]
> Only change your epoch if you're absolutely certain no IDs have been generated in production yet. Otherwise, keep your current epoch—compatibility with existing IDs is more important than reclaiming unused years.

## Global Defaults

In distributed systems (microservices, Kubernetes, multi-region), each service instance typically has the same worker/process ID throughout its lifecycle. Global defaults eliminate the need to pass these IDs around—set them once at startup, then use the simple `generate()` API everywhere.

> [!IMPORTANT]
> Global defaults should be set **once at application startup** before generating any IDs. They cannot be changed after initialization.

**Without global defaults** - must create instances:

```rust
let generator = UserId::instance(worker_id, process_id)?;
let id = generator.generate(); // Repeat for every service
```

**With global defaults** - set once, use everywhere:

```rust
use typedflake::{TypedFlake, Config};

#[derive(TypedFlake)]
pub struct UserId(u64);

#[derive(TypedFlake)]
pub struct OrderId(u64);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read from environment (Kubernetes, Docker, etc.)
    let worker_id = std::env::var("POD_ORDINAL").unwrap_or("0".into()).parse()?;
    let process_id = std::env::var("CONTAINER_ID").unwrap_or("0".into()).parse()?;

    // Set defaults once at startup
    typedflake::global::set_defaults(Config::DEFAULT, worker_id, process_id)?;
    // Or set only config/instance
    typedflake::global::set_default_config(Config::DEFAULT)?;
    typedflake::global::set_default_instance(worker_id, process_id)?;

    // Simple API throughout your application
    let user_id = UserId::generate();   // Uses defaults
    let order_id = OrderId::generate(); // Uses defaults
    Ok(())
}
```

## Component Access

```rust
use typedflake::TypedFlake;

#[derive(TypedFlake)]
pub struct UserId(u64);

let id = UserId::generate();

// Decompose to tuple
let (timestamp, worker_id, process_id, sequence) = id.decompose();

// Components struct
let components = id.components();
println!("{}", components.timestamp);

// Individual accessors
let timestamp = id.timestamp();
let worker = id.worker_id();
let process = id.process_id();
let sequence = id.sequence();
```

## Composition & Conversions

### Compose IDs from Components

```rust
use typedflake::TypedFlake;

#[derive(TypedFlake)]
pub struct UserId(u64);

// Compose with default worker/process (validated)
let id = UserId::compose(1234567890, 42)?;

// Compose with all components (validated)
let id = UserId::compose_custom(1234567890, 15, 7, 42)?;

// Unchecked variants (masks overflow, better performance)
let id = UserId::compose_unchecked(1234567890, 42);
let id = UserId::compose_custom_unchecked(1234567890, 15, 7, 42);
```

### u64 Conversions

```rust
let id = UserId::generate();

// To u64
let raw: u64 = id.as_u64();
let raw: u64 = id.into();

// From u64 (validated - use for external data)
let id = UserId::try_from_u64(raw)?;
let id: UserId = raw.try_into()?;

// From u64 (unchecked - use for trusted sources)
let id = UserId::from_u64_unchecked(raw);
```

### String Conversions

```rust
let id = UserId::generate();

// To string
let s = id.to_string();
println!("ID: {s}");

// From string
let parsed: UserId = s.parse()?;
assert_eq!(id, parsed);
```

### JSON Serialization (Serde)

Enable the `serde` feature for JSON serialization:

```toml
[dependencies]
typedflake = { version = "0.1", features = ["serde"] }
```

IDs serialize as **strings** (not numbers) for safe cross-language compatibility:

```rust
use serde::{Deserialize, Serialize};

typedflake::id!(UserId);

#[derive(Serialize, Deserialize)]
struct User {
    id: UserId,
    name: String,
}

let user = User {
    id: UserId::generate(),
    name: "Alice".to_string(),
};

let json = serde_json::to_string_pretty(&user)?;
```

**JSON output:**

```json
{
  "id": "1234567890123456789",
  "name": "Alice"
}
```

> [!TIP]
> **Why strings?** JSON numbers are typically parsed as [IEEE 754 double-precision floats](https://en.wikipedia.org/wiki/Double-precision_floating-point_format), which safely represent integers up to 53 bits. Snowflake IDs are 64-bit, so values above `9_007_199_254_740_991` lose precision when parsed as numbers. String serialization ensures safe transmission across languages (JavaScript, Python, Java, Go, etc.) and web APIs without data loss.

---

## Architecture

TypedFlake uses a **newtype-driven architecture** where each ID type maintains completely independent state:

```
┌─────────────────────────────────────────┐
│ #[derive(TypedFlake)]                   │
│ pub struct UserId(u64);                 │
│                                         │
│ ┌─────────────────────────────────────┐ │
│ │ Static IdContext (OnceLock)         │ │
│ │                                     │ │
│ │ ┌─────────────┐  ┌───────────────┐  │ │
│ │ │   Config    │  │  StatePool    │  │ │
│ │ │ (BitLayout, │  │  (DashMap)    │  │ │
│ │ │   Epoch)    │  │               │  │ │
│ │ └─────────────┘  └───┬───────────┘  │ │
│ │                      │              │ │
│ │         ┌────────────┴────────┐     │ │
│ │         │ Lazy State Creation │     │ │
│ │         │ Arc<AtomicU64>      │     │ │
│ │         │ (worker, process)   │     │ │
│ │         └─────────────────────┘     │ │
│ └─────────────────────────────────────┘ │
└─────────────────────────────────────────┘
```

**Key design:**

- **Per-type isolation**: Each `#[derive(TypedFlake)] struct TypeName(u64)` creates a separate static context
- **Lock-free generation**: Atomic compare-and-swap operations on packed u64 state
- **Lazy allocation**: States created on-demand per (worker, process) pair using DashMap and shared across all generators for that pair

## ID Structure

A TypedFlake ID is a 64-bit integer divided into four components:

| Component      | Bits | Range                 | Description              |
| -------------- | ---- | --------------------- | ------------------------ |
| **Timestamp**  | 42   | 0 - 4,398,046,511,103 | Milliseconds since epoch |
| **Worker ID**  | 5    | 0 - 31                | Worker identifier        |
| **Process ID** | 5    | 0 - 31                | Process identifier       |
| **Sequence**   | 12   | 0 - 4,095             | IDs/ms (per instance)    |

**Default**: `42t|5w|5p|12s` = 139 years, 1024 instances, 4096 IDs/ms per instance

**Total system capacity** scales with instances: 1024 instances × 4096 IDs/ms = 4,194,304 IDs/ms

**Bit allocation is fully customizable:**

```rust
BitLayout::new(42, 4, 4, 14);  // High-throughput: 16,384 IDs/ms per instance
BitLayout::new(45, 4, 5, 10);  // Long-lived: ~1,115 years
```

---

## Performance

TypedFlake is designed for high-throughput scenarios:

- **Lock-free**: Atomic compare-and-swap operations with no mutexes
- **Zero allocations**: ID generation doesn't allocate memory
- **Cache-friendly**: Packed atomic state with cache-line alignment
- **Lazy initialization**: Only allocates state for actively-used instances

Run benchmarks:

```bash
cargo bench
```

---

## License

[MIT License](LICENSE)

## Acknowledgments

**Algorithm inspirations:**

- [Twitter Snowflake](https://github.com/twitter-archive/snowflake/tree/snowflake-2010) (original algorithm)
- [Discord's Snowflake implementation](https://discord.com/developers/docs/reference#snowflakes)

**Design philosophy:**

- [The Ultimate Guide to Rust Newtypes](https://www.howtocodeit.com/articles/ultimate-guide-rust-newtypes)
