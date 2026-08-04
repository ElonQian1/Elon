use serde::Serialize;

pub(crate) const COMPUTE_ACTIVATION_EVIDENCE_REQUEST_SCHEMA: &str =
    "compute_federation.activation_evidence_request.v1";

pub(crate) const ACTIVATION_REQUEST_STATUS_SUBMITTED: &str = "submitted";
pub(crate) const ACTIVATION_REQUEST_STATUS_CHANGES_REQUESTED: &str = "changes_requested";
pub(crate) const ACTIVATION_REQUEST_STATUS_APPROVED: &str = "approved";
pub(crate) const ACTIVATION_REQUEST_STATUS_ACTIVATED: &str = "activated";
pub(crate) const ACTIVATION_REQUEST_STATUS_REJECTED: &str = "rejected";
pub(crate) const ACTIVATION_REQUEST_STATUS_CANCELED: &str = "canceled";
pub(crate) const ACTIVATION_REQUEST_STATUS_SUPERSEDED: &str = "superseded";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeActivationEvidenceRequest {
    pub schema: &'static str,
    pub request_id: String,
    pub provider_id: String,
    pub pool_id: String,
    pub owner_user_id: String,
    pub expected_provider_policy_revision: i64,
    pub expected_provider_digest: String,
    pub expected_capacity_epoch: i64,
    pub expected_pool_revision: i64,
    pub expected_pool_digest: String,
    pub node_binding_ref: String,
    pub ready_capability_digest: String,
    pub route_proof_digest: String,
    pub hardware_observation_digest: String,
    pub ledger_audit_digest: String,
    pub status: String,
    pub request_digest: String,
    pub requested_at: String,
    pub reviewed_at: Option<String>,
    pub reviewed_by_user_id: Option<String>,
    pub review_note: Option<String>,
    pub canceled_at: Option<String>,
    pub superseded_at: Option<String>,
    pub superseded_by_user_id: Option<String>,
    pub supersede_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeActivationEvidenceRequestReceipt {
    pub request: ComputeActivationEvidenceRequest,
    pub replayed: bool,
    pub activation_effect: &'static str,
}
