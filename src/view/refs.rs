//! Trait implementations for references
//!
//! Note: The `auto_impl` crate would do all of this for us,
//! but it does not support GATs at the moment.

use crate::{Direction, LinkError, NodeIndex, PortIndex, PortOffset};

use delegate::delegate;

use super::{LinkMut, LinkView, MultiMut, MultiView, PortMut, PortView};

impl<G: PortView> PortView for &G {
    type NodeIndexBase = G::NodeIndexBase;
    type PortIndexBase = G::PortIndexBase;
    type PortOffsetBase = G::PortOffsetBase;

    delegate! {
        to (*self) {
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
            fn contains_node(&self, node: NodeIndex<Self::NodeIndexBase>) -> bool;
            fn contains_port(&self, port: PortIndex<Self::PortIndexBase>) -> bool;
            fn is_empty(&self) -> bool;
            fn node_count(&self) -> usize;
            fn port_count(&self) -> usize;
            fn nodes_iter(&self) -> impl Iterator<Item = NodeIndex<Self::NodeIndexBase>> + Clone;
            fn ports_iter(&self) -> impl Iterator<Item = PortIndex<Self::PortIndexBase>> + Clone;
            fn node_capacity(&self) -> usize;
            fn port_capacity(&self) -> usize;
            fn node_port_capacity(&self, node: NodeIndex<Self::NodeIndexBase>) -> usize;
        }
    }
}

impl<G: PortView> PortView for &mut G {
    type NodeIndexBase = G::NodeIndexBase;
    type PortIndexBase = G::PortIndexBase;
    type PortOffsetBase = G::PortOffsetBase;

    delegate! {
        to (&**self) {
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
            fn contains_node(&self, node: NodeIndex<Self::NodeIndexBase>) -> bool;
            fn contains_port(&self, port: PortIndex<Self::PortIndexBase>) -> bool;
            fn is_empty(&self) -> bool;
            fn node_count(&self) -> usize;
            fn port_count(&self) -> usize;
            fn nodes_iter(&self) -> impl Iterator<Item = NodeIndex<Self::NodeIndexBase>> + Clone;
            fn ports_iter(&self) -> impl Iterator<Item = PortIndex<Self::PortIndexBase>> + Clone;
            fn node_capacity(&self) -> usize;
            fn port_capacity(&self) -> usize;
            fn node_port_capacity(&self, node: NodeIndex<Self::NodeIndexBase>) -> usize;
        }
    }
}

impl<G: LinkView> LinkView for &G {
    type LinkEndpoint = G::LinkEndpoint;

    delegate! {
        to (*self) {
            fn get_connections(&self, from: NodeIndex<Self::NodeIndexBase>, to: NodeIndex<Self::NodeIndexBase>) -> impl Iterator<Item = (Self::LinkEndpoint, Self::LinkEndpoint)> + Clone;
            fn port_links(&self, port: PortIndex<Self::PortIndexBase>) -> impl Iterator<Item = (Self::LinkEndpoint, Self::LinkEndpoint)> + Clone;
            fn links(&self, node: NodeIndex<Self::NodeIndexBase>, direction: Direction) -> impl Iterator<Item = (Self::LinkEndpoint, Self::LinkEndpoint)> + Clone;
            fn all_links(&self, node: NodeIndex<Self::NodeIndexBase>) -> impl Iterator<Item = (Self::LinkEndpoint, Self::LinkEndpoint)> + Clone;
            fn neighbours(&self, node: NodeIndex<Self::NodeIndexBase>, direction: Direction) -> impl Iterator<Item = NodeIndex<Self::NodeIndexBase>> + Clone;
            fn all_neighbours(&self, node: NodeIndex<Self::NodeIndexBase>) -> impl Iterator<Item = NodeIndex<Self::NodeIndexBase>> + Clone;
            fn link_count(&self) -> usize;
        }
    }
}

impl<G: LinkView> LinkView for &mut G {
    type LinkEndpoint = G::LinkEndpoint;

    delegate! {
        to (&**self) {
            fn get_connections(&self, from: NodeIndex<Self::NodeIndexBase>, to: NodeIndex<Self::NodeIndexBase>) -> impl Iterator<Item = (Self::LinkEndpoint, Self::LinkEndpoint)> + Clone;
            fn port_links(&self, port: PortIndex<Self::PortIndexBase>) -> impl Iterator<Item = (Self::LinkEndpoint, Self::LinkEndpoint)> + Clone;
            fn links(&self, node: NodeIndex<Self::NodeIndexBase>, direction: Direction) -> impl Iterator<Item = (Self::LinkEndpoint, Self::LinkEndpoint)> + Clone;
            fn all_links(&self, node: NodeIndex<Self::NodeIndexBase>) -> impl Iterator<Item = (Self::LinkEndpoint, Self::LinkEndpoint)> + Clone;
            fn neighbours(&self, node: NodeIndex<Self::NodeIndexBase>, direction: Direction) -> impl Iterator<Item = NodeIndex<Self::NodeIndexBase>> + Clone;
            fn all_neighbours(&self, node: NodeIndex<Self::NodeIndexBase>) -> impl Iterator<Item = NodeIndex<Self::NodeIndexBase>> + Clone;
            fn link_count(&self) -> usize;
        }
    }
}

impl<G: MultiView> MultiView for &G {
    delegate! {
        to (*self) {
            fn subports(&self, node: NodeIndex<Self::NodeIndexBase>, direction: Direction) -> impl Iterator<Item = Self::LinkEndpoint> + Clone;
            fn all_subports(&self, node: NodeIndex<Self::NodeIndexBase>) -> impl Iterator<Item = Self::LinkEndpoint> + Clone;
            fn subport_link(&self, subport: Self::LinkEndpoint) -> Option<Self::LinkEndpoint>;
        }
    }
}

impl<G: MultiView> MultiView for &mut G {
    delegate! {
        to (&**self) {
            fn subports(&self, node: NodeIndex<Self::NodeIndexBase>, direction: Direction) -> impl Iterator<Item = Self::LinkEndpoint> + Clone;
            fn all_subports(&self, node: NodeIndex<Self::NodeIndexBase>) -> impl Iterator<Item = Self::LinkEndpoint> + Clone;
            fn subport_link(&self, subport: Self::LinkEndpoint) -> Option<Self::LinkEndpoint>;
        }
    }
}

impl<G: PortMut> PortMut for &mut G {
    delegate! {
        to (*self) {
            fn add_node(&mut self, incoming: usize, outgoing: usize) -> NodeIndex<Self::NodeIndexBase>;
            fn remove_node(&mut self, node: NodeIndex<Self::NodeIndexBase>);
            fn clear(&mut self);
            fn reserve(&mut self, nodes: usize, ports: usize);
            fn set_num_ports<F: FnMut(PortIndex<Self::PortIndexBase>, crate::portgraph::PortOperation<Self::PortIndexBase>)>(&mut self, node: NodeIndex<Self::NodeIndexBase>, incoming: usize, outgoing: usize, rekey: F);
            fn swap_nodes(&mut self, a: NodeIndex<Self::NodeIndexBase>, b: NodeIndex<Self::NodeIndexBase>);
            fn compact_nodes<F: FnMut(NodeIndex<Self::NodeIndexBase>, NodeIndex<Self::NodeIndexBase>)>(&mut self, rekey: F);
            fn compact_ports<F: FnMut(PortIndex<Self::PortIndexBase>, PortIndex<Self::PortIndexBase>)>(&mut self, rekey: F);
            fn shrink_to_fit(&mut self);
        }
    }
}

impl<G: LinkMut> LinkMut for &mut G {
    delegate! {
        to (*self) {
            fn link_ports(
                &mut self,
                port_a: PortIndex<Self::PortIndexBase>,
                port_b: PortIndex<Self::PortIndexBase>,
            ) -> Result<(Self::LinkEndpoint, Self::LinkEndpoint), LinkError<Self::NodeIndexBase, Self::PortIndexBase, Self::PortOffsetBase>>;
            fn unlink_port(&mut self, port: PortIndex<Self::PortIndexBase>) -> Option<Self::LinkEndpoint>;
        }
    }
}

impl<G: MultiMut> MultiMut for &mut G {
    delegate! {
        to (*self) {
            fn link_subports(
                &mut self,
                subport_from: Self::LinkEndpoint,
                subport_to: Self::LinkEndpoint,
            ) -> Result<(), LinkError<Self::NodeIndexBase, Self::PortIndexBase, Self::PortOffsetBase>>;
            fn unlink_subport(&mut self, subport: Self::LinkEndpoint) -> Option<Self::LinkEndpoint>;
        }
    }
}
