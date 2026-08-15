use serde::{Deserialize, Serialize};

use crate::compute_federation::external_pool_adapter_release::ComputeExternalPoolAdapterReleaseCapability;

pub(crate) const RUNTIME_COMPATIBILITY_PROFILE_SCHEMA: &str =
    "compute_federation.external_pool_adapter_runtime_compatibility_profile.v1";
pub(crate) const RUNTIME_COMPATIBILITY_PROFILE_ENVELOPE_SCHEMA: &str =
    "compute_federation.external_pool_adapter_runtime_compatibility_profile_envelope.v1";
pub(crate) const RUNTIME_COMPATIBILITY_CHALLENGE_SCHEMA: &str =
    "compute_federation.external_pool_adapter_runtime_compatibility_challenge.v1";
pub(crate) const RUNTIME_COMPATIBILITY_REPORT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_runtime_compatibility_candidate_report.v1";
pub(crate) const RUNTIME_COMPATIBILITY_PROFILE_ID: &str =
    "external_pool_adapter_linux_runtime_compatibility_v1";
pub(crate) const RUNTIME_COMPATIBILITY_PROFILE_REVISION: u64 = 1;
pub(crate) const RUNTIME_COMPATIBILITY_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const RUNTIME_COMPATIBILITY_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const RUNTIME_COMPATIBILITY_NO_EFFECT: &str = "none";
pub(crate) const RUNTIME_COMPATIBILITY_CANDIDATE_STATUS: &str =
    "unsigned_runtime_compatibility_candidate_no_authority";
pub(crate) const RUNTIME_COMPATIBILITY_EVIDENCE_SCOPE: &str =
    "caller_asserted_public_fixture_observations_not_signature_or_execution_proof_v1";
pub(crate) const MAX_COMPATIBILITY_CHALLENGE_MINUTES: i64 = 10;
pub(crate) const MAX_COMPATIBILITY_RUN_SECONDS: i64 = 30;
pub(crate) const MAX_COMPATIBILITY_REQUEST_BYTES: u64 = 16_384;
pub(crate) const MAX_COMPATIBILITY_RESPONSE_BYTES: u64 = 65_536;
pub(crate) const MAX_COMPATIBILITY_PROBE_TIMEOUT_MS: u64 = 15_000;

pub(crate) const REQUIRED_RUNTIME_OBSERVATIONS: [&str; 8] = [
    "authenticated_bootstrap",
    "config_delivery",
    "credential_delivery",
    "adapter_request_generation",
    "broker_exact_exchange",
    "adapter_response_validation",
    "authenticated_shutdown",
    "bounded_reap",
];

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterCompatibilityPolicyRef {
    pub policy_id: String,
    pub policy_revision: u64,
    pub policy_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterCompatibilityElspProtocol {
    pub protocol_id: String,
    pub protocol_revision: u64,
    pub transport: String,
    pub framing: String,
    pub frame_magic_ascii: String,
    pub frame_header_bytes: u64,
    pub frame_mac_bytes: u64,
    pub frame_kind_control: u64,
    pub frame_kind_config: u64,
    pub frame_kind_credential: u64,
    pub control_encoding: String,
    pub binary_encoding: String,
    pub sequence_policy: String,
    pub mac: String,
    pub config_delivery_kind: String,
    pub credential_delivery_kind: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterCompatibilityElnwProtocol {
    pub frame_kind: String,
    pub magic_ascii: String,
    pub version: u64,
    pub flags: u64,
    pub begin_kind: u64,
    pub request_kind: u64,
    pub response_kind: u64,
    pub receipt_kind: u64,
    pub request_header_bytes: u64,
    pub response_header_bytes: u64,
    pub receipt_bytes: u64,
    pub max_request_bytes: u64,
    pub max_response_bytes: u64,
    pub max_probe_timeout_ms: u64,
    pub root_domain: String,
    pub integer_encoding: String,
    pub completion_policy: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterCompatibilityBrokerProtocol {
    pub transport_owner: String,
    pub transport_kind: String,
    pub tls_version_policy: String,
    pub tls_server_name_policy: String,
    pub tls_leaf_identity_policy: String,
    pub proxy_policy: String,
    pub redirect_policy: String,
    pub zero_rtt_policy: String,
    pub client_certificate_policy: String,
    pub adapter_network_policy: String,
    pub application_exchange_policy: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilityObservationRequirement {
    pub observation_id: String,
    pub observation_revision: u64,
    pub required_outcome: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilityEffects {
    pub conformance_effect: String,
    pub credential_effect: String,
    pub adapter_effect: String,
    pub provider_effect: String,
    pub route_effect: String,
    pub activation_effect: String,
    pub execution_effect: String,
    pub usage_effect: String,
    pub market_effect: String,
    pub settlement_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilityProfile {
    pub schema: String,
    pub profile_id: String,
    pub profile_revision: u64,
    pub host_os: String,
    pub host_arch: String,
    pub release_capabilities: Vec<ComputeExternalPoolAdapterReleaseCapability>,
    pub runtime_launch_policy: ExternalPoolAdapterCompatibilityPolicyRef,
    pub upstream_transport_policy: ExternalPoolAdapterCompatibilityPolicyRef,
    pub supervisor_session_policy: ExternalPoolAdapterCompatibilityPolicyRef,
    pub elsp: ExternalPoolAdapterCompatibilityElspProtocol,
    pub elnw: ExternalPoolAdapterCompatibilityElnwProtocol,
    pub broker: ExternalPoolAdapterCompatibilityBrokerProtocol,
    pub required_observations: Vec<ExternalPoolAdapterRuntimeCompatibilityObservationRequirement>,
    pub candidate_evidence_scope: String,
    pub effects: ExternalPoolAdapterRuntimeCompatibilityEffects,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilityProfileEnvelope {
    pub schema: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub profile_digest: String,
    pub profile: ExternalPoolAdapterRuntimeCompatibilityProfile,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilityChallengeMaterial {
    pub profile_id: String,
    pub profile_revision: u64,
    pub profile_digest: String,
    pub adapter_id: String,
    pub release_version: String,
    pub implementation_sha256: String,
    pub capability_set_digest: String,
    pub runtime_image_digest: String,
    pub challenge_nonce_base64: String,
    pub challenge_nonce_digest: String,
    pub issued_at: String,
    pub expires_at: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilityChallenge {
    pub schema: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub challenge_digest: String,
    pub challenge: ExternalPoolAdapterRuntimeCompatibilityChallengeMaterial,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilityObservation {
    pub observation_id: String,
    pub observation_revision: u64,
    pub outcome: String,
    pub evidence_digest: String,
    pub duration_ms: u64,
    pub policy_violation_count: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilityNoWorkEvidence {
    pub probe_nonce_base64: String,
    pub probe_nonce_digest: String,
    pub request_bytes: u64,
    pub response_bytes: u64,
    pub request_sha256: String,
    pub response_sha256: String,
    pub probe_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilityCandidateMaterial {
    pub verifier_report_id: String,
    pub challenge_digest: String,
    pub profile_digest: String,
    pub adapter_id: String,
    pub release_version: String,
    pub implementation_sha256: String,
    pub capability_set_digest: String,
    pub runtime_image_digest: String,
    pub run_started_at: String,
    pub run_completed_at: String,
    pub child_network_attempt_count: u64,
    pub write_outside_ephemeral_count: u64,
    pub additional_process_attempt_count: u64,
    pub observations: Vec<ExternalPoolAdapterRuntimeCompatibilityObservation>,
    pub no_work: ExternalPoolAdapterRuntimeCompatibilityNoWorkEvidence,
    pub evidence_scope: String,
    pub candidate_status: String,
    pub effects: ExternalPoolAdapterRuntimeCompatibilityEffects,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilityCandidateReport {
    pub schema: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub report_digest: String,
    pub report: ExternalPoolAdapterRuntimeCompatibilityCandidateMaterial,
}
