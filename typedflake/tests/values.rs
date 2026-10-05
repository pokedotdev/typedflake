//! Declaration forms and pure value operations. Nothing here needs `init`.

use std::collections::HashSet;

use typedflake::{
    BitLayout, Epoch, Format, Id, InvalidId, NodeError, ParseIdError, Parts, TimestampError,
    TypedNode, typedflake,
};

const EPOCH_MILLIS: u64 = 1_735_689_600_000;

pub const APP_IDS: Format = Format {
    epoch: Epoch::from_date(2025, 1, 1),
    bits: BitLayout {
        timestamp: 41,
        node: 10,
        sequence: 12,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, TypedNode)]
pub struct AppNode {
    #[node(bits = 5)]
    pub worker: u8,
    #[node(bits = 5)]
    pub process: u8,
}

/// Doc comments and unrelated derives stay on the type.
#[typedflake(epoch = "2025-01-01")]
#[derive(Default)]
pub struct UserId(i64);

#[typedflake(format = APP_IDS)]
pub struct OrderId(i64);

#[typedflake(format = APP_IDS, node = AppNode)]
pub struct TypedId(i64);

#[typedflake(epoch = "2025-01-01")]
pub struct UnsignedId(u64);

#[typedflake(epoch = "2025-01-01", bits(timestamp = 32, node = 5, sequence = 8))]
pub struct ReducedId(i64);

#[typedflake(epoch = "2025-01-01", bits(timestamp = 40, node = 0, sequence = 8))]
struct NodelessId(i64);

fn raw(elapsed_millis: u64, node: u64, sequence: u64) -> u64 {
    (elapsed_millis << 22) | (node << 12) | sequence
}

#[test]
fn inline_and_shared_formats_are_equivalent() {
    assert_eq!(UserId::FORMAT, APP_IDS);
    assert_eq!(OrderId::FORMAT, APP_IDS);
    assert_eq!(TypedId::FORMAT, APP_IDS);
}

#[test]
fn default_bits_depend_on_the_integer() {
    let bits = |format: Format| {
        (
            format.bits.timestamp,
            format.bits.node,
            format.bits.sequence,
        )
    };
    assert_eq!(bits(UserId::FORMAT), (41, 10, 12));
    assert_eq!(bits(UnsignedId::FORMAT), (42, 10, 12));
    assert_eq!(UnsignedId::FORMAT.epoch.unix_millis(), EPOCH_MILLIS);
}

#[test]
fn ids_are_transparent_copyable_ordered_and_hashable() {
    assert_eq!(size_of::<UserId>(), size_of::<i64>());
    assert_eq!(align_of::<UserId>(), align_of::<i64>());

    let low = UserId::try_from(5_i64).unwrap();
    let high = UserId::try_from(9_i64).unwrap();
    let copy = low;
    assert_eq!(low, copy);
    assert!(low < high);
    assert_eq!(low.max(high), high);

    let set: HashSet<UserId> = [low, high, copy].into_iter().collect();
    assert_eq!(set.len(), 2);
    assert_eq!(UserId::default().get(), 0);
}

#[test]
fn debug_names_the_type_and_display_is_the_plain_number() {
    let id = UserId::try_from(123_456_789_i64).unwrap();
    assert_eq!(format!("{id:?}"), "UserId(123456789)");
    assert_eq!(format!("{id}"), "123456789");
    assert_eq!(id.to_string(), "123456789");
    assert_eq!(format!("{id:>12}"), "   123456789");
}

#[test]
fn signed_ids_accept_nonnegative_values_of_either_integer() {
    let from_signed = UserId::try_from(42_i64).unwrap();
    let from_unsigned = UserId::try_from(42_u64).unwrap();
    assert_eq!(from_signed, from_unsigned);
    assert_eq!(from_signed.get(), 42);

    assert_eq!(UserId::try_from(0_i64).unwrap().get(), 0);
    assert_eq!(UserId::try_from(i64::MAX).unwrap().get(), i64::MAX);

    let raw: i64 = from_signed.into();
    assert_eq!(raw, 42);
    assert_eq!(i64::from(from_signed), 42);
}

#[test]
fn signed_ids_reject_negative_and_oversized_values() {
    assert_eq!(
        UserId::try_from(-1_i64),
        Err(InvalidId::Negative { value: -1 })
    );
    assert_eq!(
        UserId::try_from(i64::MIN),
        Err(InvalidId::Negative { value: i64::MIN })
    );
    assert_eq!(
        UserId::try_from(u64::MAX),
        Err(InvalidId::ReservedBits {
            value: u64::MAX,
            max: i64::MAX as u64,
        })
    );
    assert_eq!(
        UserId::try_from(1_u64 << 63),
        Err(InvalidId::ReservedBits {
            value: 1 << 63,
            max: i64::MAX as u64,
        })
    );
}

#[test]
fn unsigned_ids_accept_every_u64_and_reject_negative_i64() {
    assert_eq!(UnsignedId::try_from(u64::MAX).unwrap().get(), u64::MAX);
    assert_eq!(UnsignedId::try_from(7_i64).unwrap().get(), 7);
    assert_eq!(
        UnsignedId::try_from(-7_i64),
        Err(InvalidId::Negative { value: -7 })
    );
    assert_eq!(u64::from(UnsignedId::try_from(7_u64).unwrap()), 7);
}

#[test]
fn reduced_formats_reject_reserved_bits() {
    let max = (1_u64 << 45) - 1;
    assert_eq!(ReducedId::try_from(max).unwrap().get(), max as i64);
    assert_eq!(ReducedId::try_from(max as i64).unwrap().get(), max as i64);

    for value in [max + 1, 1 << 45, 1 << 62, u64::MAX] {
        assert_eq!(
            ReducedId::try_from(value),
            Err(InvalidId::ReservedBits { value, max })
        );
    }
    assert_eq!(
        ReducedId::try_from(1_i64 << 45),
        Err(InvalidId::ReservedBits {
            value: 1 << 45,
            max
        })
    );
    assert!(matches!(
        (max + 1).to_string().parse::<ReducedId>(),
        Err(ParseIdError::Invalid(InvalidId::ReservedBits { .. }))
    ));
}

#[test]
fn strings_parse_with_the_same_validation() {
    let parsed: UserId = "123456789".parse().unwrap();
    assert_eq!(parsed.get(), 123_456_789);
    assert_eq!(parsed.to_string().parse::<UserId>().unwrap(), parsed);

    assert_eq!(
        "-5".parse::<UserId>(),
        Err(ParseIdError::Invalid(InvalidId::Negative { value: -5 }))
    );
    for text in ["", "abc", "12.5", " 12", "9223372036854775808", "0x10"] {
        assert!(
            matches!(text.parse::<UserId>(), Err(ParseIdError::Int(_))),
            "{text:?} should not parse",
        );
    }
    assert!(matches!(
        "-5".parse::<UnsignedId>(),
        Err(ParseIdError::Int(_))
    ));
    assert_eq!(
        "18446744073709551615".parse::<UnsignedId>().unwrap().get(),
        u64::MAX
    );
}

#[test]
fn parse_errors_keep_their_source() {
    use std::error::Error;

    let invalid = "-5".parse::<UserId>().unwrap_err();
    assert_eq!(invalid.source().unwrap().to_string(), "ID -5 is negative");
    let not_a_number = "abc".parse::<UserId>().unwrap_err();
    assert!(not_a_number.source().is_some());
}

#[test]
fn parts_decode_and_rebuild_simple_node_ids() {
    let id = UserId::try_from(raw(1_000, 17, 3)).unwrap();
    let parts = id.parts();
    assert_eq!(
        parts,
        Parts {
            elapsed_millis: 1_000,
            node: 17,
            sequence: 3,
        }
    );
    assert_eq!(UserId::from_parts(parts), Ok(id));
    assert_eq!(id.unix_millis(), Ok(EPOCH_MILLIS + 1_000));
}

#[test]
fn parts_decode_and_rebuild_typed_node_ids() {
    // Fields pack in declaration order, most significant first.
    let id = TypedId::try_from(raw(1_000, (17 << 5) | 1, 3)).unwrap();
    let parts = id.parts();
    assert_eq!(parts.node.worker, 17);
    assert_eq!(parts.node.process, 1);
    assert_eq!(parts.elapsed_millis, 1_000);
    assert_eq!(parts.sequence, 3);
    assert_eq!(TypedId::from_parts(parts), Ok(id));

    let built = TypedId::from_parts(Parts {
        elapsed_millis: 1,
        node: AppNode {
            worker: 31,
            process: 31,
        },
        sequence: 0,
    })
    .unwrap();
    assert_eq!(built.get() as u64, raw(1, 1023, 0));
}

#[test]
fn from_parts_checks_every_part() {
    let valid = Parts {
        elapsed_millis: (1 << 41) - 1,
        node: 1023,
        sequence: 4095,
    };
    assert_eq!(UserId::from_parts(valid).unwrap().get(), i64::MAX);

    assert_eq!(
        UserId::from_parts(Parts {
            elapsed_millis: 1 << 41,
            ..valid
        }),
        Err(InvalidId::Timestamp {
            value: 1 << 41,
            max: (1 << 41) - 1,
        })
    );
    assert_eq!(
        UserId::from_parts(Parts {
            node: 1024,
            ..valid
        }),
        Err(InvalidId::Node(NodeError::new("node", 1024, 1023)))
    );
    assert_eq!(
        UserId::from_parts(Parts {
            sequence: 4096,
            ..valid
        }),
        Err(InvalidId::Sequence {
            value: 4096,
            max: 4095,
        })
    );
    assert_eq!(
        TypedId::from_parts(Parts {
            elapsed_millis: 0,
            node: AppNode {
                worker: 3,
                process: 32,
            },
            sequence: 0,
        }),
        Err(InvalidId::Node(NodeError::new("process", 32, 31)))
    );
}

#[test]
fn zero_node_bits_accept_only_node_zero() {
    let id = NodelessId::from_parts(Parts {
        elapsed_millis: 9,
        node: 0,
        sequence: 5,
    })
    .unwrap();
    assert_eq!(id.get(), (9 << 8) | 5);
    assert_eq!(id.parts().node, 0);
    assert_eq!(
        NodelessId::from_parts(Parts {
            elapsed_millis: 9,
            node: 1,
            sequence: 5,
        }),
        Err(InvalidId::Node(NodeError::new("node", 1, 0)))
    );
}

#[test]
fn absolute_timestamp_overflow_is_checked() {
    const LATE: Format = Format {
        epoch: Epoch::new(u64::MAX - 10),
        bits: BitLayout {
            timestamp: 42,
            node: 10,
            sequence: 12,
        },
    };

    #[typedflake(format = LATE)]
    struct LateId(u64);

    let fits = LateId::try_from(10_u64 << 22).unwrap();
    assert_eq!(fits.unix_millis(), Ok(u64::MAX));

    let overflows = LateId::try_from(11_u64 << 22).unwrap();
    let error: TimestampError = overflows.unix_millis().unwrap_err();
    assert_eq!(error.epoch_unix_millis, u64::MAX - 10);
    assert_eq!(error.elapsed_millis, 11);
}

#[test]
fn zero_is_a_valid_id() {
    let zero = UserId::try_from(0_u64).unwrap();
    assert_eq!(
        zero.parts(),
        Parts {
            elapsed_millis: 0,
            node: 0,
            sequence: 0,
        }
    );
    assert_eq!(zero.unix_millis(), Ok(EPOCH_MILLIS));
}

#[test]
fn id_trait_supports_generic_code() {
    fn round_trip<I: Id + PartialEq + std::fmt::Debug>(id: I) {
        assert_eq!(I::from_parts(id.parts()).unwrap(), id);
    }

    fn node_bits<I: Id>() -> u8 {
        I::FORMAT.bits.node
    }

    round_trip(UserId::try_from(raw(5, 6, 7)).unwrap());
    round_trip(TypedId::try_from(raw(5, 6, 7)).unwrap());
    round_trip(UnsignedId::try_from(u64::MAX).unwrap());
    assert_eq!(node_bits::<ReducedId>(), 5);
}

/// IDs produced by the 0.1.x default layout (`u64`, 42 timestamp, 5 worker,
/// 5 process, 12 sequence bits, epoch 2025-01-01) must keep their meaning.
mod legacy {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, TypedNode)]
    pub struct LegacyNode {
        #[node(bits = 5)]
        pub worker: u8,
        #[node(bits = 5)]
        pub process: u8,
    }

    #[typedflake(
        epoch = "2025-01-01",
        bits(timestamp = 42, node = 10, sequence = 12),
        node = LegacyNode,
    )]
    pub struct ExactId(u64);

    #[typedflake(epoch = "2025-01-01")]
    pub struct SignedId(i64);

    /// `(raw, timestamp, worker, process, sequence)`, produced by the published
    /// 0.1.3 crate with `compose_custom` and, for the second to last, `generate`.
    const FIXTURES: [(u64, u64, u8, u8, u64); 6] = [
        (0, 0, 0, 0, 0),
        (4_329_473, 1, 1, 1, 1),
        (232_900_560_974_681_078, 55_527_820_819, 17, 9, 1_014),
        (9_223_372_036_854_775_807, 2_199_023_255_551, 31, 31, 4_095),
        (232_664_010_126_917_632, 55_471_422_702, 17, 0, 0),
        (18_446_744_073_709_551_615, 4_398_046_511_103, 31, 31, 4_095),
    ];

    #[test]
    fn exact_declaration_decodes_legacy_ids() {
        for (raw, timestamp, worker, process, sequence) in FIXTURES {
            let id = ExactId::try_from(raw).unwrap();
            assert_eq!(id.get(), raw);
            assert_eq!(
                id.parts(),
                Parts {
                    elapsed_millis: timestamp,
                    node: LegacyNode { worker, process },
                    sequence,
                }
            );
            assert_eq!(id.unix_millis(), Ok(EPOCH_MILLIS + timestamp));
        }
    }

    #[test]
    fn signed_default_declaration_decodes_legacy_ids() {
        let within_horizon = FIXTURES
            .into_iter()
            .filter(|fixture| fixture.0 <= i64::MAX as u64);
        for (raw, timestamp, worker, process, sequence) in within_horizon {
            let id = SignedId::try_from(raw).unwrap();
            assert_eq!(id.get() as u64, raw);
            assert_eq!(
                id.parts(),
                Parts {
                    elapsed_millis: timestamp,
                    node: (u32::from(worker) << 5) | u32::from(process),
                    sequence,
                }
            );
        }
    }

    #[test]
    fn signed_declaration_rejects_legacy_ids_past_its_horizon() {
        // Bit 63 is only reached about 69.7 years after the epoch.
        for beyond in [1_u64 << 63, u64::MAX] {
            assert!(ExactId::try_from(beyond).is_ok());
            assert!(matches!(
                SignedId::try_from(beyond),
                Err(InvalidId::ReservedBits { .. })
            ));
        }
    }
}
