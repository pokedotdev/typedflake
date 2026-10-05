use typedflake::{BitLayout, Epoch, Format, typedflake};

const APP_IDS: Format = Format {
    epoch: Epoch::from_date(2025, 1, 1),
    bits: BitLayout {
        timestamp: 41,
        node: 10,
        sequence: 12,
    },
};

#[typedflake]
struct NoOptions(i64);

#[typedflake(bits(timestamp = 41, node = 10, sequence = 12))]
struct BitsWithoutEpoch(i64);

#[typedflake(format = APP_IDS, epoch = "2025-01-01")]
struct FormatWithEpoch(i64);

#[typedflake(format = APP_IDS, bits(timestamp = 41, node = 10, sequence = 12))]
struct FormatWithBits(i64);

#[typedflake(epoch = "2025-01-01", epoch = "2025-01-02")]
struct DuplicateEpoch(i64);

#[typedflake(epoch = "2025-01-01", layout = (41, 10, 12))]
struct UnknownOption(i64);

#[typedflake(
    epoch = "2025-01-01",
    alphabet = typedflake::Alphabet::BASE62,
    alphabet = typedflake::Alphabet::BASE58,
)]
struct DuplicateAlphabet(i64);

#[typedflake(epoch = "2025-01-01", alphabet = "0123456789")]
struct AlphabetNotConstant(i64);

#[typedflake(epoch = "2025-01-01" node = u32)]
struct MissingComma(i64);

#[typedflake(epoch = "2025-01-01", bits = (41, 10, 12))]
struct BitsAssigned(i64);

#[typedflake(epoch = "2025-01-01", bits(41, 10, 12))]
struct BitsPositional(i64);

#[typedflake(epoch = "2025-01-01", bits(timestamp = 41, node = 10))]
struct BitsIncomplete(i64);

#[typedflake(epoch = "2025-01-01", bits(timestamp = 41, node = 10, sequence = 12, worker = 5))]
struct BitsUnknownField(i64);

#[typedflake(epoch = "2025-01-01", bits(timestamp = 41, node = 10, node = 10, sequence = 2))]
struct BitsDuplicateField(i64);

#[typedflake(epoch = "2025-01-01", bits(timestamp = 42, node = 10, sequence = 12))]
struct TooWideForSigned(i64);

#[typedflake(epoch = "2025-01-01", bits(timestamp = 43, node = 10, sequence = 12))]
struct TooWideForUnsigned(u64);

#[typedflake(epoch = "2025-01-01", bits(timestamp = 0, node = 10, sequence = 12))]
struct ZeroTimestamp(i64);

#[typedflake(epoch = "2025-01-01", bits(timestamp = 41, node = 10, sequence = 0))]
struct ZeroSequence(i64);

#[typedflake(epoch = "2025-01-01", bits(timestamp = 300, node = 10, sequence = 12))]
struct WidthNotU8(i64);

fn main() {}
