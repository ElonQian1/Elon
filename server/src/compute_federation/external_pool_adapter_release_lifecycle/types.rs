use serde::{Deserialize, Serialize};

pub(crate) const COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_TERMINAL_RECEIPT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_release_admission_terminal_receipt.v1";
pub(crate) const COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_TERMINAL_CANONICALIZATION: &str =
    "rfc8785_jcs";
pub(crate) const COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_TERMINAL_DIGEST_ALGORITHM: &str =
    "sha256";

pub(crate) const EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_STAGED: &str = "staged";
pub(crate) const EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_WITHDRAWN: &str = "withdrawn";
pub(crate) const EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED: &str = "revoked";
pub(crate) const EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_SUPERSEDED: &str = "superseded";
pub(crate) const EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_ACTOR_PLATFORM_ADMIN: &str =
    "platform_admin";

pub(crate) const EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_WITHDRAWAL_CONFIRMATION: &str =
    "confirm_external_pool_adapter_release_admission_withdrawal";
pub(crate) const EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_REVOCATION_CONFIRMATION: &str =
    "confirm_external_pool_adapter_release_admission_revocation";
pub(crate) const EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_SUPERSESSION_CONFIRMATION: &str =
    "confirm_external_pool_adapter_release_admission_supersession";

pub(crate) const EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_CURRENTNESS_EFFECT: &str =
    "admission_terminal";
pub(crate) const EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_ARTIFACT_INTAKE_EFFECT: &str = "blocked";
pub(crate) const EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_EXISTING_ARTIFACT_SOURCE_EFFECT: &str =
    "historical_only";
pub(crate) const EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_ADAPTER_EFFECT: &str = "none";
pub(crate) const EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_ROUTE_EFFECT: &str = "none";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeExternalPoolAdapterReleaseAdmissionBinding {
    pub admission_id: String,
    pub admission_digest: String,
    pub adapter_id: String,
    pub release_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeExternalPoolAdapterReleaseSuccessorAdmissionBinding {
    pub admission_id: String,
    pub admission_digest: String,
    pub release_version: String,
}

/// Immutable negative authority. A successor remains an independently audited staged candidate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeExternalPoolAdapterReleaseAdmissionTerminal {
    pub admission: ComputeExternalPoolAdapterReleaseAdmissionBinding,
    pub prior_status: String,
    pub terminal_status: String,
    pub successor_admission: Option<ComputeExternalPoolAdapterReleaseSuccessorAdmissionBinding>,
    pub actor_kind: String,
    pub actor_id: String,
    pub reason: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub occurred_at: String,
    pub recorded_at: String,
    pub currentness_effect: String,
    pub artifact_intake_effect: String,
    pub existing_artifact_source_effect: String,
    pub adapter_effect: String,
    pub route_effect: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeExternalPoolAdapterReleaseAdmissionTerminalReceipt {
    pub schema: String,
    pub terminal_receipt_id: String,
    pub terminal_receipt_digest: String,
    pub request_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub terminal: ComputeExternalPoolAdapterReleaseAdmissionTerminal,
}
