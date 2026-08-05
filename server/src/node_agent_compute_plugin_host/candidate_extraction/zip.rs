mod extract;
mod revalidate;
mod scan;
mod types;

pub(in crate::node_agent_compute_plugin_host) use extract::extract_verified_compute_plugin_zip_archive;
pub(in crate::node_agent_compute_plugin_host) use scan::scan_verified_compute_plugin_zip_archive;
pub(in crate::node_agent_compute_plugin_host) use types::{
    ComputePluginArchiveExtractionFailure, ExtractedComputePluginCandidateArchive,
};
