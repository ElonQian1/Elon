use anyhow::{bail, Result};
use serde::Serialize;

use crate::node_agent_compute_plugin_host::{
    identity::ComputePluginReleaseRef,
    plugin_manifest::{ComputePluginPermissionProfile, ComputePluginResourceLimits},
    runtime_loader_load_set::LoaderLockedWorkAdmittedPluginSlot,
    signed_artifact_verification::jcs_sha256_hex,
};

#[cfg(windows)]
use super::launch_security::SealedWindowsRunnerLaunchSecurity;

const START_MATERIAL_SCHEMA: &str = "elon.compute_plugin.windows_runner_process_preparation.v3";
const PREPARED_PROCESS_STATE: &str = "primary_thread_suspended";
const RESUME_AUTHORITY_STATUS: &str =
    "blocked_missing_authenticated_ipc_complete_enforcement_runtime_store_and_recovery";

pub(super) const RESUME_BLOCKERS: &[&str] = &[
    "authenticated_ipc_bootstrap",
    "cpu_enforcement",
    "disk_enforcement",
    "dynamic_module_load_enforcement",
    "caller_privilege_and_primary_token_lineage_authority",
    "cwd_volume_guid_createprocess_compatibility",
    "launch_path_parent_chain_currentness",
    "live_windows_resolution_currentness",
    "network_enforcement",
    "namespace_fence_explicit_release_and_recovery",
    "pre_resume_loader_currentness",
    "post_create_process_image_fileid_queryback",
    "post_create_live_process_machine_context_queryback",
    "process_object_owner_label_acl_queryback",
    "private_noninteractive_window_station_desktop_isolation",
    "runtime_transition_store",
    "runtime_transition_recovery",
    "sidecar_uptime_enforcement",
    "target_token_exact_object_accesscheck_receipts",
    "termination_outcome_recovery",
    "vram_enforcement",
];

#[derive(Serialize)]
struct WindowsRunnerProcessStartMaterial<'a> {
    schema: &'static str,
    process_state: &'static str,
    resume_authority_status: &'static str,
    resume_blockers: &'static [&'static str],
    installation_id_digest: &'a str,
    root_identity_digest: &'a str,
    working_directory_identity_digest: &'a str,
    plugin_id: &'a str,
    slot_ref: &'a str,
    release: &'a ComputePluginReleaseRef,
    work_admission_id: &'a str,
    work_admission_source_digest: &'a str,
    work_admission_receipt_digest: &'a str,
    work_admission_generation: i64,
    runtime_generation_unchanged: i64,
    authority_state_revision_unchanged: i64,
    authority_epoch_unchanged: i64,
    process_owner_epoch_unchanged: i64,
    clock_epoch_digest: &'a str,
    grant_ref: &'a str,
    grant_digest: &'a str,
    runner_relative_path: &'a str,
    runner_digest: &'a str,
    runner_size_bytes: i64,
    runner_file_identity_digest: &'a str,
    startup_import_resolution_profile_digest: &'a str,
    startup_import_namespace_authority_digest: &'a str,
    required_launch_context_digest: &'a str,
    launch_token_profile_digest: &'a str,
    launch_token_restricted: bool,
    launch_token_app_container: bool,
    process_security_descriptor_digest: &'a str,
    thread_security_descriptor_digest: &'a str,
    child_object_dacl: &'static str,
    token_user_sid_digest: &'a str,
    process_owner_sid_digest: &'a str,
    thread_owner_sid_digest: &'a str,
    process_mandatory_label_digest: &'a str,
    thread_mandatory_label_digest: &'a str,
    object_isolation_profile_digest: &'a str,
    token_granted_access_mask: u32,
    target_object_access_check_set_digest: &'a str,
    caller_token_privilege_lineage_digest: &'a str,
    private_desktop_isolation_digest: &'a str,
    entrypoint_arguments: &'a [String],
    entrypoint_arguments_digest: &'a str,
    granted_resources: &'a ComputePluginResourceLimits,
    granted_permissions: &'a ComputePluginPermissionProfile,
    job_kill_on_close: bool,
    job_active_process_limit: u32,
    job_memory_limit_bytes: usize,
    job_assignment_mode: &'static str,
    runtime_phase_effect: &'static str,
    runtime_generation_effect: &'static str,
    health_effect: &'static str,
    readiness_effect: &'static str,
    provider_effect: &'static str,
    route_effect: &'static str,
    offer_effect: &'static str,
    capacity_effect: &'static str,
    execution_effect: &'static str,
    attempt_effect: &'static str,
    lease_effect: &'static str,
    usage_effect: &'static str,
    settlement_effect: &'static str,
    money_effect: &'static str,
}

#[derive(Serialize)]
struct WindowsRunnerRequiredLaunchContext<'a> {
    schema: &'static str,
    launch_context_selector_digest: &'a str,
    target_id: &'a str,
    target_operating_system: &'a str,
    target_architecture: &'a str,
    process_machine_context_digest: &'a str,
    launch_token_profile_digest: &'a str,
    launch_token_restricted: bool,
    launch_token_app_container: bool,
    token_user_sid_digest: &'a str,
    process_security_descriptor_digest: &'a str,
    thread_security_descriptor_digest: &'a str,
    process_owner_sid_digest: &'a str,
    thread_owner_sid_digest: &'a str,
    process_mandatory_label_digest: &'a str,
    thread_mandatory_label_digest: &'a str,
    object_isolation_profile_digest: &'a str,
    token_granted_access_mask: u32,
    target_object_access_check_set_digest: &'a str,
    caller_token_privilege_lineage_digest: &'a str,
    private_desktop_isolation_digest: &'a str,
    child_object_dacl: &'static str,
    inherited_handles: bool,
    environment: &'static str,
    entrypoint_arguments_digest: &'a str,
    working_directory_identity_digest: &'a str,
    working_directory_source: &'static str,
    executable_path_source: &'static str,
    startup_import_resolution_profile_digest: &'a str,
    process_creation_flags: &'static [&'static str],
    job_assignment_mode: &'static str,
}

pub(super) const PROCESS_CREATION_FLAGS: &[&str] = &[
    "create_no_window",
    "create_suspended",
    "create_unicode_environment",
    "extended_startupinfo_present",
];
pub(super) const EMPTY_ENVIRONMENT_POLICY: &str = "explicit_empty_unicode_environment_block_v1";

pub(super) struct WindowsRunnerProcessPolicy {
    pub(super) arguments: Vec<String>,
    pub(super) active_process_limit: u32,
    pub(super) job_memory_limit_bytes: usize,
    pub(super) start_material_digest: String,
}

impl WindowsRunnerProcessPolicy {
    #[cfg(windows)]
    pub(super) fn from_sources(
        loader_locked: &LoaderLockedWorkAdmittedPluginSlot<'_>,
        launch_security: &SealedWindowsRunnerLaunchSecurity,
    ) -> Result<Self> {
        let pair = loader_locked.receipts();
        pair.validate()?;
        let source = pair.source().source();
        let receipt = pair.receipt().receipt();
        let image = loader_locked.image();
        let profile = source.launch_profile();
        profile.validate()?;
        let resources = profile.granted_resources();
        let active_process_limit = u32::try_from(resources.max_processes)?;
        let job_memory_limit_bytes = usize::try_from(resources.max_memory_bytes)?;
        if profile.target().operating_system != "windows"
            || active_process_limit == 0
            || job_memory_limit_bytes == 0
            || profile
                .entrypoint_arguments()
                .iter()
                .any(|argument| argument.contains('\0'))
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_RUNNER_POLICY_INVALID");
        }
        let generations = receipt.generations();
        let authority = receipt.authority();
        let launch_context = WindowsRunnerRequiredLaunchContext {
            schema: "elon.compute_plugin.windows_runner_required_launch_context.v3",
            launch_context_selector_digest: image.launch_context_selector_digest(),
            target_id: profile.target_id(),
            target_operating_system: &profile.target().operating_system,
            target_architecture: &profile.target().architecture,
            process_machine_context_digest: image.process_machine_context_digest(),
            launch_token_profile_digest: launch_security.token_profile_digest(),
            launch_token_restricted: launch_security.restricted_token_expected(),
            launch_token_app_container: launch_security.app_container_expected(),
            token_user_sid_digest: launch_security.token_user_sid_digest(),
            process_security_descriptor_digest: launch_security.process_descriptor_digest(),
            thread_security_descriptor_digest: launch_security.thread_descriptor_digest(),
            process_owner_sid_digest: launch_security.process_owner_sid_digest(),
            thread_owner_sid_digest: launch_security.thread_owner_sid_digest(),
            process_mandatory_label_digest: launch_security.process_mandatory_label_digest(),
            thread_mandatory_label_digest: launch_security.thread_mandatory_label_digest(),
            object_isolation_profile_digest: launch_security.object_isolation_profile_digest(),
            token_granted_access_mask: launch_security.token_granted_access_mask(),
            target_object_access_check_set_digest: launch_security
                .target_object_access_check_set_digest(),
            caller_token_privilege_lineage_digest: launch_security
                .caller_token_privilege_lineage_digest(),
            private_desktop_isolation_digest: launch_security.private_desktop_isolation_digest(),
            child_object_dacl: "present_non_null_empty_not_sibling_safe_alone",
            inherited_handles: false,
            environment: EMPTY_ENVIRONMENT_POLICY,
            entrypoint_arguments_digest: profile.entrypoint_arguments_digest(),
            working_directory_identity_digest: image.working_directory_identity_digest(),
            working_directory_source: "retained_handle_derived",
            executable_path_source: "retained_handle_derived",
            startup_import_resolution_profile_digest: image
                .startup_import_resolution_profile_digest(),
            process_creation_flags: PROCESS_CREATION_FLAGS,
            job_assignment_mode: "proc_thread_attribute_job_list",
        };
        let required_launch_context_digest = jcs_sha256_hex(&launch_context)?;
        if required_launch_context_digest
            != launch_security.expected_required_launch_context_digest()
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_REQUIRED_LAUNCH_CONTEXT_CHANGED");
        }
        let material = WindowsRunnerProcessStartMaterial {
            schema: START_MATERIAL_SCHEMA,
            process_state: PREPARED_PROCESS_STATE,
            resume_authority_status: RESUME_AUTHORITY_STATUS,
            resume_blockers: RESUME_BLOCKERS,
            installation_id_digest: source.installation_id_digest(),
            root_identity_digest: image.root_identity_digest(),
            working_directory_identity_digest: image.working_directory_identity_digest(),
            plugin_id: source.plugin_id(),
            slot_ref: source.slot_ref(),
            release: source.release(),
            work_admission_id: receipt.work_admission_id(),
            work_admission_source_digest: pair.source().source_digest(),
            work_admission_receipt_digest: pair.receipt().receipt_digest(),
            work_admission_generation: generations.work_admission_generation_after(),
            runtime_generation_unchanged: generations.runtime_generation(),
            authority_state_revision_unchanged: authority.authority_state_revision_after(),
            authority_epoch_unchanged: authority.authority_epoch_after(),
            process_owner_epoch_unchanged: authority.process_owner_epoch(),
            clock_epoch_digest: receipt.clock_epoch_digest(),
            grant_ref: profile.grant_ref(),
            grant_digest: profile.grant_digest(),
            runner_relative_path: profile.runner_relative_path(),
            runner_digest: profile.runner_file_digest(),
            runner_size_bytes: profile.runner_file_size_bytes(),
            runner_file_identity_digest: image.file_identity_digest(),
            startup_import_resolution_profile_digest: image
                .startup_import_resolution_profile_digest(),
            startup_import_namespace_authority_digest: image
                .startup_import_namespace_authority_digest(),
            required_launch_context_digest: &required_launch_context_digest,
            launch_token_profile_digest: launch_security.token_profile_digest(),
            launch_token_restricted: launch_security.restricted_token_expected(),
            launch_token_app_container: launch_security.app_container_expected(),
            process_security_descriptor_digest: launch_security.process_descriptor_digest(),
            thread_security_descriptor_digest: launch_security.thread_descriptor_digest(),
            child_object_dacl: "present_non_null_empty_not_sibling_safe_alone",
            token_user_sid_digest: launch_security.token_user_sid_digest(),
            process_owner_sid_digest: launch_security.process_owner_sid_digest(),
            thread_owner_sid_digest: launch_security.thread_owner_sid_digest(),
            process_mandatory_label_digest: launch_security.process_mandatory_label_digest(),
            thread_mandatory_label_digest: launch_security.thread_mandatory_label_digest(),
            object_isolation_profile_digest: launch_security.object_isolation_profile_digest(),
            token_granted_access_mask: launch_security.token_granted_access_mask(),
            target_object_access_check_set_digest: launch_security
                .target_object_access_check_set_digest(),
            caller_token_privilege_lineage_digest: launch_security
                .caller_token_privilege_lineage_digest(),
            private_desktop_isolation_digest: launch_security.private_desktop_isolation_digest(),
            entrypoint_arguments: profile.entrypoint_arguments(),
            entrypoint_arguments_digest: profile.entrypoint_arguments_digest(),
            granted_resources: resources,
            granted_permissions: profile.granted_permissions(),
            job_kill_on_close: true,
            job_active_process_limit: active_process_limit,
            job_memory_limit_bytes,
            job_assignment_mode: "proc_thread_attribute_job_list",
            runtime_phase_effect: "none",
            runtime_generation_effect: "none",
            health_effect: "none",
            readiness_effect: "none",
            provider_effect: "none",
            route_effect: "none",
            offer_effect: "none",
            capacity_effect: "none",
            execution_effect: "none",
            attempt_effect: "none",
            lease_effect: "none",
            usage_effect: "none",
            settlement_effect: "none",
            money_effect: "none",
        };
        Ok(Self {
            arguments: profile.entrypoint_arguments().to_vec(),
            active_process_limit,
            job_memory_limit_bytes,
            start_material_digest: jcs_sha256_hex(&material)?,
        })
    }
}

impl std::fmt::Debug for WindowsRunnerProcessPolicy {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WindowsRunnerProcessPolicy")
            .field("arguments", &"<redacted>")
            .field("active_process_limit", &self.active_process_limit)
            .field("job_memory_limit_bytes", &self.job_memory_limit_bytes)
            .field("start_material_digest", &"<redacted>")
            .field("resume_authority", &"blocked")
            .finish()
    }
}
