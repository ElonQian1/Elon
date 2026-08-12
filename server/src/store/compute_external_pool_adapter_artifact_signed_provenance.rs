//! Transactional verifier and immutable receipt for exact Artifact signed provenance.

mod read;
mod types;
mod write;

pub(in crate::store) use read::current_external_pool_adapter_artifact_signed_provenance_authority_on;
pub(in crate::store) use read::external_pool_adapter_artifact_signed_provenance_authority_on;
pub(in crate::store) use types::ExternalPoolAdapterArtifactSignedProvenanceAuthority;
pub(crate) use types::{
    CreateExternalPoolAdapterArtifactSignedProvenance,
    ExternalPoolAdapterArtifactSignedProvenanceCurrentnessReceipt,
    ExternalPoolAdapterArtifactSignedProvenanceSummary,
    ExternalPoolAdapterArtifactSignedProvenanceWriteReceipt,
    GetExternalPoolAdapterArtifactSignatureChallenge,
};
