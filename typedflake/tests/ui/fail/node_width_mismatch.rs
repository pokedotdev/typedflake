use typedflake::{BitLayout, Epoch, Format, TypedNode, typedflake};

#[derive(Debug, Clone, Copy, TypedNode)]
struct EightBitNode {
    #[node(bits = 4)]
    worker: u8,
    #[node(bits = 4)]
    process: u8,
}

const APP_IDS: Format = Format {
    epoch: Epoch::from_date(2025, 1, 1),
    bits: BitLayout {
        timestamp: 41,
        node: 10,
        sequence: 12,
    },
};

#[typedflake(format = APP_IDS, node = EightBitNode)]
struct SharedFormat(i64);

fn main() {}
