use std::fmt::Debug;
use std::iter::FusedIterator;

use bitvec::vec::BitVec;

use crate::{Direction, LinkView, NodeIndex, PortIndex, PortView, SecondaryMap};

/// Returns an iterator doing a post-order traversal of a spanning tree in a
/// [`PortGraph`].
///
/// The iterator will visit all nodes reachable from the `source`s, returning
/// the nodes in each subtree following the port order before returning the
/// root.
///
/// # Example
///
/// We create the following tree:
///
/// a ━▸ b ┳━▸ c
///        ┣━▸ d ━▸ e
///        ┗━▸ f
///
/// And traverse it in post-order:
///
/// ```
/// # if cfg!(miri) { return; } // bitvec 1.1.1 fails Miri Stacked Borrows checks
/// # use ::portgraph::algorithms::*;
/// # use ::portgraph::*;
/// # type PortGraph = ::portgraph::PortGraph<u32, u32, u16>;
/// let mut graph = PortGraph::new();
///
/// let a = graph.add_node(0, 1);
/// let b = graph.add_node(1, 3);
/// let c = graph.add_node(1, 1);
/// let d = graph.add_node(1, 1);
/// let e = graph.add_node(1, 0);
/// let f = graph.add_node(1, 0);
///
/// graph.link_nodes(a, 0, b, 0).unwrap();
/// graph.link_nodes(b, 0, c, 0).unwrap();
/// graph.link_nodes(b, 1, d, 0).unwrap();
/// graph.link_nodes(b, 2, f, 0).unwrap();
/// graph.link_nodes(d, 0, e, 0).unwrap();
///
/// // Forward starting from `a`
/// let order: PostOrder<_> = postorder(&graph, vec![a], Direction::Outgoing);
/// assert_eq!(order.collect::<Vec<_>>(), vec![c, e, d, f, b, a]);
///
/// // Reverse starting from `e`
/// let order: PostOrder<_> = postorder(&graph, vec![e], Direction::Incoming);
/// assert_eq!(order.collect::<Vec<_>>(), vec![a, b, d, e]);
/// ```
///
///
pub fn postorder<G, Map>(
    graph: G,
    source: impl IntoIterator<Item = NodeIndex<G::NodeIndexBase>>,
    direction: Direction,
) -> PostOrder<'static, G, Map>
where
    Map: SecondaryMap<NodeIndex<G::NodeIndexBase>, bool>,
    G: LinkView,
{
    PostOrder::new(graph, source, direction, None, None)
}

/// Returns an iterator doing a post-order traversal of a spanning tree in a
/// [`PortGraph`], applying a filter to the nodes and ports. No filtered nodes
/// are returned, and neither are any nodes only accessible via filtered nodes
/// or filtered ports.
///
/// If the filter closures return false for a node or port, it is skipped.
///
/// The iterator will visit all nodes reachable from the `source`s, returning
/// the nodes in each subtree following the port order before returning the
/// root.
///
/// # Example
///
/// We create the following tree:
///
/// a ━▸ b ┳━▸ c
///        ┣━▸ d ━▸ e
///        ┗━▸ f
///
/// And traverse it in post-order:
///
/// ```
/// # if cfg!(miri) { return; } // bitvec 1.1.1 fails Miri Stacked Borrows checks
/// # use ::portgraph::algorithms::*;
/// # use ::portgraph::*;
/// # type PortGraph = ::portgraph::PortGraph<u32, u32, u16>;
/// let mut graph = PortGraph::new();
///
/// let a = graph.add_node(0, 1);
/// let b = graph.add_node(1, 3);
/// let c = graph.add_node(1, 1);
/// let d = graph.add_node(1, 1);
/// let e = graph.add_node(1, 0);
/// let f = graph.add_node(1, 0);
///
/// graph.link_nodes(a, 0, b, 0).unwrap();
/// graph.link_nodes(b, 0, c, 0).unwrap();
/// graph.link_nodes(b, 1, d, 0).unwrap();
/// graph.link_nodes(b, 2, f, 0).unwrap();
/// graph.link_nodes(d, 0, e, 0).unwrap();
///
/// // Forward starting from `a`
/// let order: PostOrder<_> = postorder_filtered(
///     &graph,
///     vec![a],
///     Direction::Outgoing,
///     |_| true,
///     |_node, port| ![graph.output(b, 2), graph.input(d, 0)].contains(&Some(port))
/// );
/// assert_eq!(order.collect::<Vec<_>>(), vec![c, b, a]);
///
/// // Reverse starting from `e`
/// let order: PostOrder<_> = postorder_filtered(
///     &graph,
///     vec![e],
///     Direction::Incoming,
///     |n| n != b,
///     |_,_| true
/// );
/// assert_eq!(order.collect::<Vec<_>>(), vec![d, e]);
/// ```
///
///
pub fn postorder_filtered<'f, G, Map>(
    graph: G,
    source: impl IntoIterator<Item = NodeIndex<G::NodeIndexBase>>,
    direction: Direction,
    node_filter: impl FnMut(NodeIndex<G::NodeIndexBase>) -> bool + 'f,
    port_filter: impl FnMut(NodeIndex<G::NodeIndexBase>, PortIndex<G::PortIndexBase>) -> bool + 'f,
) -> PostOrder<'f, G, Map>
where
    Map: SecondaryMap<NodeIndex<G::NodeIndexBase>, bool>,
    G: LinkView,
{
    PostOrder::new(
        graph,
        source,
        direction,
        Some(Box::new(node_filter)),
        Some(Box::new(port_filter)),
    )
}

/// Iterator over a [`PortGraph`] in post-order.
///
/// See [`postorder`] for more information.
#[allow(clippy::type_complexity)]
pub struct PostOrder<'f, G, Map = BitVec>
where
    G: PortView,
{
    graph: G,
    stack: Vec<NodeIndex<G::NodeIndexBase>>,
    visited: Map,
    finished: Map,
    direction: Direction,
    /// The number of nodes already returned from the iterator.
    /// This is used to calculate the upper bound for the iterator's `size_hint`.
    nodes_seen: usize,
    /// A filter closure for the nodes to visit. If the closure returns false,
    /// the node is skipped.
    node_filter: Option<Box<dyn FnMut(NodeIndex<G::NodeIndexBase>) -> bool + 'f>>,
    /// A filter closure for the ports to visit. If the closure returns false,
    /// the port is skipped.
    port_filter: Option<
        Box<dyn FnMut(NodeIndex<G::NodeIndexBase>, PortIndex<G::PortIndexBase>) -> bool + 'f>,
    >,
}

impl<'f, G, Map> PostOrder<'f, G, Map>
where
    Map: SecondaryMap<NodeIndex<G::NodeIndexBase>, bool>,
    G: LinkView,
{
    #[allow(clippy::type_complexity)]
    fn new(
        graph: G,
        source: impl IntoIterator<Item = NodeIndex<G::NodeIndexBase>>,
        direction: Direction,
        mut node_filter: Option<Box<dyn FnMut(NodeIndex<G::NodeIndexBase>) -> bool + 'f>>,
        port_filter: Option<
            Box<dyn FnMut(NodeIndex<G::NodeIndexBase>, PortIndex<G::PortIndexBase>) -> bool + 'f>,
        >,
    ) -> Self {
        let mut visited: Map = SecondaryMap::new();
        let mut finished: Map = SecondaryMap::new();
        // If the default value of `Map` is not `false`, we must mark all ports as not visited.
        if visited.default_value() {
            for node in graph.nodes_iter() {
                visited.set(node, false);
                finished.set(node, false);
            }
        }

        let mut source: Vec<_> = if let Some(node_filter) = node_filter.as_mut() {
            source.into_iter().filter(|&n| node_filter(n)).collect()
        } else {
            source.into_iter().collect()
        };
        source.reverse();

        Self {
            graph,
            stack: source,
            visited,
            finished,
            direction,
            nodes_seen: 0,
            node_filter,
            port_filter,
        }
    }
}

impl<G, Map> Iterator for PostOrder<'_, G, Map>
where
    Map: SecondaryMap<NodeIndex<G::NodeIndexBase>, bool>,
    G: LinkView,
{
    type Item = NodeIndex<G::NodeIndexBase>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(next) = self.stack.last().copied() {
            if !self.visited.get(next) {
                self.visited.set(next, true);
                let ports = self.graph.ports(next, self.direction).collect::<Vec<_>>();
                // The node is visited for the first time. We leave the node on the stack and push
                // all of its neighbours in the traversal direction.
                for port in ports.into_iter().rev() {
                    if !self
                        .port_filter
                        .as_mut()
                        .map_or(true, |filter| filter(next, port))
                    {
                        continue;
                    }

                    for link in self.graph.port_links(port) {
                        let link_endpoint = link.1.into();
                        let link_node = self.graph.port_node(link_endpoint).unwrap();

                        if !self
                            .node_filter
                            .as_mut()
                            .map_or(true, |filter| filter(link_node))
                            || !self
                                .port_filter
                                .as_mut()
                                .map_or(true, |filter| filter(link_node, link_endpoint))
                        {
                            continue;
                        }

                        if !self.visited.get(link_node) {
                            self.stack.push(link_node);
                        }
                    }
                }
            } else if !self.finished.get(next) {
                self.finished.set(next, true);
                // The node is visited for the second time. We remove it from the stack and return
                // as the next node in the traversal.
                self.stack.pop();
                self.nodes_seen += 1;
                return Some(next);
            } else {
                // The node has already been visited at least twice, so we ignore it.
                self.stack.pop();
            }
        }

        None
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.graph.node_count() - self.nodes_seen))
    }
}

impl<G, Map> FusedIterator for PostOrder<'_, G, Map>
where
    Map: SecondaryMap<NodeIndex<G::NodeIndexBase>, bool>,
    G: LinkView,
{
}

impl<G, Map> std::fmt::Debug for PostOrder<'_, G, Map>
where
    Map: Debug,
    G: Debug + PortView,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostOrder")
            .field("graph", &self.graph)
            .field("stack", &self.stack)
            .field("visited", &self.visited)
            .field("finished", &self.finished)
            .field("direction", &self.direction)
            .field("nodes_seen", &self.nodes_seen)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use crate::{LinkMut, MultiPortGraph, PortMut};

    use super::*;

    type PortGraph = crate::PortGraph<u32, u32, u16>;

    #[test]
    #[cfg_attr(miri, ignore = "bitvec 1.1.1 fails Miri Stacked Borrows checks")]
    fn postorder_tree() {
        let mut graph = PortGraph::new();
        // a ━▸ b ┳━▸ c
        //        ┣━▸
        //        ┣━▸ d ━▸ e
        //        ┗━▸ f
        let a = graph.add_node(0, 1);
        let b = graph.add_node(1, 4);
        let c = graph.add_node(1, 1);
        let d = graph.add_node(1, 1);
        let e = graph.add_node(2, 0);
        let f = graph.add_node(1, 0);

        graph.link_nodes(a, 0, b, 0).unwrap();
        graph.link_nodes(b, 0, c, 0).unwrap();
        graph.link_nodes(b, 2, d, 0).unwrap();
        graph.link_nodes(b, 3, f, 0).unwrap();
        graph.link_nodes(d, 0, e, 1).unwrap();

        // Forward starting from `a`
        let order: PostOrder<_> = postorder(&graph, vec![a], Direction::Outgoing);
        assert_eq!(order.collect::<Vec<_>>(), vec![c, e, d, f, b, a]);

        // Skipping `a`, starting from `b`
        let order: PostOrder<_> = postorder(&graph, vec![b], Direction::Outgoing);
        assert_eq!(order.collect::<Vec<_>>(), vec![c, e, d, f, b]);

        // Exploring `d` before `b`
        let order: PostOrder<_> = postorder(&graph, vec![d, b], Direction::Outgoing);
        assert_eq!(order.collect::<Vec<_>>(), vec![e, d, c, f, b]);

        // Reverse starting from `e`
        let order: PostOrder<_> = postorder(&graph, vec![e], Direction::Incoming);
        assert_eq!(order.collect::<Vec<_>>(), vec![a, b, d, e]);
    }

    #[test]
    #[cfg_attr(miri, ignore = "bitvec 1.1.1 fails Miri Stacked Borrows checks")]
    fn postorder_dag() {
        let mut graph = PortGraph::new();
        // a -> b -> c
        //       \     \
        //        \--------> d
        let a = graph.add_node(0, 1);
        let b = graph.add_node(1, 2);
        let c = graph.add_node(1, 1);
        let d = graph.add_node(2, 0);

        graph.link_nodes(a, 0, b, 0).unwrap();
        graph.link_nodes(b, 0, c, 0).unwrap();
        graph.link_nodes(b, 1, d, 0).unwrap();
        graph.link_nodes(c, 0, d, 1).unwrap();

        // Forward starting from `a`
        let order: PostOrder<_> = postorder(&graph, vec![a], Direction::Outgoing);
        assert_eq!(order.collect::<Vec<_>>(), vec![d, c, b, a]);

        // Backwards starting from `d`
        let order: PostOrder<_> = postorder(&graph, vec![d], Direction::Incoming);
        assert_eq!(order.collect::<Vec<_>>(), vec![a, b, c, d]);
    }

    #[test]
    #[cfg_attr(miri, ignore = "bitvec 1.1.1 fails Miri Stacked Borrows checks")]
    fn postorder_cycle() {
        let mut graph = PortGraph::new();
        // a -> b -> c -> d -> b -> ...
        let a = graph.add_node(0, 1);
        let b = graph.add_node(3, 2);
        let c = graph.add_node(1, 1);
        let d = graph.add_node(1, 1);

        graph.link_nodes(a, 0, b, 0).unwrap();
        graph.link_nodes(b, 0, c, 0).unwrap();
        graph.link_nodes(c, 0, d, 0).unwrap();
        graph.link_nodes(d, 0, b, 2).unwrap();

        // Forward starting from `a`
        let order: PostOrder<_> = postorder(&graph, vec![a], Direction::Outgoing);
        assert_eq!(order.collect::<Vec<_>>(), vec![d, c, b, a]);

        // Backwards starting from `c`
        let order: PostOrder<_> = postorder(&graph, vec![c], Direction::Incoming);
        assert_eq!(order.collect::<Vec<_>>(), vec![a, d, b, c]);
    }

    #[test]
    fn postorder_multiport_diamond() {
        let mut graph = MultiPortGraph::<u32>::new();
        // a --> b - -> d
        //  \        /
        //   \-> c -/
        let a = graph.add_node(0, 1);
        let b = graph.add_node(1, 1);
        let c = graph.add_node(1, 1);
        let d = graph.add_node(1, 0);

        let a_out = graph.ports(a, Direction::Outgoing).next().unwrap();
        let b_out = graph.ports(b, Direction::Outgoing).next().unwrap();
        let c_out = graph.ports(c, Direction::Outgoing).next().unwrap();

        let b_in = graph.ports(b, Direction::Incoming).next().unwrap();
        let c_in = graph.ports(c, Direction::Incoming).next().unwrap();
        let d_in = graph.ports(d, Direction::Incoming).next().unwrap();

        // Linking a_out twice will implicitly create a node that we do not want
        // to appear in the post order when passing a MultiPortGraph.
        graph.link_ports(a_out, b_in).unwrap();
        graph.link_ports(a_out, c_in).unwrap();
        // Linking d_in twice will also create an implicit node.
        graph.link_ports(b_out, d_in).unwrap();
        graph.link_ports(c_out, d_in).unwrap();

        // Raw portgraph order starting from `a` includes implicit nodes.
        let order: PostOrder<_> = postorder(graph.as_portgraph(), vec![a], Direction::Outgoing);
        assert_eq!(order.collect::<Vec<_>>().len(), 6);

        // Forward starting from `a`
        let order: PostOrder<_> = postorder(&graph, vec![a], Direction::Outgoing);
        assert_eq!(order.collect::<Vec<_>>(), vec![d, c, b, a]);

        // Backwards starting from `c`
        let order: PostOrder<_> = postorder(&graph, vec![c], Direction::Incoming);
        assert_eq!(order.collect::<Vec<_>>(), vec![a, c]);
    }

    #[test]
    #[cfg_attr(miri, ignore = "bitvec 1.1.1 fails Miri Stacked Borrows checks")]
    fn postorder_with_filters() {
        let mut graph = PortGraph::new();
        // a -> b -> c
        //       \     \
        //        \--2-----> d
        let a = graph.add_node(0, 1);
        let b = graph.add_node(1, 3);
        let c = graph.add_node(1, 1);
        let d = graph.add_node(3, 0);

        graph.link_nodes(a, 0, b, 0).unwrap();
        graph.link_nodes(b, 0, c, 0).unwrap();
        graph.link_nodes(b, 1, d, 0).unwrap();
        graph.link_nodes(c, 0, d, 1).unwrap();
        graph.link_nodes(b, 2, d, 2).unwrap(); // Double link

        // Completely node-filtered
        let order: PostOrder<_> =
            postorder_filtered(&graph, vec![a], Direction::Outgoing, |_| false, |_, _| true);
        assert_eq!(order.collect::<Vec<_>>(), vec![]);

        // Completely edge-filtered
        let order: PostOrder<_> =
            postorder_filtered(&graph, vec![a], Direction::Outgoing, |_| true, |_, _| false);
        assert_eq!(order.collect::<Vec<_>>(), vec![a]);

        // Backwards starting from `d`. Filtering one of the double links from `b`, and node `c`.
        let order: PostOrder<_> = postorder_filtered(
            &graph,
            vec![d],
            Direction::Incoming,
            |n| n != c,
            |_, p| Some(p) != graph.output(b, 2),
        );
        assert_eq!(order.collect::<Vec<_>>(), vec![a, b, d]);
    }
}
