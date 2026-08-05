mod authorization;
mod capability;
mod revalidation;

pub(in crate::node_agent_compute_plugin_host) use authorization::{
    authorize_revalidated_candidate_staging, CandidateStagingAuthorityBindingFailure,
    CandidateStagingAuthorityBindingPhase,
};
pub(in crate::node_agent_compute_plugin_host) use capability::{
    AuthorizedComputePluginCandidateStaging, RevalidatedComputePluginCandidateStaging,
};
pub(in crate::node_agent_compute_plugin_host) use revalidation::{
    revalidate_extracted_candidate_for_staging, CandidateStagingRevalidationFailure,
    CandidateStagingRevalidationPhase,
};
