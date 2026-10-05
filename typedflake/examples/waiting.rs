//! Wait for sequence capacity instead of handling exhaustion yourself.
//!
//! Run with `cargo run -p typedflake --example waiting --features tokio`.

use std::time::Duration;

use typedflake::{GenerateError, typedflake};

/// Only four IDs per millisecond, so this example runs out quickly.
#[typedflake(epoch = "2025-01-01", bits(timestamp = 41, node = 10, sequence = 2))]
pub struct TicketId(i64);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    typedflake::init(1)?;

    // `generate` never waits: once the millisecond is full it reports it.
    let exhausted =
        (0..1_000).any(|_| matches!(TicketId::generate(), Err(GenerateError::SequenceExhausted)));
    println!("ran out of sequence numbers: {exhausted}");

    // Blocking code waits with a time budget.
    let id = TicketId::generate_blocking(Duration::from_millis(20))?;
    println!("blocking: {id}");

    // Async code waits on a Tokio timer, bounded by a normal Tokio timeout.
    let id = tokio::time::timeout(Duration::from_millis(20), TicketId::generate_async()).await??;
    println!("async:    {id}");

    Ok(())
}
