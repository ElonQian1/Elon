mod extract;
mod revalidate;
mod scan;
mod types;

pub(in crate::node_agent_compute_plugin_host) use extract::extract_verified_compute_plugin_zip_archive;
pub(in crate::node_agent_compute_plugin_host) use scan::scan_verified_compute_plugin_zip_archive;
pub(in crate::node_agent_compute_plugin_host) use types::{
    ComputePluginArchiveExtractionFailure, ComputePluginStagingSealEvidence,
    ExtractedComputePluginCandidateArchive, HashedComputePluginExtractedArchiveEvidence,
    EXTRACTED_ARCHIVE_EVIDENCE_SCHEMA, HASHED_EXTRACTED_ARCHIVE_EVIDENCE_SCHEMA,
    STAGING_EVIDENCE_CANONICALIZATION, STAGING_EVIDENCE_DIGEST_ALGORITHM,
    STAGING_SEAL_EVIDENCE_SCHEMA, STAGING_SEAL_PAYLOAD_SCHEMA,
};
