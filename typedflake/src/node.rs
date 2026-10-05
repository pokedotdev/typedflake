//! Node values: the part of an ID that identifies its generator.

use core::fmt;

use crate::format::low_mask;

/// A value that fills the node field of an ID.
///
/// Two kinds of node exist:
///
/// - `u32`, a plain node number that takes whatever width the ID format
///   reserves.
/// - A struct deriving [`TypedNode`](macro@crate::TypedNode), which splits the
///   node field into named parts of fixed width.
///
/// `u32` is the only integer node, which is what lets `typedflake::init(17)`
/// infer its argument type.
///
/// Constructing a node does not validate it. Its fields are checked against
/// their widths when it is consumed, by [`init`](crate::init) or
/// [`Id::generator`](crate::Id::generator).
pub trait Node: Copy + Send + Sync + 'static {
    /// Width of this node type, or `None` to take the width the ID format
    /// reserves (at most 32 bits).
    const BITS: Option<u8>;

    /// Packs this node into the low `bits` bits, rejecting values that do not
    /// fit instead of truncating them.
    fn pack(self, bits: u8) -> Result<u64, NodeError>;

    /// Rebuilds a node from a packed value that fits in `bits` bits.
    fn unpack(raw: u64, bits: u8) -> Self;
}

impl Node for u32 {
    const BITS: Option<u8> = None;

    fn pack(self, bits: u8) -> Result<u64, NodeError> {
        pack_field(0, "node", u64::from(self), bits)
    }

    fn unpack(raw: u64, _bits: u8) -> Self {
        // Formats cap a plain node at 32 bits, so a decoded node always fits.
        raw as u32
    }
}

/// Appends one field to a packed node, checking that its value fits.
#[doc(hidden)]
pub fn pack_field(
    packed: u64,
    field: &'static str,
    value: u64,
    bits: u8,
) -> Result<u64, NodeError> {
    let max = low_mask(bits);
    if value > max {
        return Err(NodeError::new(field, value, max));
    }
    Ok((packed << bits) | value)
}

/// A node field holds a value too large for its width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct NodeError {
    /// Name of the offending field; `"node"` for a plain `u32` node.
    pub field: &'static str,
    /// Value that was supplied.
    pub value: u64,
    /// Largest value the field accepts.
    pub max: u64,
}

impl NodeError {
    /// Creates an error for a field whose value exceeds its limit.
    pub const fn new(field: &'static str, value: u64, max: u64) -> Self {
        Self { field, value, max }
    }
}

impl fmt::Display for NodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "node field `{}` is {}, but its maximum is {}",
            self.field, self.value, self.max
        )
    }
}

impl std::error::Error for NodeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_node_packs_within_its_width() {
        assert_eq!(17u32.pack(10), Ok(17));
        assert_eq!(1023u32.pack(10), Ok(1023));
        assert_eq!(0u32.pack(0), Ok(0));
        assert_eq!(u32::MAX.pack(32), Ok(u64::from(u32::MAX)));
    }

    #[test]
    fn plain_node_rejects_values_that_do_not_fit() {
        assert_eq!(1024u32.pack(10), Err(NodeError::new("node", 1024, 1023)));
        assert_eq!(1u32.pack(0), Err(NodeError::new("node", 1, 0)));
    }

    #[test]
    fn plain_node_round_trips() {
        assert_eq!(u32::unpack(17u32.pack(10).unwrap(), 10), 17);
    }

    #[test]
    fn pack_field_appends_most_significant_first() {
        let packed = pack_field(0, "worker", 0b10001, 5).unwrap();
        let packed = pack_field(packed, "process", 0b00001, 5).unwrap();
        assert_eq!(packed, 0b10001_00001);
    }
}
