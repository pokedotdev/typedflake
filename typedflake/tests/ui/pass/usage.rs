//! The primary usage patterns from the design, compiled together.


use typedflake::{BitLayout, Epoch, Format, Generator, Parts, TypedNode, typedflake};

pub const APP_IDS: Format = Format {
    epoch: Epoch::from_date(2025, 1, 1),
    bits: BitLayout {
        timestamp: 41,
        node: 10,
        sequence: 12,
    },
};

pub const SHARED_BITS: BitLayout = BitLayout {
    timestamp: 42,
    node: 10,
    sequence: 12,
};

pub const WIDE_IDS: Format = Format {
    epoch: Epoch::new(1_735_689_600_000),
    bits: SHARED_BITS,
};

#[derive(Debug, Clone, Copy, TypedNode)]
pub struct AppNode {
    #[node(bits = 5)]
    pub worker: u8,
    #[node(bits = 5)]
    pub process: u8,
}

mod ids {
    use super::*;

    #[typedflake(epoch = "2025-01-01")]
    pub struct UserId(i64);

    #[typedflake(epoch = "2025-01-01")]
    pub struct OrderId(i64);

    #[typedflake(format = APP_IDS)]
    pub struct InvoiceId(i64);

    #[typedflake(format = WIDE_IDS)]
    pub(crate) struct EventId(u64);

    #[typedflake(
        epoch = "2025-01-01",
        bits(timestamp = 41, node = 10, sequence = 12),
    )]
    pub struct InlineId(i64);

    #[typedflake(format = APP_IDS, node = AppNode)]
    pub struct TypedId(i64);

    /// Documented, with unrelated attributes and derives.
    #[typedflake(epoch = "2025-01-01", bits(timestamp = 32, node = 5, sequence = 8))]
    #[derive(Default)]
    #[allow(dead_code)]
    pub struct ReducedId(i64);
}

use ids::*;

fn simple() -> Result<(), Box<dyn std::error::Error>> {
    typedflake::init(17)?;

    let user_id = UserId::generate()?;
    let _order_id = OrderId::generate()?;
    let _waited = UserId::generate_blocking()?;

    let raw: i64 = user_id.get();
    let restored = UserId::try_from(raw)?;
    let raw: i64 = restored.into();
    let _from_unsigned = UserId::try_from(raw as u64)?;
    let parsed: UserId = "123456789".parse()?;
    let _text = parsed.to_string();

    let generator: Generator<UserId> = UserId::generator(17)?;
    let _id = generator.generate()?;
    let _shared = generator.clone();

    let parts: Parts<u32> = user_id.parts();
    println!("{} {} {}", parts.elapsed_millis, parts.node, parts.sequence);
    let _rebuilt = UserId::from_parts(parts)?;
    let _unix_millis: u64 = user_id.unix_millis()?;

    let _event = EventId::try_from(u64::MAX)?;
    let _invoice = InvoiceId::try_from(1_i64)?;
    let _inline = InlineId::try_from(1_i64)?;
    let _reduced = ReducedId::default();
    Ok(())
}

fn typed() -> Result<(), Box<dyn std::error::Error>> {
    typedflake::init(AppNode {
        worker: 17,
        process: 1,
    })?;

    let generator = TypedId::generator(AppNode {
        worker: 17,
        process: 1,
    })?;
    let id = generator.generate()?;

    let parts = id.parts();
    println!("{} {}", parts.node.worker, parts.node.process);
    let _rebuilt = TypedId::from_parts(parts)?;
    Ok(())
}

fn main() {
    let _ = simple();
    let _ = typed();
}
