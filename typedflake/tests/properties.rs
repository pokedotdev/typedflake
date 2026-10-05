//! Property tests over the pure value model, including extreme legal widths.

use proptest::prelude::*;
use typedflake::{Id, InvalidId, TypedNode, typedflake};

#[derive(Debug, Clone, Copy, PartialEq, Eq, TypedNode)]
pub struct SplitNode {
    #[node(bits = 3)]
    pub region: u8,
    #[node(bits = 9)]
    pub rack: u16,
    #[node(bits = 20)]
    pub host: u32,
}

#[typedflake(epoch = "2025-01-01")]
pub struct SignedId(i64);

#[typedflake(epoch = "2025-01-01")]
pub struct UnsignedId(u64);

#[typedflake(epoch = "2025-01-01", bits(timestamp = 32, node = 5, sequence = 8))]
pub struct ReducedId(i64);

#[typedflake(epoch = "2025-01-01", bits(timestamp = 1, node = 0, sequence = 1))]
pub struct MinimalId(i64);

#[typedflake(epoch = "2025-01-01", bits(timestamp = 63, node = 0, sequence = 1))]
pub struct WideTimestampId(u64);

#[typedflake(epoch = "2025-01-01", bits(timestamp = 1, node = 0, sequence = 63))]
pub struct WideSequenceId(u64);

#[typedflake(epoch = "2025-01-01", bits(timestamp = 1, node = 32, sequence = 30))]
pub struct WideNodeId(i64);

#[typedflake(
    epoch = "2025-01-01",
    bits(timestamp = 20, node = 32, sequence = 11),
    node = SplitNode,
)]
pub struct SplitId(i64);

fn total_bits<I: Id>() -> u32 {
    let bits = I::FORMAT.bits;
    u32::from(bits.timestamp) + u32::from(bits.node) + u32::from(bits.sequence)
}

fn raw_max<I: Id>() -> u64 {
    u64::MAX >> (64 - total_bits::<I>())
}

/// Checks every pure operation on one ID type for an arbitrary `u64`.
fn check<I>(seed: u64) -> Result<(), TestCaseError>
where
    I: Id + TryFrom<u64, Error = InvalidId> + TryFrom<i64, Error = InvalidId>,
    I: std::fmt::Debug + std::fmt::Display + std::str::FromStr + PartialEq + Ord,
    I::Node: std::fmt::Debug + PartialEq,
    I::Repr: Into<i128>,
    <I as std::str::FromStr>::Err: std::fmt::Debug,
{
    let max = raw_max::<I>();

    // Values above the format's range are rejected, never truncated.
    if seed > max {
        prop_assert_eq!(
            I::try_from(seed),
            Err(InvalidId::ReservedBits { value: seed, max })
        );
    }

    let raw = seed & max;
    let id = I::try_from(raw).unwrap();
    let repr: i128 = id.get().into();
    prop_assert_eq!(repr, i128::from(raw));

    // Parts round trip and each stays within its width.
    let bits = I::FORMAT.bits;
    let parts = id.parts();
    prop_assert!(parts.elapsed_millis <= u64::MAX >> (64 - u32::from(bits.timestamp)));
    prop_assert!(parts.sequence <= u64::MAX >> (64 - u32::from(bits.sequence)));
    prop_assert_eq!(I::from_parts(parts), Ok(id));

    // Strings round trip.
    prop_assert_eq!(id.to_string(), raw.to_string());
    prop_assert_eq!(id.to_string().parse::<I>().unwrap(), id);

    // Signed input agrees with unsigned input, and negatives are rejected.
    if let Ok(signed) = i64::try_from(raw) {
        prop_assert_eq!(I::try_from(signed), Ok(id));
    }
    let negative = -1 - i64::try_from(seed >> 1).unwrap();
    prop_assert_eq!(
        I::try_from(negative),
        Err(InvalidId::Negative { value: negative })
    );

    // Ordering follows the raw value.
    let other = I::try_from(seed.rotate_left(17) & max).unwrap();
    let other_repr: i128 = other.get().into();
    prop_assert_eq!(id.cmp(&other), repr.cmp(&other_repr));

    Ok(())
}

proptest! {
    #[test]
    fn signed_default(seed: u64) { check::<SignedId>(seed)?; }

    #[test]
    fn unsigned_default(seed: u64) { check::<UnsignedId>(seed)?; }

    #[test]
    fn reduced(seed: u64) { check::<ReducedId>(seed)?; }

    #[test]
    fn minimal(seed: u64) { check::<MinimalId>(seed)?; }

    #[test]
    fn wide_timestamp(seed: u64) { check::<WideTimestampId>(seed)?; }

    #[test]
    fn wide_sequence(seed: u64) { check::<WideSequenceId>(seed)?; }

    #[test]
    fn wide_node(seed: u64) { check::<WideNodeId>(seed)?; }

    #[test]
    fn split_node(seed: u64) { check::<SplitId>(seed)?; }

    #[test]
    fn split_node_fields_round_trip(region in 0u8..8, rack in 0u16..512, host in 0u32..(1 << 20)) {
        let node = SplitNode { region, rack, host };
        let id = SplitId::from_parts(typedflake::Parts {
            elapsed_millis: 1,
            node,
            sequence: 1,
        })
        .unwrap();
        prop_assert_eq!(id.parts().node, node);
    }

    #[test]
    fn split_node_rejects_out_of_range_fields(region in 8u8.., rack in 512u16.., host in (1u32 << 20)..) {
        for node in [
            SplitNode { region, rack: 0, host: 0 },
            SplitNode { region: 0, rack, host: 0 },
            SplitNode { region: 0, rack: 0, host },
        ] {
            let result = SplitId::from_parts(typedflake::Parts {
                elapsed_millis: 1,
                node,
                sequence: 1,
            });
            prop_assert!(matches!(result, Err(InvalidId::Node(_))));
        }
    }
}
