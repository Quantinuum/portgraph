//! Internal data structures for node storage.

use std::ops::Range;
use thiserror::Error;

use crate::index::{MaybeNodeIndex, Unsigned};
use crate::{Direction, PortIndex};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Meta data stored for a valid node.
///
/// The struct is optimized so that `Option<NodeMeta>` has the same size as
/// `NodeMeta`, by using a non-zero value to encode `incoming + 1`.
///
/// # Generic parameters
/// - `P`: The unsigned integer type used for port indices.
/// - `PO`: The unsigned integer type used for port offsets and capacities.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct NodeMeta<P: Unsigned, PO: Unsigned> {
    /// The index of the first port in the port list.
    /// If the node has no ports, this will point to the index 0.
    first_port: PortIndex<P>,
    /// The number of incoming ports plus 1.
    /// We use the `NonZeroU16` here to ensure that `NodeEntry` is 8 bytes.
    incoming: PO::NonZero,
    /// The number of outgoing ports.
    outgoing: PO,
    /// The port capacity allocated to this node. Changing the number of ports
    /// up to this capacity does not require reallocation.
    capacity: PO,
}

impl<P: Unsigned, PO: Unsigned> NodeMeta<P, PO> {
    /// The maximum number of incoming ports for a node.
    /// This is restricted by the `NonZeroU16` representation.
    pub(super) const MAX_INCOMING: usize = u16::MAX as usize - 1;
    /// The maximum number of outgoing ports for a node.
    pub(super) const MAX_OUTGOING: usize = u16::MAX as usize;

    /// Initialize a new `NodeMeta` with the given port counts.
    ///
    /// Returns `None` if the port counts exceed the maximum allowed, or if their sum exceeds the capacity.
    #[inline]
    pub fn try_new(
        first_port: PortIndex<P>,
        incoming: PO,
        outgoing: PO,
        capacity: PO,
    ) -> Result<Self, NodeMetaError> {
        if incoming.to_usize() > Self::MAX_INCOMING.to_usize() {
            return Err(NodeMetaError::TooManyIncoming {
                incoming: incoming.to_usize(),
                max: Self::MAX_INCOMING.to_usize(),
            });
        }
        if outgoing.to_usize() > Self::MAX_OUTGOING.to_usize() {
            return Err(NodeMetaError::TooManyOutgoing {
                outgoing: outgoing.to_usize(),
                max: Self::MAX_OUTGOING.to_usize(),
            });
        }
        if incoming.to_usize().saturating_add(outgoing.to_usize()) > capacity.to_usize() {
            return Err(NodeMetaError::TotalExceedsCapacity {
                total: incoming.to_usize().saturating_add(outgoing.to_usize()),
                capacity: capacity.to_usize(),
            });
        }
        assert!(capacity.to_usize() > 0 || first_port.index() == 0);
        Ok(Self {
            first_port,
            // SAFETY: The value cannot be zero, and won't overflow since it's bounded by `MAX_INCOMING`.
            incoming: unsafe { PO::to_nonzero_unchecked(incoming + PO::one()) },
            outgoing,
            capacity,
        })
    }

    #[inline]
    pub fn first_port(&self) -> PortIndex<P> {
        self.first_port
    }

    /// Returns the number of incoming ports.
    #[inline]
    pub fn incoming(&self) -> usize {
        PO::from_nonzero(self.incoming).to_usize().saturating_sub(1)
    }

    /// Returns the number of outgoing ports.
    #[inline]
    pub fn outgoing(&self) -> usize {
        self.outgoing.to_usize()
    }

    /// Returns the number of ports in a given direction.
    pub fn num_ports(&self, dir: Direction) -> usize {
        match dir {
            Direction::Incoming => self.incoming(),
            Direction::Outgoing => self.outgoing(),
        }
    }

    /// Returns the total number ports.
    #[inline]
    pub fn port_count(&self) -> usize {
        self.outgoing() + self.incoming()
    }

    /// Returns the allocated port capacity for this node.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity.to_usize()
    }

    /// Returns a range over the port indices of this node.
    #[inline]
    pub fn all_ports(&self) -> Range<usize> {
        let start = self.first_port.index();
        let end = start + self.incoming() + self.outgoing();
        start..end
    }

    /// Returns a range over the port indices of this node in a given direction.
    #[inline]
    pub fn ports(&self, direction: Direction) -> Range<usize> {
        match direction {
            Direction::Incoming => self.incoming_ports(),
            Direction::Outgoing => self.outgoing_ports(),
        }
    }

    /// Returns a range over the incoming port indices of this node.
    #[inline]
    pub fn incoming_ports(&self) -> Range<usize> {
        let start = self.first_port.index();
        let end = start + self.incoming();
        start..end
    }

    /// Returns a range over the outgoing port indices of this node.
    #[inline]
    pub fn outgoing_ports(&self) -> Range<usize> {
        let start = self.first_port.index() + self.incoming();
        let end = start + self.outgoing();
        start..end
    }

    /// Returns a range over the unused pre-allocated port indices of this node.
    #[inline]
    pub fn unused_ports(&self) -> Range<usize> {
        let start = self.first_port.index() + self.port_count();
        let end = self.first_port.index() + self.capacity();
        start..end
    }
}

impl<P: Unsigned, PO: Unsigned> Default for NodeMeta<P, PO> {
    fn default() -> Self {
        Self::try_new(PortIndex::default(), PO::zero(), PO::zero(), PO::zero()).unwrap()
    }
}

/// Error raised while creating a node metadata with invalid parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub(crate) enum NodeMetaError {
    /// The number of incoming ports exceeds the maximum allowed.
    #[error("Number of incoming ports ({incoming}) exceeds maximum allowed ({max}).")]
    TooManyIncoming { incoming: usize, max: usize },
    /// The number of outgoing ports exceeds the maximum allowed.
    #[error("Number of outgoing ports ({outgoing}) exceeds maximum allowed ({max}).")]
    TooManyOutgoing { outgoing: usize, max: usize },
    /// The total number of ports exceeds the allocated capacity.
    #[error("Total number of ports ({total}) exceeds allocated capacity ({capacity}).")]
    TotalExceedsCapacity { total: usize, capacity: usize },
}

/// Meta data stored for a node, which might be free.
///
/// # Generic parameters
/// - `P`: The unsigned integer type used for port indices.
/// - `PO`: The unsigned integer type used for port offsets and capacities.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub(super) enum NodeEntry<P: Unsigned, PO: Unsigned> {
    /// No node is stored at this entry.
    /// Instead the entry forms a doubly-linked list of free nodes.
    #[cfg_attr(feature = "serde", serde(rename = "f",))]
    Free(FreeNodeEntry),
    /// A node is stored at this entry.
    ///
    /// This value allows for null-value optimization so that
    /// `size_of::<NodeEntry>() == size_of::<NodeMeta>()`.
    #[cfg_attr(feature = "serde", serde(rename = "n",))]
    Node(NodeMeta<P, PO>),
}

impl<P: Unsigned, PO: Unsigned> NodeEntry<P, PO> {
    /// Returns the free node list entry.
    #[inline]
    pub fn free_entry(&self) -> Option<&FreeNodeEntry> {
        match self {
            NodeEntry::Free(entry) => Some(entry),
            NodeEntry::Node(_) => None,
        }
    }

    /// Returns the free node list entry.
    #[inline]
    pub fn free_entry_mut(&mut self) -> Option<&mut FreeNodeEntry> {
        match self {
            NodeEntry::Free(entry) => Some(entry),
            NodeEntry::Node(_) => None,
        }
    }

    /// Create a new free node entry
    #[inline]
    pub fn new_free(
        prev: impl Into<MaybeNodeIndex<u32>>,
        next: impl Into<MaybeNodeIndex<u32>>,
    ) -> Self {
        NodeEntry::Free(FreeNodeEntry {
            prev: prev.into(),
            next: next.into(),
        })
    }
}

/// Metadata for a free node space.
///
/// The entry forms a doubly-linked list of free nodes.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub(super) struct FreeNodeEntry {
    /// The previous free node.
    pub(super) prev: MaybeNodeIndex<u32>,
    /// The next free node.
    pub(super) next: MaybeNodeIndex<u32>,
}

/* --------------------- Serde implementation for NodeMeta, avoiding a bound on PO::NonZero */

#[cfg(feature = "serde")]
impl<P, PO> serde::Serialize for NodeMeta<P, PO>
where
    P: Unsigned + serde::Serialize,
    PO: Unsigned + serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("NodeMeta", 4)?;
        state.serialize_field("first_port", &self.first_port)?;
        state.serialize_field("incoming", &PO::from_nonzero(self.incoming))?;
        state.serialize_field("outgoing", &self.outgoing)?;
        state.serialize_field("capacity", &self.capacity)?;
        state.end()
    }
}

#[cfg(feature = "serde")]
impl<'de, P, PO> serde::Deserialize<'de> for NodeMeta<P, PO>
where
    P: Unsigned + serde::Deserialize<'de>,
    PO: Unsigned + serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, SeqAccess, Visitor};
        use std::marker::PhantomData;

        const FIELDS: &[&str] = &["first_port", "incoming", "outgoing", "capacity"];

        enum Field {
            FirstPort,
            Incoming,
            Outgoing,
            Capacity,
        }

        impl<'de> serde::Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                        f.write_str("`first_port`, `incoming`, `outgoing`, or `capacity`")
                    }

                    fn visit_str<E: de::Error>(self, value: &str) -> Result<Field, E> {
                        match value {
                            "first_port" => Ok(Field::FirstPort),
                            "incoming" => Ok(Field::Incoming),
                            "outgoing" => Ok(Field::Outgoing),
                            "capacity" => Ok(Field::Capacity),
                            _ => Err(de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct NodeMetaVisitor<P: Unsigned, PO: Unsigned>(PhantomData<(P, PO)>);

        impl<'de, P, PO> Visitor<'de> for NodeMetaVisitor<P, PO>
        where
            P: Unsigned + serde::Deserialize<'de>,
            PO: Unsigned + serde::Deserialize<'de>,
        {
            type Value = NodeMeta<P, PO>;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("struct NodeMeta")
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let first_port = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let incoming: PO = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                let outgoing = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(2, &self))?;
                let capacity = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(3, &self))?;
                let incoming = PO::to_nonzero(incoming)
                    .ok_or_else(|| de::Error::custom("incoming value must be non-zero"))?;
                Ok(NodeMeta {
                    first_port,
                    incoming,
                    outgoing,
                    capacity,
                })
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut first_port = None;
                let mut incoming: Option<PO> = None;
                let mut outgoing = None;
                let mut capacity = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::FirstPort => {
                            if first_port.is_some() {
                                return Err(de::Error::duplicate_field("first_port"));
                            }
                            first_port = Some(map.next_value()?);
                        }
                        Field::Incoming => {
                            if incoming.is_some() {
                                return Err(de::Error::duplicate_field("incoming"));
                            }
                            incoming = Some(map.next_value()?);
                        }
                        Field::Outgoing => {
                            if outgoing.is_some() {
                                return Err(de::Error::duplicate_field("outgoing"));
                            }
                            outgoing = Some(map.next_value()?);
                        }
                        Field::Capacity => {
                            if capacity.is_some() {
                                return Err(de::Error::duplicate_field("capacity"));
                            }
                            capacity = Some(map.next_value()?);
                        }
                    }
                }
                let first_port =
                    first_port.ok_or_else(|| de::Error::missing_field("first_port"))?;
                let incoming: PO = incoming.ok_or_else(|| de::Error::missing_field("incoming"))?;
                let outgoing = outgoing.ok_or_else(|| de::Error::missing_field("outgoing"))?;
                let capacity = capacity.ok_or_else(|| de::Error::missing_field("capacity"))?;
                let incoming = PO::to_nonzero(incoming)
                    .ok_or_else(|| de::Error::custom("incoming value must be non-zero"))?;
                Ok(NodeMeta {
                    first_port,
                    incoming,
                    outgoing,
                    capacity,
                })
            }
        }

        deserializer.deserialize_struct("NodeMeta", FIELDS, NodeMetaVisitor(PhantomData))
    }
}
