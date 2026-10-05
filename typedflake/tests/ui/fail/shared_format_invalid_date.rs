use typedflake::{BitLayout, Epoch, Format};

const INVALID: Format = Format {
    epoch: Epoch::from_date(2025, 2, 30),
    bits: BitLayout {
        timestamp: 41,
        node: 10,
        sequence: 12,
    },
};

fn main() {
    let _ = INVALID;
}
