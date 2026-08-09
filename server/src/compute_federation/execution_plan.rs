//! Immutable, Provider-neutral execution authority projected before Attempt dispatch.
//!
//! The ledger and producer live in `store::compute_attempt_execution_plans`. The verified
//! inputs in this module deliberately have no constructor until the node/endpoint/Adapter and
//! artifact-authority verification paths exist.

mod canonical;
mod types;
mod validated;

pub(crate) use canonical::{
    canonical_artifact_access_json_and_digest, canonical_execution_capability_json_and_digest,
    canonical_execution_plan_json_and_digest, canonical_execution_plan_seal_json_and_digest,
    canonical_input_digest, canonical_plan_access_set_digest,
    canonical_resource_grant_json_and_digest, canonical_workload_spec_digest,
};
pub(crate) use types::*;
pub(crate) use validated::{
    ValidatedComputeAttemptExecutionPlanInputs, VerifiedComputeArtifactAccess,
    VerifiedComputeExecutionCapability,
};
