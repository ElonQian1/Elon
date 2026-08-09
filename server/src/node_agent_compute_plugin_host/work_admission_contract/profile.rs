use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::node_agent_compute_plugin_host::{
    install_plan::ComputePluginGrantBinding,
    install_plan_admission_validation::is_identifier,
    manifest_validation::is_sha256,
    plugin_manifest::{
        resource_limits_are_non_negative, ComputePluginHealthCheck, ComputePluginPermissionProfile,
        ComputePluginResourceLimits, ComputePluginTarget, SignedComputePluginManifest,
        COMPUTE_PLUGIN_ENTRYPOINT_SIDECAR,
    },
    signed_artifact_verification::jcs_sha256_hex,
};

mod manifest_shape;

use manifest_shape::validate_work_admission_signed_manifest;

const PROFILE_SCHEMA: &str = "elon.compute_plugin.work_admission_launch_profile.v1";
const ARGUMENTS_SCHEMA: &str = "elon.compute_plugin.entrypoint_arguments.v1";
const GRANT_SCHEMA: &str = "elon.compute_plugin.grant_binding.v1";
const TARGET_JSON_MAX_BYTES: usize = 131_072;
const TASK_KINDS_JSON_MAX_BYTES: usize = 65_536;
const ENTRYPOINT_ARGUMENTS_JSON_MAX_BYTES: usize = 65_536;
const HEALTH_CHECK_JSON_MAX_BYTES: usize = 65_536;
const GRANTED_PERMISSIONS_JSON_MAX_BYTES: usize = 131_072;
const I_JSON_MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;

macro_rules! string_getters {
    ($($name:ident),* $(,)?) => {$(
        pub(in crate::node_agent_compute_plugin_host) fn $name(&self) -> &str {
            &self.$name
        }
    )*};
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginWorkAdmissionLaunchProfile {
    schema: String,
    plugin_id: String,
    plugin_version: String,
    publisher_id: String,
    manifest_digest: String,
    signed_manifest_envelope_digest: String,
    target_id: String,
    target: ComputePluginTarget,
    task_kinds: Vec<String>,
    host_api_protocol_id: String,
    host_api_revision: u32,
    entrypoint_kind: String,
    entrypoint_relative_path: String,
    entrypoint_arguments: Vec<String>,
    entrypoint_arguments_digest: String,
    health_check: ComputePluginHealthCheck,
    runner_relative_path: String,
    runner_file_digest: String,
    runner_file_size_bytes: i64,
    runner_file_executable: bool,
    grant_ref: String,
    grant_digest: String,
    granted_permissions: ComputePluginPermissionProfile,
    granted_resources: ComputePluginResourceLimits,
}

#[derive(Serialize)]
struct EntrypointArgumentsBinding<'a> {
    schema: &'static str,
    arguments: &'a [String],
}

#[derive(Serialize)]
struct GrantDigestBinding<'a> {
    schema: &'static str,
    grant_ref: &'a str,
    granted_permissions: &'a ComputePluginPermissionProfile,
    granted_resources: &'a ComputePluginResourceLimits,
}

impl ComputePluginWorkAdmissionLaunchProfile {
    pub(super) fn from_authority_source(
        signed: &SignedComputePluginManifest,
        signed_manifest_envelope_digest: &str,
        grant: &ComputePluginGrantBinding,
        selected_host_api_revision: u32,
    ) -> Result<(
        Self,
        crate::node_agent_compute_plugin_host::identity::ComputePluginReleaseRef,
    )> {
        let release =
            validate_work_admission_signed_manifest(signed, signed_manifest_envelope_digest)?;
        validate_grant(grant, signed)?;
        let value = &signed.manifest;
        if selected_host_api_revision < value.host_api.minimum_revision
            || selected_host_api_revision > value.host_api.maximum_revision
            || value.entrypoint.entrypoint_kind != COMPUTE_PLUGIN_ENTRYPOINT_SIDECAR
        {
            bail!("COMPUTE_PLUGIN_WORK_ADMISSION_MANIFEST_PROJECTION_INVALID");
        }
        let runner = value
            .package
            .files
            .iter()
            .find(|file| file.relative_path == value.entrypoint.relative_path)
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WORK_ADMISSION_RUNNER_FILE_MISSING"))?;
        if !runner.executable || runner.size_bytes <= 0 || !is_sha256(&runner.digest) {
            bail!("COMPUTE_PLUGIN_WORK_ADMISSION_RUNNER_FILE_INVALID");
        }
        let arguments_digest = jcs_sha256_hex(&EntrypointArgumentsBinding {
            schema: ARGUMENTS_SCHEMA,
            arguments: &value.entrypoint.arguments,
        })?;
        let profile = Self {
            schema: PROFILE_SCHEMA.to_string(),
            plugin_id: value.plugin_id.clone(),
            plugin_version: value.plugin_version.clone(),
            publisher_id: value.publisher_id.clone(),
            manifest_digest: signed.manifest_digest.clone(),
            signed_manifest_envelope_digest: signed_manifest_envelope_digest.to_string(),
            target_id: value.target.target_id.clone(),
            target: value.target.clone(),
            task_kinds: value.task_kinds.clone(),
            host_api_protocol_id: value.host_api.protocol_id.clone(),
            host_api_revision: selected_host_api_revision,
            entrypoint_kind: value.entrypoint.entrypoint_kind.clone(),
            entrypoint_relative_path: value.entrypoint.relative_path.clone(),
            entrypoint_arguments: value.entrypoint.arguments.clone(),
            entrypoint_arguments_digest: arguments_digest,
            health_check: value.entrypoint.health_check.clone(),
            runner_relative_path: runner.relative_path.clone(),
            runner_file_digest: runner.digest.clone(),
            runner_file_size_bytes: runner.size_bytes,
            runner_file_executable: runner.executable,
            grant_ref: grant.grant_ref.clone(),
            grant_digest: grant.grant_digest.clone(),
            granted_permissions: grant.granted_permissions.clone(),
            granted_resources: grant.granted_resources.clone(),
        };
        profile.validate()?;
        Ok((profile, release))
    }

    pub(in crate::node_agent_compute_plugin_host) fn validate(&self) -> Result<()> {
        let arguments_digest = jcs_sha256_hex(&EntrypointArgumentsBinding {
            schema: ARGUMENTS_SCHEMA,
            arguments: &self.entrypoint_arguments,
        })?;
        let projections_fit_store =
            serialized_json_len_at_most(&self.target, TARGET_JSON_MAX_BYTES)?
                && serialized_json_len_at_most(&self.task_kinds, TASK_KINDS_JSON_MAX_BYTES)?
                && serialized_json_len_at_most(
                    &self.entrypoint_arguments,
                    ENTRYPOINT_ARGUMENTS_JSON_MAX_BYTES,
                )?
                && serialized_json_len_at_most(&self.health_check, HEALTH_CHECK_JSON_MAX_BYTES)?
                && serialized_json_len_at_most(
                    &self.granted_permissions,
                    GRANTED_PERMISSIONS_JSON_MAX_BYTES,
                )?;
        if self.schema != PROFILE_SCHEMA
            || self.plugin_id.trim().is_empty()
            || self.plugin_version.trim().is_empty()
            || self.publisher_id.trim().is_empty()
            || self.target_id.trim().is_empty()
            || self.target.target_id != self.target_id
            || self.target.operating_system.trim().is_empty()
            || self.target.architecture.trim().is_empty()
            || self.target.accelerator_kind.is_some() != self.target.accelerator_abi.is_some()
            || self.task_kinds.is_empty()
            || !strictly_sorted(self.task_kinds.iter().map(String::as_str))
            || self.host_api_protocol_id.trim().is_empty()
            || self.host_api_revision == 0
            || self.entrypoint_kind != COMPUTE_PLUGIN_ENTRYPOINT_SIDECAR
            || self.entrypoint_relative_path != self.runner_relative_path
            || self.health_check.protocol.trim().is_empty()
            || self.health_check.timeout_ms <= 0
            || self.health_check.interval_ms <= 0
            || self.health_check.healthy_after_successes <= 0
            || self.health_check.unhealthy_after_failures <= 0
            || self.runner_file_size_bytes <= 0
            || self.runner_file_size_bytes > I_JSON_MAX_SAFE_INTEGER
            || !self.runner_file_executable
            || !is_identifier(&self.grant_ref)
            || !is_sha256(&self.manifest_digest)
            || !is_sha256(&self.signed_manifest_envelope_digest)
            || !is_sha256(&self.entrypoint_arguments_digest)
            || !is_sha256(&self.runner_file_digest)
            || !is_sha256(&self.grant_digest)
            || arguments_digest != self.entrypoint_arguments_digest
            || !projections_fit_store
            || !resources_are_executable(&self.granted_resources)
            || !permissions_are_canonical(&self.granted_permissions)
        {
            bail!("COMPUTE_PLUGIN_WORK_ADMISSION_LAUNCH_PROFILE_INVALID");
        }
        Ok(())
    }

    string_getters! {
        plugin_id, plugin_version, publisher_id, manifest_digest,
        signed_manifest_envelope_digest, target_id, host_api_protocol_id,
        entrypoint_kind, entrypoint_relative_path, entrypoint_arguments_digest,
        runner_relative_path, runner_file_digest, grant_ref, grant_digest,
    }

    pub(in crate::node_agent_compute_plugin_host) fn task_kinds(&self) -> &[String] {
        &self.task_kinds
    }

    pub(in crate::node_agent_compute_plugin_host) fn target(&self) -> &ComputePluginTarget {
        &self.target
    }

    pub(in crate::node_agent_compute_plugin_host) fn health_check(
        &self,
    ) -> &ComputePluginHealthCheck {
        &self.health_check
    }

    pub(in crate::node_agent_compute_plugin_host) fn entrypoint_arguments(&self) -> &[String] {
        &self.entrypoint_arguments
    }

    pub(in crate::node_agent_compute_plugin_host) fn host_api_revision(&self) -> u32 {
        self.host_api_revision
    }

    pub(in crate::node_agent_compute_plugin_host) fn runner_file_size_bytes(&self) -> i64 {
        self.runner_file_size_bytes
    }

    pub(in crate::node_agent_compute_plugin_host) fn runner_file_executable(&self) -> bool {
        self.runner_file_executable
    }

    pub(in crate::node_agent_compute_plugin_host) fn granted_permissions(
        &self,
    ) -> &ComputePluginPermissionProfile {
        &self.granted_permissions
    }

    pub(in crate::node_agent_compute_plugin_host) fn granted_resources(
        &self,
    ) -> &ComputePluginResourceLimits {
        &self.granted_resources
    }
}

fn serialized_json_len_at_most(value: &impl Serialize, maximum: usize) -> Result<bool> {
    Ok(serde_json::to_string(value)?.len() <= maximum)
}

fn validate_grant(
    grant: &ComputePluginGrantBinding,
    signed: &SignedComputePluginManifest,
) -> Result<()> {
    let requested = &signed.manifest;
    let digest = jcs_sha256_hex(&GrantDigestBinding {
        schema: GRANT_SCHEMA,
        grant_ref: &grant.grant_ref,
        granted_permissions: &grant.granted_permissions,
        granted_resources: &grant.granted_resources,
    })?;
    if !is_identifier(&grant.grant_ref)
        || digest != grant.grant_digest
        || !resources_are_executable(&grant.granted_resources)
        || !resources_are_subset(&grant.granted_resources, &requested.requested_resources)
        || !permissions_are_subset(&grant.granted_permissions, &requested.requested_permissions)
    {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_GRANT_INVALID");
    }
    Ok(())
}

fn resources_are_executable(value: &ComputePluginResourceLimits) -> bool {
    resource_limits_are_non_negative(value)
        && value.max_cpu_millicores > 0
        && value.max_cpu_millicores <= I_JSON_MAX_SAFE_INTEGER
        && value.max_memory_bytes > 0
        && value.max_memory_bytes <= I_JSON_MAX_SAFE_INTEGER
        && value.max_vram_bytes <= I_JSON_MAX_SAFE_INTEGER
        && value.max_disk_bytes > 0
        && value.max_disk_bytes <= I_JSON_MAX_SAFE_INTEGER
        && value.max_processes > 0
        && value.max_processes <= I_JSON_MAX_SAFE_INTEGER
        && value.max_sidecar_uptime_seconds > 0
        && value.max_sidecar_uptime_seconds <= I_JSON_MAX_SAFE_INTEGER
}

fn resources_are_subset(
    granted: &ComputePluginResourceLimits,
    requested: &ComputePluginResourceLimits,
) -> bool {
    granted.max_cpu_millicores <= requested.max_cpu_millicores
        && granted.max_memory_bytes <= requested.max_memory_bytes
        && granted.max_vram_bytes <= requested.max_vram_bytes
        && granted.max_disk_bytes <= requested.max_disk_bytes
        && granted.max_processes <= requested.max_processes
        && granted.max_sidecar_uptime_seconds <= requested.max_sidecar_uptime_seconds
}

fn permissions_are_subset(
    granted: &ComputePluginPermissionProfile,
    requested: &ComputePluginPermissionProfile,
) -> bool {
    permissions_are_canonical(granted)
        && (!granted.allow_network_egress || requested.allow_network_egress)
        && granted
            .allowed_egress_domains
            .iter()
            .all(|value| requested.allowed_egress_domains.contains(value))
        && (!granted.allow_child_processes || requested.allow_child_processes)
        && granted
            .filesystem_scopes
            .iter()
            .all(|value| requested.filesystem_scopes.contains(value))
        && granted
            .device_scopes
            .iter()
            .all(|value| requested.device_scopes.contains(value))
}

fn permissions_are_canonical(value: &ComputePluginPermissionProfile) -> bool {
    (!value.allow_network_egress || !value.allowed_egress_domains.is_empty())
        && (value.allow_network_egress || value.allowed_egress_domains.is_empty())
        && strictly_sorted(value.allowed_egress_domains.iter().map(String::as_str))
        && strictly_sorted(
            value
                .filesystem_scopes
                .iter()
                .map(|scope| scope.wire_name()),
        )
        && strictly_sorted(value.device_scopes.iter().map(|scope| scope.wire_name()))
}

fn strictly_sorted<'a>(values: impl Iterator<Item = &'a str>) -> bool {
    let mut previous = None;
    values.into_iter().all(|value| {
        let valid = previous.is_none_or(|prior| prior < value);
        previous = Some(value);
        valid
    })
}
