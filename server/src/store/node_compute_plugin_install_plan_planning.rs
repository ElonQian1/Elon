//! Exact cloud custody for Planning Snapshot V2 and non-signing generation outcomes.

mod digest;
mod readback;
mod source;
mod types;
mod validation;
mod write;

pub(crate) use types::{
    DurableComputePluginInstallPlanGenerationOutcomeV1,
    DurableComputePluginInstallPlanGenerationRequestV1,
    DurableComputePluginInstallPlanPlanningSnapshotV2,
    NodeComputePluginInstallPlanPlanningDispatchIntentV2, PlanningSnapshotObservationCommitV2,
};

const GENERATION_REQUEST_SCHEMA_V1: &str = "elon.compute_plugin.install_plan_generation_request.v1";
const GENERATION_OUTCOME_SCHEMA_V1: &str = "elon.compute_plugin.install_plan_generation_outcome.v1";
const PLANNING_DELIVERY_REQUEST_SCHEMA_V2: &str =
    "elon.compute_plugin.install_plan_planning_delivery_request.v2";
const GENERATION_SIGNER_PROFILE_V2: &str = "control_install_plan_v2";
const PLANNING_CANONICALIZATION: &str = "rfc8785_jcs";
const PLANNING_DIGEST_ALGORITHM: &str = "sha256";
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
const MAX_LEDGER_JSON_BYTES: usize =
    homecli_proto::MAX_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_BYTES;
