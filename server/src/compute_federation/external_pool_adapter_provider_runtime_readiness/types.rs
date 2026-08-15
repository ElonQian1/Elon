use serde::{Deserialize, Serialize};

pub(crate) const PROVIDER_RUNTIME_READINESS_POLICY_SCHEMA: &str =
    "compute_federation.external_pool_adapter_provider_runtime_readiness_policy.v1";
pub(crate) const PROVIDER_RUNTIME_READINESS_POLICY_ENVELOPE_SCHEMA: &str =
    "compute_federation.external_pool_adapter_provider_runtime_readiness_policy_envelope.v1";
pub(crate) const PROVIDER_RUNTIME_READINESS_RECEIPT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_provider_runtime_readiness_receipt.v1";
pub(crate) const PROVIDER_RUNTIME_READINESS_REVOCATION_RECEIPT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_provider_runtime_readiness_revocation_receipt.v1";
pub(crate) const PROVIDER_RUNTIME_READINESS_SUMMARY_SCHEMA: &str =
    "compute_federation.external_pool_adapter_provider_runtime_readiness_summary.v1";
pub(crate) const PROVIDER_RUNTIME_READINESS_CURRENTNESS_SCHEMA: &str =
    "compute_federation.external_pool_adapter_provider_runtime_readiness_currentness.v1";
pub(crate) const PROVIDER_RUNTIME_READINESS_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const PROVIDER_RUNTIME_READINESS_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const PROVIDER_RUNTIME_READINESS_HMAC_ALGORITHM: &str = "hmac-sha256";
pub(crate) const PROVIDER_RUNTIME_READINESS_MAX_RECEIPT_JSON_BYTES: usize = 1024 * 1024;
pub(crate) const PROVIDER_RUNTIME_READINESS_MAX_PROBE_TIMEOUT_MS: u64 = 15_000;
pub(crate) const PROVIDER_RUNTIME_READINESS_MAX_REQUEST_BYTES: u64 = 16_384;
pub(crate) const PROVIDER_RUNTIME_READINESS_MAX_RESPONSE_BYTES: u64 = 65_536;
pub(crate) const PROVIDER_RUNTIME_READINESS_HMAC_KEY_BYTES: usize = 32;
pub(crate) const PROVIDER_RUNTIME_READINESS_CUSTODY_EPOCH_BYTES: usize = 32;

pub(crate) const PROVIDER_RUNTIME_READINESS_CONFIRMATION: &str =
    "confirm_external_pool_adapter_provider_runtime_readiness";
pub(crate) const PROVIDER_RUNTIME_READINESS_REVOCATION_CONFIRMATION: &str =
    "confirm_external_pool_adapter_provider_runtime_readiness_revocation";
pub(crate) const PROVIDER_RUNTIME_READINESS_RECEIPT_STATUS: &str =
    "post_cleanup_runtime_readiness_recorded_no_activation_authority";
pub(crate) const PROVIDER_RUNTIME_READINESS_REVOCATION_STATUS: &str =
    "provider_runtime_readiness_revoked_historical_only";
pub(crate) const PROVIDER_RUNTIME_READINESS_CURRENT_STATUS: &str =
    "current_post_cleanup_runtime_readiness_no_activation_authority";
pub(crate) const PROVIDER_RUNTIME_READINESS_HISTORICAL_STATUS: &str =
    "historical_post_cleanup_runtime_readiness_only";
pub(crate) const PROVIDER_RUNTIME_READINESS_RELATIONAL_CURRENT_STATUS: &str =
    "relationally_current_requires_process_custody_reproof";
pub(crate) const PROVIDER_RUNTIME_READINESS_RELATIONAL_HISTORICAL_STATUS: &str = "historical_only";
pub(crate) const PROVIDER_RUNTIME_READINESS_EVIDENCE_SCOPE: &str =
    "v270_server_owned_provider_specific_authenticated_no_work_post_cleanup_observation_v1";
pub(crate) const PROVIDER_RUNTIME_READINESS_ACTOR_PLATFORM_ADMIN: &str = "platform_admin";
pub(crate) const PROVIDER_RUNTIME_READINESS_ACTOR_PROVIDER_OWNER: &str = "provider_owner";
pub(crate) const PROVIDER_RUNTIME_READINESS_NO_EFFECT: &str = "none";

pub(crate) const PROVIDER_RUNTIME_READINESS_CUSTODY_EPOCH_DIGEST_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-PROVIDER-RUNTIME-READINESS-CUSTODY-EPOCH-DIGEST-V1";
pub(crate) const PROVIDER_RUNTIME_READINESS_BUNDLE_IDENTITY_COMMITMENT_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-PROVIDER-RUNTIME-READINESS-BUNDLE-IDENTITY-HMAC-V1";
pub(crate) const PROVIDER_RUNTIME_READINESS_POST_CLEANUP_COMMITMENT_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-PROVIDER-RUNTIME-READINESS-POST-CLEANUP-HMAC-V1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterProviderRuntimeReadinessEffects {
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
pub(crate) struct ExternalPoolAdapterProviderRuntimeReadinessObservedReadiness {
    pub process_spawn_ready: bool,
    pub ipc_session_ready: bool,
    pub secret_delivery_ready: bool,
    pub broker_connect_ready: bool,
    pub upstream_probe_observed: bool,
    pub runtime_launch_ready: bool,
    pub activation_ready: bool,
}

/// Private keyed commitments. Deliberately has no `Debug` implementation.
#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterProviderRuntimeReadinessSealedBindings {
    pub runtime_custody_epoch_digest: String,
    pub runtime_bundle_identity_commitment: String,
    pub post_cleanup_observation_commitment: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterProviderRuntimeReadinessPolicy {
    pub schema: String,
    pub policy_id: String,
    pub policy_revision: u64,
    pub host_os: String,
    pub host_arch: String,
    pub runtime_kind: String,
    pub trigger_policy: String,
    pub startup_custody_policy: String,
    pub bundle_root_policy: String,
    pub cgroup_parent_policy: String,
    pub hmac_algorithm: String,
    pub hmac_key_policy: String,
    pub custody_epoch_policy: String,
    pub runtime_bundle_identity_commitment_policy: String,
    pub post_cleanup_observation_commitment_policy: String,
    pub evidence_policy: String,
    pub late_binding_policy: String,
    pub probe_contract: String,
    pub max_probe_timeout_ms: u64,
    pub max_request_bytes: u64,
    pub max_response_bytes: u64,
    pub cleanup_policy: String,
    pub observation_commit_policy: String,
    pub expiry_policy: String,
    pub lineage_policy: String,
    pub currentness_policy: String,
    pub revocation_policy: String,
    pub endpoint_disclosure_policy: String,
    pub caller_supplied_runtime_material_allowed: bool,
    pub activation_authority: String,
    pub effects: ExternalPoolAdapterProviderRuntimeReadinessEffects,
    pub observed_readiness: ExternalPoolAdapterProviderRuntimeReadinessObservedReadiness,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterProviderRuntimeReadinessPolicyEnvelope {
    pub schema: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub policy_digest: String,
    pub policy: ExternalPoolAdapterProviderRuntimeReadinessPolicy,
}

/// Canonical receipt material. It has no `Debug` implementation because it carries private seals.
#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterProviderRuntimeReadinessMaterial {
    pub policy_id: String,
    pub policy_revision: u64,
    pub policy_digest: String,
    pub provider_binding_id: String,
    pub provider_binding_digest: String,
    pub registry_release_id: String,
    pub registry_release_digest: String,
    pub registry_release_material_digest: String,
    pub installation_receipt_id: String,
    pub installation_receipt_digest: String,
    pub installation_content_digest: String,
    pub candidate_id: String,
    pub candidate_digest: String,
    pub delegation_id: String,
    pub delegation_digest: String,
    pub profile_id: String,
    pub profile_digest: String,
    pub target_id: String,
    pub target_digest: String,
    pub companion_id: String,
    pub companion_digest: String,
    pub provider_id: String,
    pub provider_policy_revision: i64,
    pub provider_digest: String,
    pub provider_status: String,
    pub vulnerability_reattestation_receipt_id: String,
    pub vulnerability_reattestation_receipt_digest: String,
    pub sandbox_reattestation_receipt_id: String,
    pub sandbox_reattestation_receipt_digest: String,
    pub credential_reattestation_receipt_id: String,
    pub credential_reattestation_receipt_digest: String,
    pub runtime_compatibility_verification_receipt_id: String,
    pub runtime_compatibility_verification_receipt_digest: String,
    pub launch_policy_digest: String,
    pub target_policy_digest: String,
    pub entrypoint_capsule_policy_digest: String,
    pub supervisor_session_policy_digest: String,
    pub source_capsule_sha256: String,
    pub source_capsule_size_bytes: u64,
    pub launch_image_sha256: String,
    pub launch_image_size_bytes: u64,
    pub sealed_bindings: ExternalPoolAdapterProviderRuntimeReadinessSealedBindings,
    pub probe_execution_id: String,
    pub request_bytes: u64,
    pub response_bytes: u64,
    pub probe_checked_at: String,
    pub cleanup_completed_at: String,
    pub checked_at: String,
    pub expires_at: String,
    pub sequence: u64,
    pub predecessor_readiness_receipt_id: Option<String>,
    pub predecessor_readiness_receipt_digest: Option<String>,
    pub recorded_by_actor_kind: String,
    pub recorded_by_actor_user_id: String,
    pub recorded_at: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub confirmation: String,
    pub evidence_scope: String,
    pub receipt_status: String,
    pub effects: ExternalPoolAdapterProviderRuntimeReadinessEffects,
    pub observed_readiness: ExternalPoolAdapterProviderRuntimeReadinessObservedReadiness,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterProviderRuntimeReadinessReceipt {
    pub schema: String,
    pub readiness_receipt_id: String,
    pub readiness_receipt_digest: String,
    pub readiness_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub readiness: ExternalPoolAdapterProviderRuntimeReadinessMaterial,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterProviderRuntimeReadinessRevocationMaterial {
    pub readiness_receipt_id: String,
    pub readiness_receipt_digest: String,
    pub provider_binding_id: String,
    pub provider_binding_digest: String,
    pub candidate_id: String,
    pub candidate_digest: String,
    pub profile_id: String,
    pub profile_digest: String,
    pub target_id: String,
    pub target_digest: String,
    pub companion_id: String,
    pub companion_digest: String,
    pub provider_id: String,
    pub revoked_by_actor_kind: String,
    pub revoked_by_actor_user_id: String,
    pub reason: String,
    pub revoked_at: String,
    pub recorded_at: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub confirmation: String,
    pub revocation_status: String,
    pub effects: ExternalPoolAdapterProviderRuntimeReadinessEffects,
    pub readiness: ExternalPoolAdapterProviderRuntimeReadinessObservedReadiness,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterProviderRuntimeReadinessRevocationReceipt {
    pub schema: String,
    pub revocation_receipt_id: String,
    pub revocation_receipt_digest: String,
    pub revocation_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub revocation: ExternalPoolAdapterProviderRuntimeReadinessRevocationMaterial,
}
