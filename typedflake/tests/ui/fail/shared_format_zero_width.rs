use typedflake::{BitLayout, Epoch, Format, typedflake};

const NO_SEQUENCE: Format = Format {
    epoch: Epoch::from_date(2025, 1, 1),
    bits: BitLayout {
        timestamp: 41,
        node: 10,
        sequence: 0,
    },
};

#[typedflake(format = NO_SEQUENCE)]
struct UserId(i64);

fn main() {}
