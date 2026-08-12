//! Transactional authority for the external-pool Adapter Artifact signer key registry.

mod read;
mod types;
mod write;

#[cfg(test)]
#[path = "compute_external_pool_adapter_artifact_signing_key_tests.rs"]
mod tests;

pub(crate) use types::{
    ActivateExternalPoolAdapterArtifactSigningKey,
    ExternalPoolAdapterArtifactSigningKeyActivationWriteReceipt,
    ExternalPoolAdapterArtifactSigningKeyCurrentnessReceipt,
    ExternalPoolAdapterArtifactSigningKeyRegistrationWriteReceipt,
    ExternalPoolAdapterArtifactSigningKeyRevocationWriteReceipt,
    RegisterExternalPoolAdapterArtifactSigningKey, RevokeExternalPoolAdapterArtifactSigningKey,
};

pub(in crate::store) use read::current_external_pool_adapter_artifact_signing_key_authority_on;
pub(in crate::store) use read::external_pool_adapter_artifact_signing_key_record_authority_on;
pub(in crate::store) use types::CurrentExternalPoolAdapterArtifactSigningKeyAuthority;
pub(in crate::store) use types::ExternalPoolAdapterArtifactSigningKeyRecordAuthority;
