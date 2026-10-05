use typedflake::{TypedNode, typedflake};

#[derive(Debug, Clone, Copy, TypedNode)]
struct EightBitNode {
    #[node(bits = 4)]
    worker: u8,
    #[node(bits = 4)]
    process: u8,
}

#[typedflake(epoch = "2025-01-01", bits(timestamp = 41, node = 8, sequence = 12), node = EightBitNode)]
struct Matches(i64);

#[typedflake(epoch = "2025-01-01", node = EightBitNode)]
struct DefaultBits(i64);

fn main() {}
