use serde::{Deserialize, Serialize};

pub(crate) const SUPERVISOR_SESSION_COMPANION_SCHEMA: &str =
    "compute_federation.external_pool_adapter_supervisor_session_policy_companion.v1";
pub(crate) const SUPERVISOR_SESSION_COMPANION_REVOCATION_SCHEMA: &str =
    "compute_federation.external_pool_adapter_supervisor_session_policy_companion_revocation.v1";
pub(crate) const SUPERVISOR_SESSION_COMPANION_CURRENTNESS_SCHEMA: &str =
    "compute_federation.external_pool_adapter_supervisor_session_policy_companion_currentness.v1";
pub(crate) const SUPERVISOR_SESSION_COMPANION_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const SUPERVISOR_SESSION_COMPANION_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const SUPERVISOR_SESSION_COMPANION_CONFIRMATION: &str =
    "confirm_external_pool_adapter_supervisor_session_policy_companion";
pub(crate) const SUPERVISOR_SESSION_COMPANION_REVOCATION_CONFIRMATION: &str =
    "confirm_external_pool_adapter_supervisor_session_policy_companion_revocation";
pub(crate) const SUPERVISOR_SESSION_POLICY_ID: &str =
    "external_pool_adapter_supervisor_session_policy_v2";
pub(crate) const SUPERVISOR_SESSION_POLICY_REVISION: u64 = 2;
pub(crate) const SUPERVISOR_SESSION_COMPANION_STATUS: &str =
    "supervisor_session_policy_companion_current_inert";
pub(crate) const SUPERVISOR_SESSION_COMPANION_EFFECT: &str =
    "supervisor_session_policy_companion_recorded_inert";
pub(crate) const SUPERVISOR_SESSION_COMPANION_REVOCATION_EFFECT: &str =
    "supervisor_session_policy_companion_revoked";
pub(crate) const SUPERVISOR_SESSION_COMPANION_NO_EFFECT: &str = "none";
pub(crate) const SUPERVISOR_SESSION_COMPANION_ACTOR_PROVIDER_OWNER: &str = "provider_owner";
pub(crate) const SUPERVISOR_SESSION_COMPANION_ACTOR_PLATFORM_ADMIN: &str = "platform_admin";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterSupervisorSessionWirePolicy {
    pub transport: String,
    pub protocol_id: String,
    pub protocol_revision: u64,
    pub legacy_launch_profile_framing: String,
    pub framing: String,
    pub frame_magic_ascii: String,
    pub frame_header_bytes: u64,
    pub frame_mac_bytes: u64,
    pub header_field_order: Vec<String>,
    pub frame_kind_control: u64,
    pub frame_kind_config: u64,
    pub frame_kind_credential: u64,
    pub control_encoding: String,
    pub binary_encoding: String,
    pub max_control_payload_bytes: u64,
    pub max_config_payload_bytes: u64,
    pub max_credential_payload_bytes: u64,
    pub max_frames_per_direction: u64,
    pub unknown_kind_policy: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterSupervisorSessionCryptoPolicy {
    pub seed_policy: String,
    pub seed_bytes: u64,
    pub nonce_policy: String,
    pub nonce_bytes: u64,
    pub kdf: String,
    pub kdf_salt: String,
    pub kdf_context: String,
    pub directional_key_bytes: u64,
    pub mac: String,
    pub mac_coverage: String,
    pub sequence_policy: String,
    pub transcript_policy: String,
    pub key_custody: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterSupervisorSessionStatePolicy {
    pub state_machine: String,
    pub host_states: Vec<String>,
    pub child_states: Vec<String>,
    pub terminal_states: Vec<String>,
    pub invalid_transition_policy: String,
    pub authentication_failure_policy: String,
    pub startup_timeout_ms: u64,
    pub handshake_timeout_ms: u64,
    pub sensitive_delivery_timeout_ms: u64,
    pub probe_timeout_ms: u64,
    pub shutdown_timeout_ms: u64,
    pub reap_timeout_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterSupervisorLinuxLaunchPolicy {
    pub primitive: String,
    pub require_clone_pidfd: bool,
    pub require_clone_into_cgroup: bool,
    pub fallback_allowed: bool,
    pub pid_namespace_enabled: bool,
    pub user_namespace_enabled: bool,
    pub mount_namespace_enabled: bool,
    pub network_namespace_enabled: bool,
    pub ipc_namespace_enabled: bool,
    pub uts_namespace_enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterSupervisorLinuxIdentityPolicy {
    pub mapping: String,
    pub deny_setgroups: bool,
    pub clear_all_capability_sets: bool,
    pub no_new_privileges: bool,
    pub dumpable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exec_transition_ptrace_guard: Option<String>,
    pub umask: u64,
    pub create_session: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterSupervisorLinuxFilesystemPolicy {
    pub mount_propagation: String,
    pub root_filesystem: String,
    pub pivot_root_required: bool,
    pub proc_mounted: bool,
    pub sys_mounted: bool,
    pub dev_mounted: bool,
    pub working_directory: String,
    pub tmp_mount_flags: Vec<String>,
    pub tmp_mode: u64,
    pub tmp_limit_bytes: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterSupervisorLinuxCgroupPolicy {
    pub hierarchy: String,
    pub dedicated_leaf_required: bool,
    pub pids_max: u64,
    pub memory_max_bytes: u64,
    pub memory_swap_max_bytes: u64,
    pub memory_oom_group: bool,
    pub cpu_quota_us: u64,
    pub cpu_period_us: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterSupervisorLinuxRlimitPolicy {
    pub core_bytes: u64,
    pub nofile: u64,
    pub nproc: u64,
    pub address_space_bytes: u64,
    pub file_size_bytes: u64,
    pub stack_bytes: u64,
    pub memlock_bytes: u64,
    pub cpu_seconds: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterSupervisorLinuxDescriptorPolicy {
    pub stdin_fd: u64,
    pub stdin_policy: String,
    pub stdout_fd: u64,
    pub stdout_policy: String,
    pub stderr_fd: u64,
    pub stderr_policy: String,
    pub child_ipc_fd: u64,
    pub child_ipc_fd_policy: String,
    pub seed_fd: u64,
    pub seed_fd_bytes: u64,
    pub seed_fd_cloexec: bool,
    pub seed_fd_read_phase: String,
    pub seed_fd_close_after_read: bool,
    pub capsule_fd: u64,
    pub capsule_fd_cloexec: bool,
    pub close_range_from_fd: u64,
    pub close_range_unshare: bool,
    pub post_exec_open_fds: Vec<u64>,
    pub post_seed_open_fds: Vec<u64>,
    pub child_ipc_fd_allowed: bool,
    pub child_network_or_target_fd_allowed: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterSupervisorLinuxSeccompPolicy {
    pub architecture: String,
    pub unknown_syscall_action: String,
    pub audit_arch_policy: String,
    pub bootstrap_allowed_syscalls: Vec<String>,
    pub runtime_allowed_syscalls: Vec<String>,
    pub argument_rules: Vec<String>,
    pub exec_rule: String,
    pub deny_new_executable_mappings_after_exec: bool,
    pub deny_process_creation: bool,
    pub deny_network_syscalls: bool,
    pub deny_mount_namespace_capability_keyring_ptrace_bpf_perf_io_uring: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterSupervisorLinuxShutdownPolicy {
    pub process_handle: String,
    pub initial_signal: String,
    pub grace_ms: u64,
    pub terminal_signal: String,
    pub reap: String,
    pub pid_fallback_allowed: bool,
    pub descendant_policy: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterSupervisorLinuxConfinementPolicy {
    pub host_os: String,
    pub host_arch: String,
    pub launch: ExternalPoolAdapterSupervisorLinuxLaunchPolicy,
    pub identity: ExternalPoolAdapterSupervisorLinuxIdentityPolicy,
    pub filesystem: ExternalPoolAdapterSupervisorLinuxFilesystemPolicy,
    pub cgroup: ExternalPoolAdapterSupervisorLinuxCgroupPolicy,
    pub rlimits: ExternalPoolAdapterSupervisorLinuxRlimitPolicy,
    pub descriptors: ExternalPoolAdapterSupervisorLinuxDescriptorPolicy,
    pub seccomp: ExternalPoolAdapterSupervisorLinuxSeccompPolicy,
    pub network_policy: String,
    pub stderr_limit_bytes: u64,
    pub stderr_overflow_policy: String,
    pub shutdown: ExternalPoolAdapterSupervisorLinuxShutdownPolicy,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterSupervisorSessionPolicy {
    pub policy_id: String,
    pub policy_revision: u64,
    pub supervisor_owner: String,
    pub compatibility: String,
    pub wire: ExternalPoolAdapterSupervisorSessionWirePolicy,
    pub crypto: ExternalPoolAdapterSupervisorSessionCryptoPolicy,
    pub state: ExternalPoolAdapterSupervisorSessionStatePolicy,
    pub linux_confinement: ExternalPoolAdapterSupervisorLinuxConfinementPolicy,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterSupervisorSessionPolicyCompanionMaterial {
    pub profile_id: String,
    pub profile_digest: String,
    pub candidate_id: String,
    pub candidate_digest: String,
    pub provider_binding_id: String,
    pub provider_binding_digest: String,
    pub provider_id: String,
    pub provider_owner_account_id: String,
    pub provider_policy_revision: i64,
    pub provider_digest: String,
    pub provider_status: String,
    pub launch_policy_digest: String,
    pub process_isolation_policy_id: String,
    pub process_isolation_policy_revision: u64,
    pub process_isolation_policy_digest: String,
    pub resource_policy_id: String,
    pub resource_policy_revision: u64,
    pub resource_policy_digest: String,
    pub network_egress_policy_id: String,
    pub network_egress_policy_revision: u64,
    pub network_egress_policy_digest: String,
    pub delegation_id: String,
    pub delegation_digest: String,
    pub registry_release_id: String,
    pub registry_release_digest: String,
    pub installation_receipt_id: String,
    pub installation_receipt_digest: String,
    pub installation_content_digest: String,
    pub route_adapter_projection_id: String,
    pub logical_adapter_id: String,
    pub release_version: String,
    pub adapter_config_revision: i64,
    pub adapter_config_digest: String,
    pub implementation_digest: String,
    pub capability_set_digest: String,
    pub credential_verifier_digest: String,
    pub service_actor_id: String,
    pub entrypoint_capsule_policy_id: String,
    pub entrypoint_capsule_policy_revision: u64,
    pub entrypoint_capsule_policy_digest: String,
    pub target_id: String,
    pub target_digest: String,
    pub target_policy_digest: String,
    pub supervisor_session_policy_digest: String,
    pub supervisor_session_policy: ExternalPoolAdapterSupervisorSessionPolicy,
    pub sequence: u64,
    pub predecessor_companion_id: Option<String>,
    pub predecessor_companion_digest: Option<String>,
    pub recorded_by_actor_kind: String,
    pub recorded_by_actor_user_id: String,
    pub recorded_at: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub confirmation: String,
    pub companion_status: String,
    pub companion_effect: String,
    pub adapter_effect: String,
    pub runtime_effect: String,
    pub provider_effect: String,
    pub credential_effect: String,
    pub route_effect: String,
    pub execution_effect: String,
    pub usage_effect: String,
    pub market_effect: String,
    pub settlement_effect: String,
    pub process_spawn_ready: bool,
    pub ipc_session_ready: bool,
    pub secret_delivery_ready: bool,
    pub broker_connect_ready: bool,
    pub upstream_probe_observed: bool,
    pub runtime_launch_ready: bool,
    pub activation_ready: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt {
    pub schema: String,
    pub companion_id: String,
    pub companion_digest: String,
    pub companion_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub companion: ExternalPoolAdapterSupervisorSessionPolicyCompanionMaterial,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterSupervisorSessionPolicyCompanionRevocationMaterial {
    pub companion_id: String,
    pub companion_digest: String,
    pub target_id: String,
    pub target_digest: String,
    pub profile_id: String,
    pub profile_digest: String,
    pub provider_binding_id: String,
    pub provider_binding_digest: String,
    pub provider_id: String,
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
    pub process_spawn_ready: bool,
    pub ipc_session_ready: bool,
    pub secret_delivery_ready: bool,
    pub broker_connect_ready: bool,
    pub upstream_probe_observed: bool,
    pub runtime_launch_ready: bool,
    pub activation_ready: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterSupervisorSessionPolicyCompanionRevocationReceipt {
    pub schema: String,
    pub revocation_id: String,
    pub revocation_digest: String,
    pub revocation_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub revocation: ExternalPoolAdapterSupervisorSessionPolicyCompanionRevocationMaterial,
}
