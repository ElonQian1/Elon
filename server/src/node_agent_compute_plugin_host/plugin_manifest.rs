use serde::{Deserialize, Serialize};

pub(crate) const COMPUTE_PLUGIN_MANIFEST_SCHEMA: &str = "elon.compute_plugin.manifest.v1";
pub(crate) const SIGNED_COMPUTE_PLUGIN_MANIFEST_SCHEMA: &str =
    "elon.compute_plugin.signed_manifest.v1";
pub(crate) const COMPUTE_PLUGIN_ENTRYPOINT_SIDECAR: &str = "sidecar";
pub(crate) const COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const COMPUTE_PLUGIN_MANIFEST_SIGNATURE_DOMAIN: &str = "ELON-COMPUTE-PLUGIN-MANIFEST-V1";
pub(crate) const COMPUTE_PLUGIN_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const COMPUTE_PLUGIN_SIGNATURE_ALGORITHM: &str = "ed25519";
pub(crate) const COMPUTE_PLUGIN_MAX_PACKAGE_FILES: usize = 4_096;
pub(crate) const COMPUTE_PLUGIN_MAX_ENTRYPOINT_ARGUMENTS: usize = 64;

/// The signature is outside this canonical payload, so it never signs itself.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginManifest {
    pub schema: String,
    pub plugin_id: String,
    pub plugin_version: String,
    pub publisher_id: String,
    pub package: ComputePluginPackage,
    pub host_api: ComputePluginHostApiRange,
    pub task_kinds: Vec<String>,
    pub target: ComputePluginTarget,
    pub entrypoint: ComputePluginEntrypoint,
    pub system_dependencies: Vec<ComputePluginSystemDependency>,
    pub download_dependencies: Vec<ComputePluginDownloadDependency>,
    pub requested_resources: ComputePluginResourceLimits,
    pub requested_permissions: ComputePluginPermissionProfile,
    pub state_compatibility: Option<ComputePluginStateCompatibility>,
}

/// Digest covers canonical `manifest` bytes only. The signature covers domain + NUL + digest.
/// Ordered lists must be normalized and de-duplicated before signing; JCS does not sort arrays.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct SignedComputePluginManifest {
    pub schema: String,
    pub manifest: ComputePluginManifest,
    pub canonicalization: String,
    pub manifest_digest_algorithm: String,
    pub manifest_digest: String,
    pub signature: ComputePluginSignature,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginSignature {
    pub algorithm: String,
    pub signing_key_id: String,
    pub signature_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginPackage {
    pub media_type: String,
    pub archive_format: String,
    pub digest_algorithm: String,
    pub package_digest: String,
    pub package_size_bytes: i64,
    pub unpacked_size_bytes: i64,
    pub files: Vec<ComputePluginPackageFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginPackageFile {
    /// Validator accepts normalized relative regular-file paths only: no links, devices or `..`.
    pub relative_path: String,
    pub digest: String,
    pub size_bytes: i64,
    pub executable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginHostApiRange {
    pub protocol_id: String,
    pub minimum_revision: u32,
    pub maximum_revision: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginTarget {
    /// One manifest names one exact target; compatibility is never a Cartesian product of lists.
    pub target_id: String,
    pub operating_system: String,
    pub architecture: String,
    pub accelerator_kind: Option<String>,
    pub accelerator_abi: Option<String>,
    pub minimum_driver_versions: Vec<ComputePluginDriverRequirement>,
    pub requires_virtualization: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginDriverRequirement {
    pub driver_family: String,
    pub minimum_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginEntrypoint {
    pub entrypoint_kind: String,
    pub relative_path: String,
    pub arguments: Vec<String>,
    pub health_check: ComputePluginHealthCheck,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginHealthCheck {
    pub protocol: String,
    pub timeout_ms: i64,
    pub interval_ms: i64,
    pub healthy_after_successes: i64,
    pub unhealthy_after_failures: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginSystemDependency {
    pub dependency_id: String,
    pub version_requirement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginDownloadDependency {
    pub artifact_id: String,
    pub digest_algorithm: String,
    pub digest: String,
    pub media_type: String,
    pub size_bytes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginResourceLimits {
    pub max_cpu_millicores: i64,
    pub max_memory_bytes: i64,
    pub max_vram_bytes: i64,
    pub max_disk_bytes: i64,
    pub max_processes: i64,
    pub max_sidecar_uptime_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginPermissionProfile {
    pub allow_network_egress: bool,
    pub allowed_egress_domains: Vec<String>,
    pub filesystem_scopes: Vec<ComputePluginFilesystemScope>,
    pub allow_child_processes: bool,
    pub device_scopes: Vec<ComputePluginDeviceScope>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ComputePluginFilesystemScope {
    PluginPackageReadOnly,
    PluginStateReadWrite,
    WorkloadInputReadOnly,
    WorkloadOutputWriteOnly,
    ScratchReadWrite,
}

impl ComputePluginFilesystemScope {
    pub(crate) const fn wire_name(&self) -> &'static str {
        match self {
            Self::PluginPackageReadOnly => "plugin_package_read_only",
            Self::PluginStateReadWrite => "plugin_state_read_write",
            Self::WorkloadInputReadOnly => "workload_input_read_only",
            Self::WorkloadOutputWriteOnly => "workload_output_write_only",
            Self::ScratchReadWrite => "scratch_read_write",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ComputePluginDeviceScope {
    Accelerator,
    VideoEncoder,
    VideoDecoder,
}

impl ComputePluginDeviceScope {
    pub(crate) const fn wire_name(&self) -> &'static str {
        match self {
            Self::Accelerator => "accelerator",
            Self::VideoEncoder => "video_encoder",
            Self::VideoDecoder => "video_decoder",
        }
    }
}

/// Upgrade retention, draining and rollback are local InstallPlan policy, not publisher authority.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginStateCompatibility {
    pub state_schema: String,
    pub writes_version: String,
    pub reads_versions: Vec<String>,
}

pub(crate) fn resource_limits_are_non_negative(limits: &ComputePluginResourceLimits) -> bool {
    limits.max_cpu_millicores >= 0
        && limits.max_memory_bytes >= 0
        && limits.max_vram_bytes >= 0
        && limits.max_disk_bytes >= 0
        && limits.max_processes >= 0
        && limits.max_sidecar_uptime_seconds >= 0
}
