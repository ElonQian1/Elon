mod authorization;
mod capability;
mod recovery_key;
mod revalidation;
mod store;

pub(in crate::node_agent_compute_plugin_host) use authorization::{
    authorize_revalidated_candidate_staging, CandidateStagingAuthorityBindingFailure,
    CandidateStagingAuthorityBindingPhase,
};
pub(in crate::node_agent_compute_plugin_host) use capability::{
    AuthorizedComputePluginCandidateStaging, RevalidatedComputePluginCandidateStaging,
    ValidatedCandidateStagingStorePermit,
};
pub(in crate::node_agent_compute_plugin_host) use recovery_key::ComputePluginCandidateStagingRecoveryKey;
pub(in crate::node_agent_compute_plugin_host) use revalidation::{
    revalidate_extracted_candidate_for_staging, CandidateStagingRevalidationFailure,
    CandidateStagingRevalidationPhase,
};
pub(in crate::node_agent_compute_plugin_host) use store::{
    adopt_recovered_candidate_staging, store_authorized_candidate_staging,
    CandidateStagingOutcomeUncertainCustody, CandidateStagingRecoveryAdoptionFailure,
    CandidateStagingRecoveryAdoptionPhase, CandidateStagingStoreFailure,
    CandidateStagingStorePhase, StagedComputePluginCandidateArchive,
};
