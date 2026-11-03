use crate::{Direction, LinkView, NodeIndex, PortIndex, PortView, SecondaryMap};
use bitvec::prelude::BitVec;
use smallvec::SmallVec;
use std::{collections::VecDeque, fmt::Debug, iter::FusedIterator};

/// Returns an iterator over a [`LinkView`] in topological order.
///
/// The iterator starts at the `source` nodes, which are visited without checking
/// their predecessors. Other nodes are only visited once all their predecessors
/// in `direction` have been visited. This includes all links when a port has
/// multiple links.
///
/// Nodes with predecessors that cannot be reached from the sources are not
/// visited, even if the nodes themselves are reachable.
///
/// ## Type parameters
/// - `G`: The graph type implementing [`LinkView`],
/// - `Map`: Internal workspace for graph traversal (see below).
///
/// The `Map` type parameter specifies the type of the secondary map that is
/// used to store port visitation data. The default is [`BitVec`], which is
/// efficient for full graph traversal, i.e. when all nodes are reachable from
/// the source nodes. For sparse traversals, `HashMap` or `BTreeMap` may be more
/// efficient.
///
/// Implements [Kahn's algorithm](https://en.wikipedia.org/wiki/Topological_sorting#Kahn's_algorithm),
/// tie-breaking within the same rank is done using a queue, so nodes that are
/// discovered first are visited first.
///
/// # Example
///
/// ```
/// # if cfg!(miri) { return; } // bitvec 1.1.1 fails Miri Stacked Borrows checks
/// # use ::portgraph::algorithms::*;
/// # use ::portgraph::*;
/// # type PortGraph = ::portgraph::PortGraph<u32, u32, u16>;
/// let mut graph: PortGraph = PortGraph::new();
/// let node_a = graph.add_node(2, 2);
/// let node_b = graph.add_node(2, 2);
/// let node_c = graph.add_node(2, 2);
/// graph.link_nodes(node_a, 0, node_b, 0).unwrap();
/// graph.link_nodes(node_a, 1, node_c, 0).unwrap();
///
/// // Run a topological sort on the graph starting at node A.
/// let topo: TopoSort<_> = toposort(&graph, [node_a], Direction::Outgoing);
/// assert_eq!(topo.collect::<Vec<_>>(), [node_a, node_b, node_c]);
/// ```
pub fn toposort<G, Map>(
    graph: G,
    source: impl IntoIterator<Item = NodeIndex<G::NodeIndexBase>>,
    direction: Direction,
) -> TopoSort<'static, G, Map>
where
    Map: SecondaryMap<PortIndex<G::PortIndexBase>, bool>,
    G: LinkView,
{
    TopoSort::new(graph, source, direction, None, None)
}

/// Returns an iterator over a [`LinkView`] in topological order, applying a
/// filter to the nodes and ports. No filtered nodes are returned, and neither
/// are any nodes only accessible via filtered nodes or filtered ports.
///
/// If the filter closures return false for a node or port, it is skipped.
///
/// If either port of a link is filtered out, the link is ignored. Other than the
/// sources, nodes are only visited if they can be reached via an unfiltered link
/// and all their unfiltered predecessor links have been visited. This includes
/// all links when a port has multiple links.
///
/// Filtering a node does not filter its links. Its successors cannot be visited
/// unless the connecting ports are also filtered out. The filter closures should
/// return the same result for a given node or port throughout the traversal.
///
/// ## Type parameters
/// - `'f`: The lifetime of the filter closures,
/// - `G`: The graph type implementing [`LinkView`],
/// - `Map`: Internal workspace for graph traversal (see below).
///
/// The `Map` type parameter specifies the type of the secondary map that is
/// used to store port visitation data. The default is [`BitVec`], which is
/// efficient for full graph traversal, i.e. when all nodes are reachable from
/// the source nodes. For sparse traversals, `HashMap` or `BTreeMap` may be more
/// efficient.
///
/// Implements [Kahn's
/// algorithm](https://en.wikipedia.org/wiki/Topological_sorting#Kahn's_algorithm),
/// tie-breaking within the same rank is done using a queue, so nodes that are
/// discovered first are visited first.
///
/// # Example
///
/// ```
/// # if cfg!(miri) { return; } // bitvec 1.1.1 fails Miri Stacked Borrows checks
/// # use ::portgraph::algorithms::*;
/// # use ::portgraph::*;
/// # type PortGraph = ::portgraph::PortGraph<u32, u32, u16>;
/// let mut graph: PortGraph = PortGraph::new();
/// let node_a = graph.add_node(2, 2);
/// let node_b = graph.add_node(2, 2);
/// let node_c = graph.add_node(2, 2);
/// let node_d = graph.add_node(2, 2);
/// graph.link_nodes(node_a, 0, node_b, 0).unwrap();
/// graph.link_nodes(node_a, 1, node_c, 0).unwrap();
///
/// // Run a topological sort on the graph starting at node A.
/// let topo: TopoSort<_> = toposort_filtered(
///     &graph,
///     [node_a, node_d],
///     Direction::Outgoing,
///     |n| n != node_d,
///     |_, p| Some(p) != graph.output(node_a, 1),
/// );
/// assert_eq!(topo.collect::<Vec<_>>(), [node_a, node_b]);
/// ```
pub fn toposort_filtered<'f, G, Map>(
    graph: G,
    source: impl IntoIterator<Item = NodeIndex<G::NodeIndexBase>>,
    direction: Direction,
    node_filter: impl FnMut(NodeIndex<G::NodeIndexBase>) -> bool + 'f,
    port_filter: impl FnMut(NodeIndex<G::NodeIndexBase>, PortIndex<G::PortIndexBase>) -> bool + 'f,
) -> TopoSort<'f, G, Map>
where
    Map: SecondaryMap<PortIndex<G::PortIndexBase>, bool>,
    G: LinkView,
{
    TopoSort::new(
        graph,
        source,
        direction,
        Some(Box::new(node_filter)),
        Some(Box::new(port_filter)),
    )
}

/// Iterator over a [`LinkView`] in topological order.
///
/// See [`toposort`] for more information.
///
/// Implements [Kahn's algorithm](https://en.wikipedia.org/wiki/Topological_sorting#Kahn's_algorithm),
/// tie-breaking within the same rank is done using a queue, so nodes that are
/// discovered first are visited first.
pub struct TopoSort<'f, G, Map = BitVec>
where
    G: PortView,
{
    graph: G,
    /// The ports that have been visited.
    /// Ports in `direction` are marked when their node is visited. Ports in the
    /// reverse direction are marked once all their unfiltered links have been
    /// visited, or when their node is added as a source.
    visited_ports: Map,
    /// A VecDeque is used for the node list to produce a canonical ordering,
    /// as successors of nodes already have a canonical ordering due to ports.
    candidate_nodes: VecDeque<NodeIndex<G::NodeIndexBase>>,
    direction: Direction,
    /// The number of nodes already returned from the iterator.
    /// This is used to calculate the upper bound for the iterator's `size_hint`.
    nodes_seen: usize,
    /// A filter closure for the nodes to visit. If the closure returns false,
    /// the node is skipped.
    #[allow(clippy::type_complexity)]
    node_filter: Option<Box<dyn FnMut(NodeIndex<G::NodeIndexBase>) -> bool + 'f>>,
    /// A filter closure for the ports to visit. If the closure returns false,
    /// the port is skipped.
    #[allow(clippy::type_complexity)]
    port_filter: Option<
        Box<dyn FnMut(NodeIndex<G::NodeIndexBase>, PortIndex<G::PortIndexBase>) -> bool + 'f>,
    >,
}

impl<'f, Map, G> TopoSort<'f, G, Map>
where
    Map: SecondaryMap<PortIndex<G::PortIndexBase>, bool>,
    G: LinkView,
{
    /// Initialises a new topological sort of a portgraph in a specified direction
    /// starting at a collection of `source` nodes.
    ///
    /// See [`toposort`] and [`toposort_filtered`] for more information.
    ///
    /// If the default value of `Map` is not `false`, this requires O(#ports) time.
    #[allow(clippy::type_complexity)]
    pub fn new(
        graph: G,
        source: impl IntoIterator<Item = NodeIndex<G::NodeIndexBase>>,
        direction: Direction,
        mut node_filter: Option<Box<dyn FnMut(NodeIndex<G::NodeIndexBase>) -> bool + 'f>>,
        port_filter: Option<
            Box<dyn FnMut(NodeIndex<G::NodeIndexBase>, PortIndex<G::PortIndexBase>) -> bool + 'f>,
        >,
    ) -> Self {
        let mut visited_ports: Map = SecondaryMap::new();

        let candidate_nodes: VecDeque<_> = if let Some(node_filter) = node_filter.as_mut() {
            source.into_iter().filter(|&n| node_filter(n)).collect()
        } else {
            source.into_iter().collect()
        };

        // If the default value of `Map` is not `false`, we must mark all ports as not visited.
        if visited_ports.default_value() {
            for port in graph.ports_iter() {
                visited_ports.set(port, false);
            }
        }

        // Mark all the candidate ports as visited, so we don't visit them again.
        for node in candidate_nodes.iter() {
            for port in graph.ports(*node, direction.reverse()) {
                visited_ports.set(port, true);
            }
        }

        Self {
            graph,
            visited_ports,
            candidate_nodes,
            direction,
            nodes_seen: 0,
            node_filter,
            port_filter,
        }
    }

    /// Returns whether there are ports that have not been visited yet.
    /// If the iterator has seen all nodes this implies that there is a cycle.
    pub fn ports_remaining(&self) -> impl Iterator<Item = PortIndex<G::PortIndexBase>> + '_ {
        self.graph
            .ports_iter()
            .filter(move |&p| !self.visited_ports.get(p))
    }

    /// Add more source nodes to the iterator.
    ///
    /// Nodes that have already been yielded will not be returned again.
    pub fn add_sources(&mut self, sources: impl IntoIterator<Item = NodeIndex<G::NodeIndexBase>>) {
        for node in sources.into_iter() {
            if self.ignore_node(node) {
                continue;
            };

            let mut new_candidate = false;
            for port in self.graph.ports(node, self.direction.reverse()) {
                new_candidate |= !self.visited_ports.get(port);
                self.visited_ports.set(port, true);
            }
            if self.graph.num_ports(node, self.direction.reverse()) == 0 {
                // If the node has no incoming ports, we check if it has been visited by looking at the outgoing ports.
                new_candidate = !self.candidate_nodes.contains(&node)
                    && self
                        .graph
                        .ports(node, self.direction)
                        .any(|p| !self.visited_ports.get(p));
            }

            if new_candidate {
                self.candidate_nodes.push_back(node);
            }
        }
    }

    /// Checks if a node becomes ready when reached through `from_port`, i.e.
    /// all its unfiltered predecessor links have been visited.
    ///
    /// If `from_port` is already marked as visited, the node cannot become ready
    /// again. This avoids adding the same node to the candidate list twice.
    fn becomes_ready(
        &mut self,
        node: NodeIndex<G::NodeIndexBase>,
        from_port: PortIndex<G::PortIndexBase>,
    ) -> bool {
        if *self.visited_ports.get(from_port) || self.ignore_node(node) {
            return false;
        }
        let ports: Vec<_> = self.graph.ports(node, self.direction.reverse()).collect();
        ports.into_iter().all(|p| self.port_ready(node, p))
    }

    /// Checks if all links to a port have been visited or filtered out.
    ///
    /// A port with multiple links is only marked as visited once all its links
    /// are ready. We check the ports at the other end of each link to keep track
    /// of which predecessors have been visited.
    fn port_ready(
        &mut self,
        node: NodeIndex<G::NodeIndexBase>,
        port: PortIndex<G::PortIndexBase>,
    ) -> bool {
        if *self.visited_ports.get(port) {
            return true;
        }
        let ready = self.ignore_port(node, port) || {
            let predecessors: SmallVec<[PortIndex<G::PortIndexBase>; 2]> =
                self.graph.port_links(port).map(|(_, p)| p.into()).collect();
            predecessors.into_iter().all(|p| {
                *self.visited_ports.get(p) || self.ignore_port(self.graph.port_node(p).unwrap(), p)
            })
        };
        if ready {
            self.visited_ports.set(port, true);
        }
        ready
    }

    /// Returns `true` if the node should be ignored.
    #[inline]
    fn ignore_node(&mut self, node: NodeIndex<G::NodeIndexBase>) -> bool {
        !self
            .node_filter
            .as_mut()
            .map_or(true, |filter| filter(node))
    }

    /// Returns `true` if the port should be ignored.
    #[inline]
    fn ignore_port(
        &mut self,
        node: NodeIndex<G::NodeIndexBase>,
        port: PortIndex<G::PortIndexBase>,
    ) -> bool {
        !self
            .port_filter
            .as_mut()
            .map_or(true, |filter| filter(node, port))
    }
}

impl<Map, G> Iterator for TopoSort<'_, G, Map>
where
    Map: SecondaryMap<PortIndex<G::PortIndexBase>, bool>,
    G: LinkView,
{
    type Item = NodeIndex<G::NodeIndexBase>;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.candidate_nodes.pop_front()?;
        let ports = self.graph.ports(node, self.direction).collect::<Vec<_>>();

        for port in ports {
            self.visited_ports.set(port, true);

            if self.ignore_port(node, port) {
                continue;
            }

            let linked_ports: SmallVec<[PortIndex<G::PortIndexBase>; 2]> = self
                .graph
                .port_links(port)
                .map(|(_, next_port)| next_port.into())
                .collect();
            for port in linked_ports {
                let target = self.graph.port_node(port).unwrap();

                if !self.ignore_port(target, port) && self.becomes_ready(target, port) {
                    self.candidate_nodes.push_back(target);
                }
            }
        }

        self.nodes_seen += 1;
        Some(node)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (
            self.candidate_nodes.len(),
            Some(self.graph.node_count() - self.nodes_seen),
        )
    }
}

impl<Map, G> FusedIterator for TopoSort<'_, G, Map>
where
    Map: SecondaryMap<PortIndex<G::PortIndexBase>, bool>,
    G: LinkView,
{
}

impl<Map, G> Debug for TopoSort<'_, G, Map>
where
    Map: Debug,
    G: Debug + PortView,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TopoSort")
            .field("graph", &self.graph)
            .field("visited_ports", &self.visited_ports)
            .field("candidate_nodes", &self.candidate_nodes)
            .field("direction", &self.direction)
            .field("nodes_seen", &self.nodes_seen)
            .finish()
    }
}
#[cfg(test)]
mod test {
    use std::collections::BTreeSet;

    use rstest::rstest;

    use super::*;

    use crate::{Direction, LinkMut, PortMut, PortView};

    type PortGraph = crate::PortGraph<u32, u32, u16>;
    type MultiPortGraph = crate::MultiPortGraph<u32, u32, u16>;

    /// Check that the producer is not visited if one of its successors is
    /// unreachable, even when both links share a port.
    #[rstest]
    #[cfg_attr(miri, ignore = "bitvec 1.1.1 fails Miri Stacked Borrows checks")]
    fn toposort_merge(#[values(false, true)] same_port: bool) {
        let mut graph = MultiPortGraph::new();
        let producer = graph.add_node(0, 2);
        let active = graph.add_node(1, 1);
        let inactive = graph.add_node(1, 1);
        let root = graph.add_node(2, 0);
        graph.link_nodes(producer, 0, active, 0).unwrap();
        graph
            .link_nodes(producer, usize::from(!same_port), inactive, 0)
            .unwrap();
        graph.link_nodes(active, 0, root, 0).unwrap();
        graph.link_nodes(inactive, 0, root, 1).unwrap();

        let topo = toposort_filtered::<_, BitVec>(
            &graph,
            [root],
            Direction::Incoming,
            |_| true,
            |_, port| Some(port) != graph.input(root, 1),
        );
        assert_eq!(topo.collect::<Vec<_>>(), [root, active]);
    }

    /// Check that a merge node is only visited after both paths leading to it.
    /// If one source is missing, the merge is not visited until it is added.
    #[rstest]
    #[cfg_attr(miri, ignore = "bitvec 1.1.1 fails Miri Stacked Borrows checks")]
    fn multi_merge(
        #[values(Direction::Incoming, Direction::Outgoing)] direction: Direction,
        #[values(false, true)] same_port: bool,
        #[values(false, true)] include_slow: bool,
    ) {
        let mut graph = MultiPortGraph::new();
        let [fast, slow, middle, merge] = std::array::from_fn(|_| graph.add_node(2, 2));
        let edges = [
            (fast, 0, merge, 0),
            (slow, 0, middle, 0),
            (middle, 0, merge, usize::from(!same_port)),
        ];
        for (from, output, to, input) in edges {
            let (from, output, to, input) = if direction == Direction::Outgoing {
                (from, output, to, input)
            } else {
                (to, input, from, output)
            };
            graph.link_nodes(from, output, to, input).unwrap();
        }
        let roots = if include_slow {
            vec![fast, slow]
        } else {
            vec![fast]
        };
        let mut topo: TopoSort<_> = toposort(&graph, roots, direction);
        let expected = if include_slow {
            vec![fast, slow, middle, merge]
        } else {
            vec![fast]
        };
        assert_eq!(topo.by_ref().collect::<Vec<_>>(), expected);
        if !include_slow {
            let merge_port = graph
                .ports(merge, direction.reverse())
                .nth(usize::from(!same_port))
                .unwrap();
            assert!(topo.ports_remaining().any(|p| p == merge_port));
            topo.add_sources([fast, slow]);
            assert_eq!(topo.by_ref().collect::<Vec<_>>(), [slow, middle, merge]);
        }
        assert_eq!(topo.ports_remaining().count(), 0);
        topo.add_sources([fast, slow, middle, merge]);
        assert_eq!(topo.next(), None);
    }

    /// Check that a cycle is not traversed even when its links share a port
    /// with a link from outside the cycle. Adding a source inside it allows
    /// the remaining nodes to be visited.
    #[rstest]
    #[cfg_attr(miri, ignore = "bitvec 1.1.1 fails Miri Stacked Borrows checks")]
    fn multi_cycle(
        #[values(Direction::Incoming, Direction::Outgoing)] direction: Direction,
        #[values(false, true)] same_port: bool,
    ) {
        let mut graph = MultiPortGraph::new();
        let [root, a, b] = std::array::from_fn(|_| graph.add_node(2, 2));
        for (from, to, offset) in [(root, a, 0), (b, a, usize::from(!same_port)), (a, b, 0)] {
            let from_port = graph.ports(from, direction).next().unwrap();
            let to_port = graph.ports(to, direction.reverse()).nth(offset).unwrap();
            graph.link_ports(from_port, to_port).unwrap();
        }
        let mut topo: TopoSort<_> = toposort(&graph, [root], direction);
        assert_eq!(topo.by_ref().collect::<Vec<_>>(), [root]);
        topo.add_sources([a]);
        assert_eq!(topo.collect::<Vec<_>>(), [a, b]);
    }

    /// Check that filtering a predecessor prevents visiting the merge node,
    /// unless the connecting port is also filtered out.
    #[rstest]
    #[cfg_attr(miri, ignore = "bitvec 1.1.1 fails Miri Stacked Borrows checks")]
    fn filtered_predecessor(
        #[values(Direction::Incoming, Direction::Outgoing)] direction: Direction,
        #[values(false, true)] filter_port: bool,
    ) {
        let mut graph = MultiPortGraph::new();
        let [active, inactive, merge] = std::array::from_fn(|_| graph.add_node(1, 1));
        let merge_port = graph.ports(merge, direction.reverse()).next().unwrap();
        let inactive_port = graph.ports(inactive, direction).next().unwrap();
        for node in [active, inactive] {
            let from_port = graph.ports(node, direction).next().unwrap();
            graph.link_ports(from_port, merge_port).unwrap();
        }
        let topo: TopoSort<_> = toposort_filtered(
            &graph,
            [active, inactive],
            direction,
            |n| n != inactive,
            |_, p| !filter_port || p != inactive_port,
        );
        let expected = if filter_port {
            vec![active, merge]
        } else {
            vec![active]
        };
        assert_eq!(topo.collect::<Vec<_>>(), expected);
    }

    /// Check that filtering either end of a link prevents traversing it and
    /// allows its target to be visited via other links.
    #[rstest]
    #[case::single(PortGraph::new())]
    #[case::multi(MultiPortGraph::new())]
    #[cfg_attr(miri, ignore = "bitvec 1.1.1 fails Miri Stacked Borrows checks")]
    fn filtered_endpoints<G: LinkMut<NodeIndexBase = u32, PortIndexBase = u32>>(
        #[case] mut graph: G,
        #[values(Direction::Incoming, Direction::Outgoing)] direction: Direction,
        #[values(false, true)] target_end: bool,
    ) {
        let [active, inactive, merge] = std::array::from_fn(|_| graph.add_node(2, 2));
        let active_port = graph.ports(active, direction).next().unwrap();
        let inactive_port = graph.ports(inactive, direction).next().unwrap();
        let merge_active = graph.ports(merge, direction.reverse()).next().unwrap();
        let merge_inactive = graph.ports(merge, direction.reverse()).nth(1).unwrap();
        graph.link_ports(active_port, merge_active).unwrap();
        graph.link_ports(inactive_port, merge_inactive).unwrap();

        // A filtered edge must not block a merge reached along another edge.
        let filtered = if target_end {
            merge_inactive
        } else {
            inactive_port
        };
        let topo: TopoSort<_> =
            toposort_filtered(&graph, [active], direction, |_| true, |_, p| p != filtered);
        assert_eq!(topo.collect::<Vec<_>>(), [active, merge]);

        // A filtered edge must not be traversed even when its source is visited.
        let filtered = if target_end {
            merge_active
        } else {
            active_port
        };
        let topo: TopoSort<_> = toposort_filtered(
            &graph,
            [active],
            direction,
            |_| true,
            |_, p| p != filtered && p != merge_inactive,
        );
        assert_eq!(topo.collect::<Vec<_>>(), [active]);
    }

    /// Check all four-node DAGs and source sets. Sharing input or output ports
    /// should not change which nodes are visited or break the topological order.
    #[rstest]
    #[cfg_attr(miri, ignore = "bitvec 1.1.1 fails Miri Stacked Borrows checks")]
    fn small_dags(
        #[values(Direction::Incoming, Direction::Outgoing)] direction: Direction,
        #[values(false, true)] shared_inputs: bool,
        #[values(false, true)] shared_outputs: bool,
    ) {
        let possible_edges = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
        for edge_mask in 0..1 << possible_edges.len() {
            let mut graph = MultiPortGraph::new();
            let nodes: [_; 4] = std::array::from_fn(|_| graph.add_node(4, 4));
            let edges: Vec<_> = possible_edges
                .iter()
                .enumerate()
                .filter(|(i, _)| edge_mask & (1 << i) != 0)
                .map(|(_, &(a, b))| (a, b))
                .collect();
            for &(a, b) in &edges {
                graph
                    .link_nodes(
                        nodes[a],
                        if shared_outputs { 0 } else { b },
                        nodes[b],
                        if shared_inputs { 0 } else { a },
                    )
                    .unwrap();
            }
            for root_mask in 0..16 {
                let mut expected: BTreeSet<_> =
                    (0..4).filter(|i| root_mask & (1 << i) != 0).collect();
                let roots: Vec<_> = expected.iter().map(|&i| nodes[i]).collect();
                loop {
                    let old_len = expected.len();
                    for node in 0..4 {
                        let predecessors: Vec<_> = edges
                            .iter()
                            .filter_map(|&(a, b)| {
                                let (a, b) = if direction == Direction::Outgoing {
                                    (a, b)
                                } else {
                                    (b, a)
                                };
                                (b == node).then_some(a)
                            })
                            .collect();
                        if !predecessors.is_empty()
                            && predecessors.iter().all(|p| expected.contains(p))
                        {
                            expected.insert(node);
                        }
                    }
                    if expected.len() == old_len {
                        break;
                    }
                }
                let topo: TopoSort<_> = toposort(&graph, roots.clone(), direction);
                let order = topo.collect::<Vec<_>>();
                let actual: BTreeSet<_> = order.iter().copied().collect();
                assert_eq!(actual, expected.iter().map(|&i| nodes[i]).collect());
                assert_eq!(order.len(), actual.len(), "duplicate nodes");
                for &(a, b) in &edges {
                    let (a, b) = if direction == Direction::Outgoing {
                        (a, b)
                    } else {
                        (b, a)
                    };
                    if let Some(b_pos) = order.iter().position(|&n| n == nodes[b]) {
                        if !roots.contains(&nodes[b]) {
                            let a_pos = order.iter().position(|&n| n == nodes[a]).unwrap();
                            assert!(a_pos < b_pos, "predecessor emitted after merge");
                        }
                    }
                }
            }
        }
    }

    #[test]
    #[cfg_attr(miri, ignore = "bitvec 1.1.1 fails Miri Stacked Borrows checks")]
    fn small_toposort() {
        let mut graph: PortGraph = PortGraph::new();
        let node_a = graph.add_node(2, 3);
        let node_b = graph.add_node(3, 2);
        let node_c = graph.add_node(3, 2);
        let node_d = graph.add_node(3, 2);
        let node_e = graph.add_node(3, 2);

        // Add two edges between node A and B
        graph.link_nodes(node_a, 0, node_b, 0).unwrap();
        graph.link_nodes(node_a, 1, node_b, 1).unwrap();
        graph.link_nodes(node_a, 2, node_e, 0).unwrap();
        graph.link_nodes(node_b, 0, node_c, 0).unwrap();
        graph.link_nodes(node_c, 0, node_d, 0).unwrap();

        // Run a topological sort on the graph starting at node A.
        let topo: TopoSort<_> = toposort(&graph, [node_a, node_d], Direction::Outgoing);
        assert_eq!(
            topo.collect::<Vec<_>>(),
            [node_a, node_d, node_b, node_e, node_c]
        );

        let topo_filtered: TopoSort<_> = toposort_filtered(
            &graph,
            [node_a, node_d],
            Direction::Outgoing,
            |n| ![node_d, node_e].contains(&n),
            |_, p| Some(p) != graph.output(node_b, 0),
        );
        assert_eq!(topo_filtered.collect::<Vec<_>>(), [node_a, node_b]);
    }

    #[test]
    #[cfg_attr(miri, ignore = "bitvec 1.1.1 fails Miri Stacked Borrows checks")]
    fn test_toposort_multi_port() {
        let mut graph: MultiPortGraph = MultiPortGraph::new();
        let node_a = graph.add_node(0, 1);
        let node_b = graph.add_node(1, 0);
        let node_c = graph.add_node(1, 0);
        graph.link_nodes(node_a, 0, node_b, 0).unwrap();
        graph.link_nodes(node_a, 0, node_c, 0).unwrap();

        let topo: TopoSort<_> = toposort(&graph, [node_a], Direction::Outgoing);
        assert_eq!(
            topo.collect::<BTreeSet<_>>(),
            BTreeSet::from_iter([node_a, node_b, node_c])
        );
    }
}
