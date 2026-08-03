//! Deterministic offline handoff contract for a future Sui network adapter.

use serde::Serialize;
use serde_json::Value;

pub(super) const SUI_ADAPTER_HANDOFF_SCHEMA: &str = "task_economy.sui_adapter_handoff.v1";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SuiAdapterHandoffConstraints {
    pub allowed_adapter_action: &'static str,
    pub signature_present: bool,
    pub transaction_broadcast: bool,
    pub finality_verified: bool,
    pub funds_moved: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SuiAdapterHandoffPayload {
    pub schema: &'static str,
    pub package_kind: &'static str,
    pub project_id: String,
    pub projection_package_id: String,
    pub source_id: String,
    pub target_network: String,
    pub package_schema: String,
    pub projection_digest: String,
    pub source_digest: String,
    pub envelope: Value,
    pub shadow_only: bool,
    pub atomic_bundle: bool,
    pub network_submission: String,
    pub submission_attempts: i64,
    pub package_created_at: String,
    pub constraints: SuiAdapterHandoffConstraints,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SuiAdapterHandoffBundle {
    #[serde(flatten)]
    pub payload: SuiAdapterHandoffPayload,
    pub handoff_digest: String,
}
