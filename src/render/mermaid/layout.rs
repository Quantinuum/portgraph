//! Layout algorithms for Mermaid rendering.

use crate::LinkView;

use super::MermaidBuilder;

/// Mermaid graph layout options.
///
/// See <https://mermaid.ai/open-source/config/layouts.html>.
///
/// Defaults to `elk`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MermaidLayout {
    /// The layout engine used by default in mermaid.
    ///
    /// This doesn't require adding a configuration block to the mermaid
    /// definition, but it has limited layout options and may produce less
    /// readable diagrams.
    Dagre,
    /// Eclipse Layout Kernel, a more advanced layout engine with tighter
    /// node placement.
    ///
    /// See <https://eclipse.dev/elk/>
    #[default]
    Elk,
}

impl MermaidLayout {
    /// Returns `True` if the layout config requires an explicit
    /// configuration block in the mermaid definition.
    pub fn requires_config(&self) -> bool {
        match self {
            MermaidLayout::Dagre => false,
            MermaidLayout::Elk => true,
        }
    }

    /// Emit the mermaid configuration block for this layout.
    pub(super) fn emit_config(&self, builder: &mut MermaidBuilder<'_, impl LinkView>) {
        match self {
            MermaidLayout::Dagre => {}
            MermaidLayout::Elk => {
                builder.push_line("layout: elk");
            }
        }
    }
}
