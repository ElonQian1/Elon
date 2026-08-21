use serde::{Deserialize, Serialize};

pub(crate) const ROLLOUT_PLAN_SCHEMA: &str = "yilong.erp.managed_rollout_plan.v1";
pub(crate) const MANAGED_INSTANCE_SCHEMA: &str = "yilong.managed-merchant-instance.v1";
pub(crate) const EDGE_ROUTE_SCHEMA: &str = "yilong.commerce-edge.route.v1";
pub(crate) const ROLLOUT_STATUS_PLANNED: &str = "planned";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CreateManagedRolloutPlanRequest {
    pub expected_configuration_revision: i64,
    pub merchant_confirmed: bool,
    pub target_node_id: String,
    pub service_user: String,
    pub store_id: String,
    pub profile_source: String,
    pub secrets_source: String,
    pub listen_port: u16,
    pub runtime_key_id: String,
    pub public_base_path: String,
    pub endpoint_base_url: String,
    pub runtime_manifest_sha256: String,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ManagedRolloutSource {
    pub instance_id: String,
    pub instance_key: String,
    pub configuration_revision: i64,
    pub blueprint_version_id: String,
    pub blueprint_version: String,
    pub release_manifest_sha256: String,
    pub merchant_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ManagedMerchantInstanceContract {
    pub schema: String,
    pub instance_id: String,
    pub service_user: String,
    pub merchant_id: String,
    pub store_id: String,
    pub profile_source: String,
    pub secrets_source: String,
    pub listen_port: u16,
    pub runtime_key_id: String,
    pub public_base_path: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ManagedEdgeRoute {
    pub schema: String,
    pub instance_id: String,
    pub public_base_path: String,
    pub upstream_addr: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ManagedRuntimeCandidate {
    pub endpoint_base_url: String,
    pub credential_ref: String,
    pub manifest_sha256: String,
    pub timeout_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ManagedRolloutPayload {
    pub schema: String,
    pub source: ManagedRolloutSource,
    pub target_node_id: String,
    pub deployment_contract: ManagedMerchantInstanceContract,
    pub edge_route: ManagedEdgeRoute,
    pub runtime_candidate: ManagedRuntimeCandidate,
    pub boundaries: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct ManagedRolloutPlan {
    pub id: String,
    pub project_id: String,
    pub instance_id: String,
    pub merchant_id: String,
    pub plan_sha256: String,
    pub status: String,
    pub payload: ManagedRolloutPayload,
    pub created_by_user_id: String,
    pub created_at: String,
}

fn default_timeout_ms() -> i64 {
    5_000
}
