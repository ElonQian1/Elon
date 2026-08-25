//! Canonical, API-free projection of the local sources that may eventually support one
//! `user_node` ReadyCapability.
//!
//! The projection is deliberately untrusted. It preserves work-admission, local ready-health and
//! caller-supplied Host runtime observations while naming the missing local-currentness,
//! runtime-transition, Host-runtime and v15-session authorities. It cannot mint Ready, activate a
//! Provider, route or dispatch work, create an Offer or Lease, or produce downstream effects.

#[path = "user_node_ready_source_lineage/canonical.rs"]
mod canonical;
#[path = "user_node_ready_source_lineage/source_equations.rs"]
mod source_equations;
#[path = "user_node_ready_source_lineage/source_inputs.rs"]
mod source_inputs;
#[path = "user_node_ready_source_lineage/types.rs"]
mod types;
#[path = "user_node_ready_source_lineage/validation.rs"]
mod validation;

pub(crate) use canonical::{
    canonical_compute_user_node_ready_source_lineage_json_and_digest,
    compute_user_node_ready_source_lineage_from_json,
    project_untrusted_compute_user_node_host_runtime_observation,
};
pub(crate) use source_equations::{
    build_compute_user_node_ready_source_lineage,
    validate_compute_user_node_ready_source_lineage_against_sources,
};
pub(crate) use source_inputs::{
    ComputeUserNodeReadySourceLineageSources, UntrustedComputeUserNodeHostRuntimeObservationDraftV1,
};
pub(crate) use types::*;
pub(crate) use validation::{
    validate_compute_user_node_ready_source_lineage,
    validate_untrusted_compute_user_node_host_runtime_observation,
};
