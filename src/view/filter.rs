//! Wrappers around portgraphs to filter out nodes and ports.

use crate::{Direction, LinkView, MultiView, NodeIndex, PortIndex, PortOffset, PortView};

use delegate::delegate;

/// Node filter used by [`FilteredGraph`].
pub type NodeFilter<Ctx, N> = fn(NodeIndex<N>, &Ctx) -> bool;
/// Link filter used by [`FilteredGraph`].
///
/// Ports that don't match this predicate will appear disconnected.
pub type LinkFilter<Ctx, P> = fn(PortIndex<P>, &Ctx) -> bool;

/// A wrapper around a [`PortView`] that filters out nodes and links.
///
///
/// Both nodes and links can be filtered by providing the filter functions
/// `node_filter` and `link_filter`.
///
/// As ports always occupy a contiguous interval of indices, they cannot be
/// filtered out directly. Instead, when `link_filter` filters out a port it
/// appears as disconnected, effectively remove the link between ports. A link
/// is removed whenever either of its ports is filtered out.
///
/// For the special case of filtering out nodes only, the type alias
/// [`NodeFiltered`] is provided, along with [`NodeFiltered::new_node_filtered`].
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct FilteredGraph<G, FN, FP, Context = ()> {
    graph: G,
    node_filter: FN,
    link_filter: FP,
    context: Context,
}

/// A wrapper around a portgraph that filters out nodes.
pub type NodeFiltered<G, FN, Context = ()> =
    FilteredGraph<G, FN, fn(PortIndex<<G as PortView>::PortIndexBase>, &Context) -> bool, Context>;

impl<G, FN, FP, Ctx> FilteredGraph<G, FN, FP, Ctx>
where
    G: Clone,
{
    /// Create a new node filtered portgraph.
    pub fn new(graph: G, node_filter: FN, link_filter: FP, context: Ctx) -> Self {
        Self {
            graph,
            node_filter,
            link_filter,
            context,
        }
    }

    /// Get the filter's context.
    pub fn context(&self) -> &Ctx {
        &self.context
    }
}

impl<G: PortView, F, Ctx> NodeFiltered<G, F, Ctx>
where
    G: Clone,
{
    /// Create a new node filtered portgraph.
    pub fn new_node_filtered(graph: G, node_filter: F, context: Ctx) -> Self {
        Self::new(graph, node_filter, |_, _| true, context)
    }
}

/// Filter functions used on the items of the [`FilteredGraph`] iterators.
impl<G, Ctx>
    FilteredGraph<G, NodeFilter<Ctx, G::NodeIndexBase>, LinkFilter<Ctx, G::PortIndexBase>, Ctx>
where
    G: PortView + Clone,
{
    /// Node filter used for the iterators
    fn node_filter(&self, node: &NodeIndex<G::NodeIndexBase>) -> bool {
        (self.node_filter)(*node, &self.context)
    }

    /// Port filter used for the iterators
    ///
    /// A port exists iff its node exists (don't use `link_filter`!)
    fn port_filter(&self, &port: &(impl Into<PortIndex<G::PortIndexBase>> + Copy)) -> bool {
        let node = self.graph.port_node(port).unwrap();
        self.node_filter(&node)
    }
}

impl<G, Ctx>
    FilteredGraph<G, NodeFilter<Ctx, G::NodeIndexBase>, LinkFilter<Ctx, G::PortIndexBase>, Ctx>
where
    G: LinkView + Clone,
{
    /// Link filter used for the iterators
    ///
    /// A link exists if both its ports exist and satisfy `link_filter`.
    fn link_filter(&self, link: &(G::LinkEndpoint, G::LinkEndpoint)) -> bool {
        let &(from, to) = link;
        self.port_filter(&from)
            && self.port_filter(&to)
            && (self.link_filter)(from.into(), &self.context)
            && (self.link_filter)(to.into(), &self.context)
    }
}

impl<G, Ctx> PortView
    for FilteredGraph<G, NodeFilter<Ctx, G::NodeIndexBase>, LinkFilter<Ctx, G::PortIndexBase>, Ctx>
where
    G: PortView + Clone,
{
    type NodeIndexBase = G::NodeIndexBase;
    type PortIndexBase = G::PortIndexBase;
    type PortOffsetBase = G::PortOffsetBase;

    #[inline]
    fn contains_node(&'_ self, node: NodeIndex<Self::NodeIndexBase>) -> bool {
        self.graph.contains_node(node) && (self.node_filter)(node, &self.context)
    }

    #[inline]
    fn contains_port(&self, port: PortIndex<Self::PortIndexBase>) -> bool {
        if self.graph.contains_port(port) {
            let node = self.graph.port_node(port).unwrap();
            self.contains_node(node)
        } else {
            false
        }
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.node_count() == 0
    }

    #[inline]
    fn node_count(&self) -> usize {
        self.nodes_iter().count()
    }

    #[inline]
    fn port_count(&self) -> usize {
        self.ports_iter().count()
    }

    #[inline]
    fn nodes_iter(&self) -> impl Iterator<Item = NodeIndex<Self::NodeIndexBase>> + Clone {
        self.graph.nodes_iter().filter(|n| self.node_filter(n))
    }

    #[inline]
    fn ports_iter(&self) -> impl Iterator<Item = PortIndex<Self::PortIndexBase>> + Clone {
        self.graph.ports_iter().filter(|p| self.port_filter(p))
    }

    delegate! {
        to self.graph {
            fn port_direction(&self, port: impl Into<PortIndex<Self::PortIndexBase>>) -> Option<Direction>;
            fn port_node(&self, port: impl Into<PortIndex<Self::PortIndexBase>>) -> Option<NodeIndex<Self::NodeIndexBase>>;
            fn port_offset(&self, port: impl Into<PortIndex<Self::PortIndexBase>>) -> Option<PortOffset<Self::PortOffsetBase>>;
            fn port_index(&self, node: NodeIndex<Self::NodeIndexBase>, offset: PortOffset<Self::PortOffsetBase>) -> Option<PortIndex<Self::PortIndexBase>>;
            fn ports(&self, node: NodeIndex<Self::NodeIndexBase>, direction: Direction) -> impl Iterator<Item = PortIndex<Self::PortIndexBase>> + Clone;
            fn all_ports(&self, node: NodeIndex<Self::NodeIndexBase>) -> impl Iterator<Item = PortIndex<Self::PortIndexBase>> + Clone;
            fn input(&self, node: NodeIndex<Self::NodeIndexBase>, offset: usize) -> Option<PortIndex<Self::PortIndexBase>>;
            fn output(&self, node: NodeIndex<Self::NodeIndexBase>, offset: usize) -> Option<PortIndex<Self::PortIndexBase>>;
            fn num_ports(&self, node: NodeIndex<Self::NodeIndexBase>, direction: Direction) -> usize;
            fn port_offsets(&self, node: NodeIndex<Self::NodeIndexBase>, direction: Direction) -> impl Iterator<Item = PortOffset<Self::PortOffsetBase>> + Clone;
            fn all_port_offsets(&self, node: NodeIndex<Self::NodeIndexBase>) -> impl Iterator<Item = PortOffset<Self::PortOffsetBase>> + Clone;
            fn node_capacity(&self) -> usize;
            fn port_capacity(&self) -> usize;
            fn node_port_capacity(&self, node: NodeIndex<Self::NodeIndexBase>) -> usize;
        }
    }
}

impl<G, Ctx> LinkView
    for FilteredGraph<G, NodeFilter<Ctx, G::NodeIndexBase>, LinkFilter<Ctx, G::PortIndexBase>, Ctx>
where
    G: LinkView + Clone,
{
    type LinkEndpoint = G::LinkEndpoint;

    fn get_connections(
        &self,
        from: NodeIndex<Self::NodeIndexBase>,
        to: NodeIndex<Self::NodeIndexBase>,
    ) -> impl Iterator<Item = (Self::LinkEndpoint, Self::LinkEndpoint)> + Clone {
        self.graph
            .get_connections(from, to)
            .filter(|l| self.link_filter(l))
    }

    fn port_links(
        &self,
        port: PortIndex<Self::PortIndexBase>,
    ) -> impl Iterator<Item = (Self::LinkEndpoint, Self::LinkEndpoint)> + Clone {
        self.graph.port_links(port).filter(|l| self.link_filter(l))
    }

    fn links(
        &self,
        node: NodeIndex<Self::NodeIndexBase>,
        direction: Direction,
    ) -> impl Iterator<Item = (Self::LinkEndpoint, Self::LinkEndpoint)> + Clone {
        self.graph
            .links(node, direction)
            .filter(|l| self.link_filter(l))
    }

    fn all_links(
        &self,
        node: NodeIndex<Self::NodeIndexBase>,
    ) -> impl Iterator<Item = (Self::LinkEndpoint, Self::LinkEndpoint)> + Clone {
        self.graph.all_links(node).filter(|l| self.link_filter(l))
    }

    fn neighbours(
        &self,
        node: NodeIndex<Self::NodeIndexBase>,
        direction: Direction,
    ) -> impl Iterator<Item = NodeIndex<Self::NodeIndexBase>> + Clone {
        self.links(node, direction)
            .map(|(_, p)| self.graph.port_node(p).unwrap())
    }

    fn all_neighbours(
        &self,
        node: NodeIndex<Self::NodeIndexBase>,
    ) -> impl Iterator<Item = NodeIndex<Self::NodeIndexBase>> + Clone {
        self.all_links(node)
            .map(|(_, p)| self.graph.port_node(p).unwrap())
    }

    fn link_count(&self) -> usize {
        self.nodes_iter()
            .flat_map(|node| self.all_links(node))
            .count()
    }
}

impl<G, Ctx> MultiView
    for FilteredGraph<G, NodeFilter<Ctx, G::NodeIndexBase>, LinkFilter<Ctx, G::PortIndexBase>, Ctx>
where
    G: MultiView + Clone,
{
    fn subports(
        &self,
        node: NodeIndex<Self::NodeIndexBase>,
        direction: Direction,
    ) -> impl Iterator<Item = Self::LinkEndpoint> + Clone {
        self.graph
            .subports(node, direction)
            .filter(|p| self.port_filter(p))
    }

    fn all_subports(
        &self,
        node: NodeIndex<Self::NodeIndexBase>,
    ) -> impl Iterator<Item = Self::LinkEndpoint> + Clone {
        self.graph
            .all_subports(node)
            .filter(|p| self.port_filter(p))
    }

    fn subport_link(&self, subport: Self::LinkEndpoint) -> Option<Self::LinkEndpoint> {
        if !(self.link_filter)(subport.into(), &self.context) {
            return None;
        }
        let to = self.graph.subport_link(subport)?;
        if !(self.link_filter)(to.into(), &self.context) {
            return None;
        }
        Some(to)
    }
}
