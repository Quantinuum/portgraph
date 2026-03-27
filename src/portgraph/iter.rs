//! Iterator structures for a portgraph

use std::{
    iter::{FusedIterator, Zip},
    marker::PhantomData,
    ops::Range,
};

use crate::{
    index::{IndexBase, MaybePortIndex},
    portgraph::{NodeEntry, PortEntry, PortGraph},
    Direction, PortView,
};
use crate::{NodeIndex, PortIndex, PortOffset};

/// Iterator methods for [`PortGraph`] with concrete return types.
///
/// Used internally by other iterator implementations to avoid the generic RPITIT return types.
impl<N: IndexBase, P: IndexBase, PO: IndexBase> PortGraph<N, P, PO> {
    /// Iterates over all the ports of the `node` in the given `direction`.
    pub(crate) fn _ports(&self, node: NodeIndex<N>, direction: Direction) -> NodePorts<P> {
        match self.node_meta_valid(node) {
            Some(node_meta) => NodePorts {
                indices: node_meta.ports(direction),
                _marker: PhantomData,
            },
            None => NodePorts::default(),
        }
    }

    /// Iterates over all the input ports of the `node`.
    ///
    /// Shorthand for [`PortView::ports`].
    #[must_use]
    #[inline]
    pub(crate) fn _inputs(&self, node: NodeIndex<N>) -> NodePorts<P> {
        self._ports(node, Direction::Incoming)
    }

    /// Iterates over all the output ports of the `node`.
    ///
    /// Shorthand for [`PortView::ports`].
    #[must_use]
    #[inline]
    pub(crate) fn _outputs(&self, node: NodeIndex<N>) -> NodePorts<P> {
        self._ports(node, Direction::Outgoing)
    }

    /// Iterates over the input and output ports of the `node` in sequence.
    pub(crate) fn _all_ports(&self, node: NodeIndex<N>) -> NodePorts<P> {
        match self.node_meta_valid(node) {
            Some(node_meta) => NodePorts {
                indices: node_meta.all_ports(),
                _marker: PhantomData,
            },
            None => NodePorts::default(),
        }
    }

    /// Iterates over all the port offsets of the `node` in the given `direction`.
    pub(crate) fn _port_offsets(
        &self,
        node: NodeIndex<N>,
        direction: Direction,
    ) -> NodePortOffsets<PO> {
        match direction {
            Direction::Incoming => NodePortOffsets {
                incoming: 0..self.num_inputs(node),
                outgoing: 0..0,
                _marker: PhantomData,
            },
            Direction::Outgoing => NodePortOffsets {
                incoming: 0..0,
                outgoing: 0..self.num_outputs(node),
                _marker: PhantomData,
            },
        }
    }

    /// Iterates over the input and output port offsets of the `node` in sequence.
    #[inline]
    pub(crate) fn _all_port_offsets(&self, node: NodeIndex<N>) -> NodePortOffsets<PO> {
        NodePortOffsets {
            incoming: 0..self.num_inputs(node),
            outgoing: 0..self.num_outputs(node),
            _marker: PhantomData,
        }
    }

    /// Iterates over the nodes in the port graph.
    #[inline]
    pub(crate) fn _nodes_iter(&self) -> Nodes<'_, N, P, PO> {
        Nodes {
            iter: self.node_meta.iter().enumerate(),
            len: self.node_count,
            _marker: PhantomData,
        }
    }

    /// Iterates over the ports in the port graph.
    #[inline]
    pub(crate) fn _ports_iter(&self) -> Ports<'_, N, P> {
        Ports {
            iter: self.port_meta.iter().enumerate(),
            len: self.port_count,
            _marker: PhantomData,
        }
    }

    /// Returns an iterator over every pair of matching ports connecting `from`
    /// with `to`.
    #[inline]
    pub(crate) fn _get_connections(
        &self,
        from: NodeIndex<N>,
        to: NodeIndex<N>,
    ) -> NodeConnections<'_, N, P, PO> {
        NodeConnections::new(self, to, self._links(from, Direction::Outgoing))
    }

    /// Iterates over the connected links of the `node` in the given
    /// `direction`.
    pub(crate) fn _links(&self, node: NodeIndex<N>, direction: Direction) -> NodeLinks<'_, P> {
        let Some(node_meta) = self.node_meta_valid(node) else {
            return NodeLinks::new(self._ports(node, direction), &[], 0..0);
        };
        let indices = node_meta.ports(direction);
        NodeLinks::new(self._ports(node, direction), &self.port_link[indices], 0..0)
    }

    /// Iterates over the connected input and output links of the `node` in sequence.
    pub(crate) fn _all_links(&self, node: NodeIndex<N>) -> NodeLinks<'_, P> {
        let Some(node_meta) = self.node_meta_valid(node) else {
            return NodeLinks::new(self._all_ports(node), &[], 0..0);
        };
        let indices = node_meta.all_ports();
        // Ignore links where the target is one of the node's output ports.
        // This way we only count self-links once.
        NodeLinks::new(
            self._all_ports(node),
            &self.port_link[indices],
            node_meta.outgoing_ports(),
        )
    }

    /// Iterates over neighbour nodes in the given `direction`.
    /// May contain duplicates if the graph has multiple links between nodes.
    #[inline]
    pub(crate) fn _neighbours(
        &self,
        node: NodeIndex<N>,
        direction: Direction,
    ) -> Neighbours<'_, N, P, PO> {
        Neighbours::from_node_links(self, self._links(node, direction))
    }

    /// Iterates over the input and output neighbours of the `node` in sequence.
    #[inline]
    pub(crate) fn _all_neighbours(&self, node: NodeIndex<N>) -> Neighbours<'_, N, P, PO> {
        Neighbours::from_node_links(self, self._all_links(node))
    }
}

/// Iterator over the ports of a node.
/// See [`PortGraph::inputs`], [`PortGraph::outputs`], and [`PortGraph::all_ports`].
#[derive(Debug, Clone)]
pub struct NodePorts<P: IndexBase> {
    pub(super) indices: Range<usize>,
    _marker: PhantomData<P>,
}

impl<P: IndexBase> Default for NodePorts<P> {
    fn default() -> Self {
        Self {
            indices: 0..0,
            _marker: PhantomData,
        }
    }
}

impl<P: IndexBase> NodePorts<P> {
    /// Return the leftover ports in the iterator as a range of integer indexes.
    #[inline]
    pub fn as_range(&self) -> Range<usize> {
        self.indices.clone()
    }
}

impl<P: IndexBase> Iterator for NodePorts<P> {
    type Item = PortIndex<P>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.indices.next().map(PortIndex::new)
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.indices.nth(n).map(PortIndex::new)
    }

    #[inline]
    fn count(self) -> usize {
        self.indices.count()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.indices.size_hint()
    }
}

impl<P: IndexBase> ExactSizeIterator for NodePorts<P> {
    fn len(&self) -> usize {
        self.indices.len()
    }
}

impl<P: IndexBase> DoubleEndedIterator for NodePorts<P> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.indices.next_back().map(PortIndex::new)
    }
}

impl<P: IndexBase> FusedIterator for NodePorts<P> {}

/// Iterator over the nodes of a graph, created by [`PortGraph::nodes_iter`].
#[derive(Clone, Debug, Default)]
pub struct Nodes<'a, N: IndexBase, P: IndexBase, PO: IndexBase> {
    pub(super) iter: std::iter::Enumerate<std::slice::Iter<'a, NodeEntry<N, P, PO>>>,
    pub(super) len: usize,
    pub(super) _marker: PhantomData<N>,
}

impl<N: IndexBase, P: IndexBase, PO: IndexBase> Iterator for Nodes<'_, N, P, PO> {
    type Item = NodeIndex<N>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.find_map(|(index, node_meta)| match node_meta {
            NodeEntry::Free(_) => None,
            NodeEntry::Node(_) => {
                self.len -= 1;
                Some(NodeIndex::<N>::new(index))
            }
        })
    }

    #[inline]
    fn count(self) -> usize {
        self.len
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<N: IndexBase, P: IndexBase, PO: IndexBase> ExactSizeIterator for Nodes<'_, N, P, PO> {
    fn len(&self) -> usize {
        self.len
    }
}

impl<N: IndexBase, P: IndexBase, PO: IndexBase> DoubleEndedIterator for Nodes<'_, N, P, PO> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.len == 0 {
            return None;
        }

        while let Some((index, node_meta)) = self.iter.next_back() {
            if let NodeEntry::Node(_) = node_meta {
                self.len -= 1;
                return Some(NodeIndex::new(index));
            }
        }

        None
    }
}

impl<N: IndexBase, P: IndexBase, PO: IndexBase> FusedIterator for Nodes<'_, N, P, PO> {}

/// Iterator over the ports of a graph, created by [`PortGraph::ports_iter`].
#[derive(Clone, Debug, Default)]
pub struct Ports<'a, N: IndexBase, P: IndexBase> {
    pub(super) iter: std::iter::Enumerate<std::slice::Iter<'a, PortEntry<N>>>,
    pub(super) len: usize,
    pub(super) _marker: PhantomData<P>,
}

impl<N: IndexBase, P: IndexBase> Iterator for Ports<'_, N, P> {
    type Item = PortIndex<P>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.find_map(|(index, port_entry)| {
            port_entry.as_meta().map(|_| {
                self.len -= 1;
                PortIndex::<P>::new(index)
            })
        })
    }

    fn count(self) -> usize {
        self.len
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<N: IndexBase, P: IndexBase> ExactSizeIterator for Ports<'_, N, P> {
    fn len(&self) -> usize {
        self.len
    }
}

impl<N: IndexBase, P: IndexBase> DoubleEndedIterator for Ports<'_, N, P> {
    fn next_back(&mut self) -> Option<Self::Item> {
        while let Some((index, port_entry)) = self.iter.next_back() {
            if !port_entry.is_free() {
                self.len -= 1;
                return Some(PortIndex::new(index));
            }
        }
        None
    }
}

impl<N: IndexBase, P: IndexBase> FusedIterator for Ports<'_, N, P> {}

/// Iterator over the port offsets of a node. See [`PortGraph::input_offsets`],
/// [`PortGraph::output_offsets`], and [`PortGraph::all_port_offsets`].
#[derive(Clone, Debug, Default)]
pub struct NodePortOffsets<PO> {
    pub(super) incoming: Range<usize>,
    // Outgoing port offsets can go up to u16::MAX, hence the u32
    pub(super) outgoing: Range<usize>,
    /// Marker type of the iterator item type.
    _marker: PhantomData<PO>,
}

impl<PO> NodePortOffsets<PO> {
    /// Return the leftover ports in the iterator as a range of integer indexes.
    #[inline]
    pub fn as_range(&self, dir: Direction) -> &Range<usize> {
        match dir {
            Direction::Incoming => &self.incoming,
            Direction::Outgoing => &self.outgoing,
        }
    }
}

impl<PO: IndexBase> Iterator for NodePortOffsets<PO> {
    type Item = PortOffset<PO>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(i) = self.incoming.next() {
            return Some(PortOffset::new_incoming(i));
        }
        if let Some(i) = self.outgoing.next() {
            return Some(PortOffset::new_outgoing(i));
        }
        None
    }

    fn count(self) -> usize {
        self.len()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl<PO: IndexBase> ExactSizeIterator for NodePortOffsets<PO> {
    fn len(&self) -> usize {
        self.incoming.len() + self.outgoing.len()
    }
}

impl<PO: IndexBase> DoubleEndedIterator for NodePortOffsets<PO> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if let Some(i) = self.outgoing.next_back() {
            return Some(PortOffset::new_outgoing(i));
        }
        if let Some(i) = self.incoming.next_back() {
            return Some(PortOffset::new_incoming(i));
        }
        None
    }
}

/// Iterator over the links of a node, created by [`LinkView::links`]. Returns
/// the port indices linked to each port.
///
/// [`LinkView::links`]: crate::LinkView::links
#[derive(Clone, Debug)]
pub struct NodeLinks<'a, P: IndexBase> {
    links: Zip<NodePorts<P>, std::slice::Iter<'a, MaybePortIndex<P>>>,
    /// Ignore links with target ports in the given range.
    /// This is used to filter out duplicated self-links.
    ignore_target_ports: Range<usize>,
}

impl<'a, P: IndexBase> NodeLinks<'a, P> {
    /// Returns a new iterator
    pub(super) fn new(
        ports: NodePorts<P>,
        links: &'a [MaybePortIndex<P>],
        ignore_target_ports: Range<usize>,
    ) -> Self {
        Self {
            links: ports.zip(links.iter()),
            ignore_target_ports,
        }
    }
}

impl<P: IndexBase> Iterator for NodeLinks<'_, P> {
    type Item = (PortIndex<P>, PortIndex<P>);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (port, link) = self.links.next()?;
            let Some(link) = link.to_option() else {
                continue;
            };
            if self.ignore_target_ports.contains(&link.index()) {
                continue;
            }
            return Some((port, link));
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, self.links.size_hint().1)
    }
}

impl<P: IndexBase> DoubleEndedIterator for NodeLinks<'_, P> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {
            let (port, link) = self.links.next_back()?;
            if let Some(link) = link.to_option() {
                return Some((port, link));
            }
        }
    }
}

impl<P: IndexBase> FusedIterator for NodeLinks<'_, P> {}

/// Iterator over the neighbours of a node, created by
/// [`LinkView::neighbours`]. May return duplicate entries if the graph has
/// multiple links between the same pair of nodes.
///
/// [`LinkView::neighbours`]: crate::LinkView::neighbours
#[derive(Clone, Debug)]
pub struct Neighbours<'a, N: IndexBase, P: IndexBase, PO: IndexBase> {
    graph: &'a PortGraph<N, P, PO>,
    linked_ports: NodeLinks<'a, P>,
}

impl<'a, N: IndexBase, P: IndexBase, PO: IndexBase> Neighbours<'a, N, P, PO> {
    /// Create a new iterator over the neighbours of a node, from an iterator
    /// over the links.
    pub fn from_node_links(graph: &'a PortGraph<N, P, PO>, links: NodeLinks<'a, P>) -> Self {
        Self {
            graph,
            linked_ports: links,
        }
    }
}

impl<N: IndexBase, P: IndexBase, PO: IndexBase> Iterator for Neighbours<'_, N, P, PO> {
    type Item = NodeIndex<N>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.linked_ports
            .next()
            .map(|(_, port)| self.graph.port_node(port).unwrap())
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.linked_ports.size_hint()
    }
}

impl<N: IndexBase, P: IndexBase, PO: IndexBase> DoubleEndedIterator for Neighbours<'_, N, P, PO> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.linked_ports
            .next_back()
            .map(|(_, port)| self.graph.port_node(port).unwrap())
    }
}

impl<N: IndexBase, P: IndexBase, PO: IndexBase> FusedIterator for Neighbours<'_, N, P, PO> {}

/// Iterator over the links connecting two nodes, created by
/// [`LinkView::get_connections`].
///
/// [`LinkView::get_connections`]: crate::LinkView::get_connections
#[derive(Clone, Debug)]
pub struct NodeConnections<'a, N: IndexBase, P: IndexBase, PO: IndexBase> {
    graph: &'a PortGraph<N, P, PO>,
    target: NodeIndex<N>,
    port_links: NodeLinks<'a, P>,
}

impl<'a, N: IndexBase, P: IndexBase, PO: IndexBase> NodeConnections<'a, N, P, PO> {
    /// Create a new iterator over the links connecting two nodes, from an
    /// iterator over the ports and links.
    pub fn new(
        graph: &'a PortGraph<N, P, PO>,
        target: NodeIndex<N>,
        links: NodeLinks<'a, P>,
    ) -> Self {
        Self {
            graph,
            target,
            port_links: links,
        }
    }
}

impl<N: IndexBase, P: IndexBase, PO: IndexBase> Iterator for NodeConnections<'_, N, P, PO> {
    type Item = (PortIndex<P>, PortIndex<P>);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (src, tgt) = self.port_links.next()?;
            if self.graph.port_node(tgt) == Some(self.target) {
                return Some((src, tgt));
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.port_links.size_hint()
    }
}

impl<N: IndexBase, P: IndexBase, PO: IndexBase> DoubleEndedIterator
    for NodeConnections<'_, N, P, PO>
{
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {
            let (src, tgt) = self.port_links.next_back()?;
            if self.graph.port_node(tgt) == Some(self.target) {
                return Some((src, tgt));
            }
        }
    }
}

impl<N: IndexBase, P: IndexBase, PO: IndexBase> FusedIterator for NodeConnections<'_, N, P, PO> {}
