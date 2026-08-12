//! Transactional verifier and immutable receipt for exact Artifact signed provenance.

mod read;
mod types;
mod write;

pub(crate) use types::{
    CreateExternalPoolAdapterArtifactSignedProvenance,
    ExternalPoolAdapterArtifactSignedProvenanceCurrentnessReceipt,
    ExternalPoolAdapterArtifactSignedProvenanceSummary,
    ExternalPoolAdapterArtifactSignedProvenanceWriteReceipt,
    GetExternalPoolAdapterArtifactSignatureChallenge,
};
