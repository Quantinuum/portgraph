//! Internal data structures for port storage.

use crate::index::{BitField, Unsigned};
use crate::{Direction, NodeIndex};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Meta data stored for a port, which might be free.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub(super) struct PortEntry<N: Unsigned>(
    /// A bitfield containing either:
    ///
    /// - A free port marker.
    ///   This signals that no port is stored at this entry.
    ///   The port index will be part of a port list currently on the free list.
    ///
    /// - A valid port marker, encoding a `NodeIndex` and a `Direction`.
    ///
    /// This is similar to `MaybeNodeIndex`.
    BitField<N>,
);

impl<N: Unsigned> PortEntry<N> {
    /// Create a new free port entry with the given next free port index.
    #[inline]
    pub fn new_free() -> Self {
        Self(BitField::new_none())
    }

    /// Create a new valid port entry with the given node index and direction.
    #[inline]
    pub fn new(node: NodeIndex<N>, direction: Direction) -> Self {
        let bit_flag = match direction {
            Direction::Incoming => false,
            Direction::Outgoing => true,
        };
        Self(BitField::new(node.index(), bit_flag))
    }

    /// Return `true` if the port entry is free, i.e. it does not contain a valid port.
    #[inline]
    pub fn is_free(self) -> bool {
        self.0.is_none()
    }

    /// Return the port metadata if the port entry is valid, or `None` if it is free.
    #[inline]
    pub fn as_meta(self) -> Option<PortMeta<N>> {
        if self.0.is_none() {
            return None;
        }
        Some(PortMeta {
            node: NodeIndex::new(self.0.index_unchecked()),
            direction: if self.0.bit_flag() == Some(true) {
                Direction::Outgoing
            } else {
                Direction::Incoming
            },
        })
    }
}

/// Meta data stored for a valid port.
///
/// Encodes a `NodeIndex` and a `Direction` by using the last bit.
/// We use a `NonZeroU32` here to ensure that `PortEntry` only uses 4 bytes.
///
/// # Generic parameters
/// - `N`: The unsigned integer type used for node indices.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PortMeta<N: Unsigned> {
    /// The node index where this port is located
    pub node: NodeIndex<N>,
    /// The direction of this port
    pub direction: Direction,
}
