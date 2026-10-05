//! Process-wide default nodes.

use core::any::{Any, TypeId, type_name};
use core::fmt;
use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use crate::generator::GenerateError;
use crate::node::{Node, NodeError};

/// One default per node type. Keeping the node value under its own type means
/// an ID can only receive a node of its declared type, not merely one of the
/// same width.
type Defaults = HashMap<TypeId, Box<dyn Any + Send + Sync>>;

static DEFAULTS: Mutex<Option<Defaults>> = Mutex::new(None);

/// Installs the node used by the static `generate` methods of every ID type
/// with this kind of node.
///
/// Call it during startup, once for each node type in use: a plain node
/// number, and any [`TypedNode`](macro@crate::TypedNode) struct. Explicit
/// generators from [`Id::generator`](crate::Id::generator) and all parsing,
/// conversion, and inspection work without it.
///
/// ```
/// use typedflake::{TypedNode, typedflake};
///
/// #[derive(Debug, Clone, Copy, PartialEq, TypedNode)]
/// pub struct AppNode {
///     #[node(bits = 5)]
///     pub worker: u8,
///     #[node(bits = 5)]
///     pub process: u8,
/// }
///
/// #[typedflake(epoch = "2025-01-01")]
/// pub struct UserId(i64);
///
/// #[typedflake(epoch = "2025-01-01", node = AppNode)]
/// pub struct OrderId(i64);
///
/// let app_node = AppNode { worker: 17, process: 1 };
/// typedflake::init(17)?;
/// typedflake::init(app_node)?;
///
/// assert_eq!(UserId::generate()?.parts().node, 17);
/// assert_eq!(OrderId::generate()?.parts().node, app_node);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// A plain `u32` has no width of its own, so its range is checked by each ID
/// type the first time it generates. A typed node is checked here.
///
/// # Errors
///
/// Returns [`InitError::AlreadyInitialized`] if a node of the same type is
/// already installed. Tests that share a process can ignore it:
/// `let _ = typedflake::init(0);`.
pub fn init<N: Node>(node: N) -> Result<(), InitError> {
    if let Some(bits) = N::BITS {
        node.pack(bits).map_err(InitError::InvalidNode)?;
    }

    // The map is never left half-updated, so a poisoned lock is still valid.
    let mut defaults = DEFAULTS.lock().unwrap_or_else(PoisonError::into_inner);
    let defaults = defaults.get_or_insert_default();
    if defaults.contains_key(&TypeId::of::<N>()) {
        return Err(InitError::AlreadyInitialized);
    }
    defaults.insert(TypeId::of::<N>(), Box::new(node));
    Ok(())
}

pub(crate) fn default_node<N: Node>() -> Result<N, GenerateError> {
    let defaults = DEFAULTS.lock().unwrap_or_else(PoisonError::into_inner);
    defaults
        .as_ref()
        .and_then(|defaults| defaults.get(&TypeId::of::<N>()))
        .and_then(|node| node.downcast_ref::<N>())
        .copied()
        .ok_or(GenerateError::NotInitialized {
            node: type_name::<N>(),
        })
}

/// A default node could not be installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum InitError {
    /// A typed node has a field that does not fit its width.
    InvalidNode(NodeError),
    /// A default node of this type is already installed.
    AlreadyInitialized,
}

impl fmt::Display for InitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNode(_) => f.write_str("default node is not valid"),
            Self::AlreadyInitialized => {
                f.write_str("a default node of this type is already initialized")
            }
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
