//! Internal data structures for port storage.

use crate::index::{BitField, Unsigned};
use crate::{Direction, NodeIndex};

/// Meta data stored for a port, which might be free.
#[derive(Debug, Clone, Copy, PartialEq)]
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

/// Backwards-compatible serialization for `PortEntry` using the format defined
/// before <https://github.com/Quantinuum/portgraph/pull/283>.
///
/// This is encoded as a `N` integer where zero represents a free port, the node index
/// is incremented by one, and the leftmost bit encodes the direction (0 for
/// incoming, 1 for outgoing).
#[cfg(feature = "serde")]
mod port_meta_serialization {
    use super::PortEntry;
    use crate::index::Unsigned;
    use crate::{Direction, NodeIndex};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    /// Returns a bit mask for the direction bit, which is the leftmost bit of
    /// the integer type `N`.
    fn direction_mask<N: Unsigned>() -> N {
        !node_mask::<N>()
    }

    /// Returns a bit mask for the node index bits, which are all bits except
    /// the leftmost bit of the integer type `N`.
    fn node_mask<N: Unsigned>() -> N {
        N::max_value() >> 1
    }

    impl<N: Unsigned + Serialize> Serialize for PortEntry<N> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            let mut bits: N = N::zero();
            if let Some(meta) = self.as_meta() {
                if meta.direction == Direction::Outgoing {
                    bits = bits | direction_mask::<N>();
                }
                let node_bits = N::from_usize(meta.node.index()).unwrap() + N::one();
                bits = bits | (node_bits & node_mask::<N>());
            }

            bits.serialize(serializer)
        }
    }

    impl<'de, N: Unsigned + Deserialize<'de>> Deserialize<'de> for PortEntry<N> {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let bits = N::deserialize(deserializer)?;
            let direction_bit = bits & direction_mask::<N>();
            let node_bits = bits & node_mask::<N>();

            if node_bits.is_zero() {
                Ok(PortEntry::new_free())
            } else {
                let direction = if direction_bit.is_zero() {
                    Direction::Incoming
                } else {
                    Direction::Outgoing
                };
                let node_index = node_bits.sub(N::one());
                Ok(PortEntry::new(
                    NodeIndex::new(node_index.to_usize()),
                    direction,
                ))
            }
        }
    }
}
