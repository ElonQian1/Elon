//! Immutable V233 receipts for deterministic SBOM and local static safety inspection.

mod read;
mod types;
mod write;

pub(in crate::store) use read::{
    current_artifact_security_authority_on, historical_artifact_security_authority_on,
};
pub(crate) use types::{
    CreateExternalPoolAdapterArtifactSecurityReceipt,
    ExternalPoolAdapterArtifactSecurityCurrentnessReceipt,
    ExternalPoolAdapterArtifactSecurityScanTarget, ExternalPoolAdapterArtifactSecuritySummary,
    ExternalPoolAdapterArtifactSecurityWriteReceipt,
};
