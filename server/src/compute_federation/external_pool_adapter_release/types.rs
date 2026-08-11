use serde::{Deserialize, Serialize};

pub(crate) const COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_REQUEST_SCHEMA: &str =
    "compute_federation.external_pool_adapter_release_request.v1";
pub(crate) const COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_CONFIRMATION: &str =
    "confirm_external_pool_adapter_release_request";
pub(crate) const COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_ROUTE_KIND: &str = "server_adapter";
pub(crate) const COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_PROVIDER_KIND: &str = "external_pool";

/// Canonical platform request DTO. Serde support grants no admission or execution authority.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeExternalPoolAdapterReleaseRequestEnvelope {
    pub schema: String,
    pub request_id: String,
    pub request_digest: String,
    pub request_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub request: ComputeExternalPoolAdapterReleaseRequest,
}

/// Administrator-declared staging material; independent review and immutable apply are separate.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeExternalPoolAdapterReleaseRequest {
    pub submitted_by_admin_user_id: String,
    pub release: ComputeExternalPoolAdapterReleaseIntent,
    pub idempotency_key: String,
    pub confirmation: String,
    pub submission_note: String,
    pub submitted_at: String,
}

/// Exact candidate metadata only. None of these fields proves downloaded or executable bytes.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeExternalPoolAdapterReleaseIntent {
    pub adapter_id: String,
    pub release_version: String,
    pub route_kind: String,
    pub supported_provider_kinds: Vec<String>,
    pub candidate_artifact_ref: String,
    pub declared_implementation_sha256: String,
    pub supported_capabilities: Vec<ComputeExternalPoolAdapterReleaseCapability>,
    pub capability_set_digest: String,
    pub expected_credential_verifier: ComputeExternalPoolAdapterReleaseVerifierIntent,
}

/// Declared protocol revision in canonical array order, not conformance evidence.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeExternalPoolAdapterReleaseCapability {
    pub capability_id: String,
    pub capability_revision: i64,
}

/// Future verifier binding intent. It does not prove registry presence or currentness.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeExternalPoolAdapterReleaseVerifierIntent {
    pub verification_kind: String,
    pub verifier_id: String,
    pub verifier_revision: i64,
    pub verifier_digest: String,
}
