use typedflake::{BitLayout, Epoch, Format, typedflake};

const WIDE: Format = Format {
    epoch: Epoch::from_date(2025, 1, 1),
    bits: BitLayout {
        timestamp: 42,
        node: 10,
        sequence: 12,
    },
};

#[typedflake(format = WIDE)]
struct Fits(u64);

#[typedflake(format = WIDE)]
struct DoesNotFit(i64);

fn main() {}
