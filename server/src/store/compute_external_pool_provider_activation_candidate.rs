//! Atomic owner delegation and inert activation-candidate storage.

mod audit_candidate;
mod audit_delegation;
mod audit_revocation;
mod build;
mod current;
mod persistence;
mod read;
mod replay;
mod roots;
mod types;
mod write;

pub(crate) mod api {
    pub(crate) use super::types::{
        CreateExternalPoolProviderActivationCandidate,
        ExternalPoolProviderActivationCandidateAuditTarget,
        ExternalPoolProviderActivationCandidateCurrentness,
        ExternalPoolProviderActivationCandidateSummary,
        ExternalPoolProviderActivationCandidateWriteReceipt,
        ExternalPoolProviderActivationDelegationRevocationSummary,
        ExternalPoolProviderActivationDelegationRevocationWriteReceipt,
        ExternalPoolProviderActivationDelegationSummary,
        ExternalPoolProviderActivationPreflightReceipt,
        GetCurrentExternalPoolProviderActivationPreflight,
        RevokeExternalPoolProviderActivationDelegation,
    };
}

pub(in crate::store) use current::current_external_pool_provider_activation_preflight_authority_on;
pub(in crate::store) use types::CurrentExternalPoolProviderActivationPreflightAuthority;
