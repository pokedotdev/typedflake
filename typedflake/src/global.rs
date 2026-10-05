//! Process-wide default node.

use core::any::{Any, type_name};
use core::fmt;
use std::sync::OnceLock;

use crate::generator::GenerateError;
use crate::node::{Node, NodeError};

/// The default keeps the node value itself, so an ID can only use it when its
/// node type is the same type, not merely the same width.
struct DefaultNode {
    node: Box<dyn Any + Send + Sync>,
    type_name: &'static str,
}

static DEFAULT: OnceLock<DefaultNode> = OnceLock::new();

/// Installs the node used by the static `generate` methods of every ID type.
///
/// Call it once during startup, with a plain node number or a
/// [`TypedNode`](macro@crate::TypedNode) struct. Explicit generators from
/// [`Id::generator`](crate::Id::generator) and all parsing, conversion, and
/// inspection work without it.
///
/// ```
/// use typedflake::typedflake;
///
/// #[typedflake(epoch = "2025-01-01")]
/// pub struct UserId(i64);
///
/// typedflake::init(17)?;
///
/// let id = UserId::generate()?;
/// assert_eq!(id.parts().node, 17);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// A plain `u32` has no width of its own, so its range is checked by each ID
/// type the first time it generates. A typed node is checked here.
///
/// # Errors
///
/// Returns [`InitError::AlreadyInitialized`] on every call after the first.
/// Tests that share a process can ignore it: `let _ = typedflake::init(0);`.
pub fn init<N: Node>(node: N) -> Result<(), InitError> {
    if let Some(bits) = N::BITS {
        node.pack(bits).map_err(InitError::InvalidNode)?;
    }

    DEFAULT
        .set(DefaultNode {
            node: Box::new(node),
            type_name: type_name::<N>(),
        })
        .map_err(|_| InitError::AlreadyInitialized)
}

pub(crate) fn default_node<N: Node>() -> Result<N, GenerateError> {
    let default = DEFAULT.get().ok_or(GenerateError::NotInitialized)?;
    default
        .node
        .downcast_ref::<N>()
        .copied()
        .ok_or(GenerateError::NodeSchemaMismatch {
            expected: type_name::<N>(),
            found: default.type_name,
        })
}

/// The default node could not be installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum InitError {
    /// A typed node has a field that does not fit its width.
    InvalidNode(NodeError),
    /// A default node is already installed.
    AlreadyInitialized,
}

impl fmt::Display for InitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNode(_) => f.write_str("default node is not valid"),
            Self::AlreadyInitialized => f.write_str("default node is already initialized"),
        }
    }
}

impl std::error::Error for InitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidNode(error) => Some(error),
            Self::AlreadyInitialized => None,
        }
    }
}
