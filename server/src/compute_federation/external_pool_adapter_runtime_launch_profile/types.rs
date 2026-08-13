use serde::{Deserialize, Serialize};

pub(crate) const RUNTIME_LAUNCH_PROFILE_SCHEMA: &str =
    "compute_federation.external_pool_adapter_runtime_launch_profile.v1";
pub(crate) const RUNTIME_LAUNCH_PROFILE_REVOCATION_SCHEMA: &str =
    "compute_federation.external_pool_adapter_runtime_launch_profile_revocation.v1";
pub(crate) const RUNTIME_LAUNCH_PROFILE_CURRENTNESS_SCHEMA: &str =
    "compute_federation.external_pool_adapter_runtime_launch_profile_currentness.v1";
pub(crate) const RUNTIME_LAUNCH_PROFILE_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const RUNTIME_LAUNCH_PROFILE_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const RUNTIME_LAUNCH_PROFILE_CONFIRMATION: &str =
    "confirm_external_pool_adapter_runtime_launch_profile";
pub(crate) const RUNTIME_LAUNCH_PROFILE_REVOCATION_CONFIRMATION: &str =
    "confirm_external_pool_adapter_runtime_launch_profile_revocation";
pub(crate) const RUNTIME_LAUNCH_POLICY_ID: &str = "external_pool_adapter_runtime_launch_policy_v1";
pub(crate) const RUNTIME_LAUNCH_POLICY_REVISION: u64 = 1;
pub(crate) const RUNTIME_LAUNCH_PROFILE_STATUS: &str = "launch_profile_current_inert";
pub(crate) const RUNTIME_LAUNCH_PROFILE_EFFECT: &str = "runtime_launch_profile_recorded_inert";
pub(crate) const RUNTIME_LAUNCH_PROFILE_REVOCATION_EFFECT: &str = "runtime_launch_profile_revoked";
pub(crate) const RUNTIME_LAUNCH_PROFILE_NO_EFFECT: &str = "none";
pub(crate) const RUNTIME_LAUNCH_PROFILE_ACTOR_PROVIDER_OWNER: &str = "provider_owner";
pub(crate) const RUNTIME_LAUNCH_PROFILE_ACTOR_PLATFORM_ADMIN: &str = "platform_admin";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRuntimeLaunchPolicy {
    pub policy_id: String,
    pub policy_revision: u64,
    pub runtime_kind: String,
    pub host_os: String,
    pub host_arch: String,
    pub host_environment: String,
    pub executable_kind: String,
    pub binary_format: String,
    pub executable_verification_status: String,
    pub materialization_kind: String,
    pub shell_allowed: bool,
    pub argv_policy: String,
    pub environment_policy: String,
    pub working_directory_policy: String,
    pub ipc_transport: String,
    pub sidecar_protocol_id: String,
    pub sidecar_protocol_revision: u64,
    pub ipc_framing: String,
    pub max_frame_bytes: u64,
    pub ipc_session_auth: String,
    pub config_resolver_kind: String,
    pub credential_resolver_kind: String,
    pub resolver_backend_policy_id: String,
    pub resolver_backend_policy_revision: u64,
    pub resolver_backend_policy_digest: String,
    pub config_delivery_kind: String,
    pub credential_delivery_kind: String,
    pub secret_custody_policy: String,
    pub probe_contract: String,
    pub process_isolation_policy_id: String,
    pub process_isolation_policy_revision: u64,
    pub process_isolation_policy_digest: String,
    pub resource_policy_id: String,
    pub resource_policy_revision: u64,
    pub resource_policy_digest: String,
    pub network_egress_policy_id: String,
    pub network_egress_policy_revision: u64,
    pub network_egress_policy_digest: String,
    pub startup_timeout_ms: u64,
    pub handshake_timeout_ms: u64,
    pub probe_timeout_ms: u64,
    pub shutdown_timeout_ms: u64,
    pub max_sidecar_processes: u64,
    pub max_stderr_bytes: u64,
    pub max_runtime_temp_bytes: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRuntimeLaunchProfileMaterial {
    pub candidate_id: String,
    pub candidate_digest: String,
    pub delegation_id: String,
    pub delegation_digest: String,
    pub provider_binding_id: String,
    pub provider_binding_digest: String,
    pub registry_release_id: String,
    pub registry_release_digest: String,
    pub installation_receipt_id: String,
    pub installation_receipt_digest: String,
    pub installation_content_digest: String,
    pub route_adapter_projection_id: String,
    pub provider_id: String,
    pub provider_owner_account_id: String,
    pub provider_policy_revision: i64,
    pub provider_digest: String,
    pub provider_status: String,
    pub logical_adapter_id: String,
    pub release_version: String,
    pub adapter_config_revision: i64,
    pub adapter_config_digest: String,
    pub implementation_digest: String,
    pub capability_set_digest: String,
    pub credential_verifier_digest: String,
    pub credential_ref_scheme: String,
    pub credential_locator_commitment: String,
    pub service_actor_id: String,
    pub entrypoint_relative_path: String,
    pub entrypoint_path_digest: String,
    pub entrypoint_sha256: String,
    pub entrypoint_size_bytes: u64,
    pub entry_inventory_digest: String,
    pub installed_file_count: u64,
    pub installed_total_bytes: u64,
    pub launch_policy_digest: String,
    pub launch_policy: ExternalPoolAdapterRuntimeLaunchPolicy,
    pub sequence: u64,
    pub predecessor_profile_id: Option<String>,
    pub predecessor_profile_digest: Option<String>,
    pub recorded_by_actor_kind: String,
    pub recorded_by_actor_user_id: String,
    pub recorded_at: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub confirmation: String,
    pub profile_status: String,
    pub profile_effect: String,
    pub adapter_effect: String,
    pub runtime_effect: String,
    pub provider_effect: String,
    pub credential_effect: String,
    pub route_effect: String,
    pub execution_effect: String,
    pub usage_effect: String,
    pub market_effect: String,
    pub settlement_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRuntimeLaunchProfileReceipt {
    pub schema: String,
    pub profile_id: String,
    pub profile_digest: String,
    pub profile_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub profile: ExternalPoolAdapterRuntimeLaunchProfileMaterial,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRuntimeLaunchProfileRevocationMaterial {
    pub profile_id: String,
    pub profile_digest: String,
    pub provider_binding_id: String,
    pub provider_binding_digest: String,
    pub candidate_id: String,
    pub candidate_digest: String,
    pub revoked_by_actor_kind: String,
    pub revoked_by_actor_user_id: String,
    pub reason: String,
    pub revoked_at: String,
    pub recorded_at: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub confirmation: String,
    pub revocation_effect: String,
    pub adapter_effect: String,
    pub runtime_effect: String,
    pub provider_effect: String,
    pub credential_effect: String,
    pub route_effect: String,
    pub execution_effect: String,
    pub usage_effect: String,
    pub market_effect: String,
    pub settlement_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRuntimeLaunchProfileRevocationReceipt {
    pub schema: String,
    pub revocation_id: String,
    pub revocation_digest: String,
    pub revocation_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub revocation: ExternalPoolAdapterRuntimeLaunchProfileRevocationMaterial,
}
