//! Exact parser-evidence locator for one preliminary module-resolution request.

/// Normal and delay imports are located by their import-table descriptor/thunk evidence.
/// Forwarders are located by their source export and hop evidence instead. Keeping the variants
/// mutually exclusive prevents a forwarder from inheriting meaningless descriptor/thunk fields
/// from the import edge that first reached it.
#[derive(Clone, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) enum WindowsPreliminaryModuleEdgeLocator
{
    Import {
        source_import_edge_ordinal: usize,
        descriptor_ordinal: usize,
        thunk_ordinal: usize,
        edge_evidence_digest: String,
    },
    Forwarder {
        source_import_edge_ordinal: usize,
        forwarder_hop_ordinal: usize,
        source_export_name: Option<String>,
        source_export_ordinal: Option<u16>,
        hop_evidence_digest: String,
    },
}

impl WindowsPreliminaryModuleEdgeLocator {
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn source_import_edge_ordinal(
        &self,
    ) -> usize {
        match self {
            Self::Import {
                source_import_edge_ordinal,
                ..
            }
            | Self::Forwarder {
                source_import_edge_ordinal,
                ..
            } => *source_import_edge_ordinal,
        }
    }

    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn forwarder_hop_ordinal(
        &self,
    ) -> Option<usize> {
        match self {
            Self::Import { .. } => None,
            Self::Forwarder {
                forwarder_hop_ordinal,
                ..
            } => Some(*forwarder_hop_ordinal),
        }
    }
}
