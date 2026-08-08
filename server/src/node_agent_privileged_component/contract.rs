use serde::{Deserialize, Serialize};

pub(crate) const PRIVILEGED_COMPONENT_MANIFEST_SCHEMA: &str =
    "elon.node.privileged_component.manifest.v1";
pub(crate) const SIGNED_PRIVILEGED_COMPONENT_MANIFEST_SCHEMA: &str =
    "elon.node.privileged_component.signed_manifest.v1";
pub(crate) const PRIVILEGED_COMPONENT_INSTALL_PLAN_SCHEMA: &str =
    "elon.node.privileged_component.install_plan.v1";
pub(crate) const SIGNED_PRIVILEGED_COMPONENT_INSTALL_PLAN_SCHEMA: &str =
    "elon.node.privileged_component.signed_install_plan.v1";
pub(crate) const PRIVILEGED_COMPONENT_MANIFEST_SIGNATURE_DOMAIN: &str =
    "ELON-NODE-PRIVILEGED-COMPONENT-MANIFEST-V1";
pub(crate) const PRIVILEGED_COMPONENT_INSTALL_PLAN_SIGNATURE_DOMAIN: &str =
    "ELON-NODE-PRIVILEGED-COMPONENT-INSTALL-PLAN-V1";
pub(crate) const PRIVILEGED_COMPONENT_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const PRIVILEGED_COMPONENT_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const PRIVILEGED_COMPONENT_SIGNATURE_ALGORITHM: &str = "ed25519";
pub(crate) const PRIVILEGED_COMPONENT_RELEASE_KEY_PURPOSE: &str =
    "node_privileged_component_release";
pub(crate) const PRIVILEGED_COMPONENT_INSTALL_PLAN_KEY_PURPOSE: &str =
    "node_privileged_component_install_plan";

pub(crate) const WINDOWS_NAMESPACE_FENCE_COMPONENT_ID: &str =
    "elon.windows.compute_namespace_fence";
pub(crate) const WINDOWS_NAMESPACE_FENCE_BACKEND_KIND: &str =
    "windows_signed_minifilter_child_namespace_fence_v1";
pub(crate) const WINDOWS_NAMESPACE_FENCE_SERVICE_NAME: &str = "ElonComputeNamespaceFence";
pub(crate) const WINDOWS_NAMESPACE_FENCE_FILTER_NAME: &str = "ElonComputeNamespaceFence";
pub(crate) const WINDOWS_NAMESPACE_FENCE_INSTANCE_NAME: &str = "ElonComputeNamespaceFence.Instance";
pub(crate) const WINDOWS_NAMESPACE_FENCE_PORT_NAME: &str = "\\ElonComputeNamespaceFencePort";
pub(crate) const WINDOWS_NAMESPACE_FENCE_PROTOCOL_ID: &str =
    "elon.windows.compute_namespace_fence.v1";
pub(crate) const WINDOWS_NAMESPACE_FENCE_PROTOCOL_REVISION: u32 = 1;
pub(crate) const WINDOWS_NAMESPACE_FENCE_PROTOCOL_MAGIC: &str = "ELONFNC1";
pub(crate) const WINDOWS_NAMESPACE_FENCE_WIRE_MAJOR_REVISION: u16 = 1;
pub(crate) const WINDOWS_NAMESPACE_FENCE_WIRE_MINOR_REVISION: u16 = 0;
pub(crate) const WINDOWS_NAMESPACE_FENCE_WIRE_BYTE_ORDER: &str = "little_endian";
/// RFC 8785 JCS SHA-256 of `docs/distributed-compute/windows-compute-namespace-fence-wire-v1.json`.
pub(crate) const WINDOWS_NAMESPACE_FENCE_WIRE_SCHEMA_SHA256: &str =
    "9557e4da4e5992ce604b2e102afd0d448d0a9fd23f5acbf49ad06a5eb17244d6";
pub(crate) const WINDOWS_NAMESPACE_FENCE_REQUIRED_FEATURE_MASK: u64 = 0x0000_0000_0000_ffff;

/// Microsoft must assign a production minifilter altitude. `None` deliberately makes the future
/// installation-policy gate fail closed instead of inventing or squatting on an altitude.
pub(crate) const WINDOWS_NAMESPACE_FENCE_ASSIGNED_ALTITUDE: Option<&str> = None;

pub(crate) const WINDOWS_NAMESPACE_FENCE_DRIVER_FILE: &str = "ElonComputeNamespaceFence.sys";
pub(crate) const WINDOWS_NAMESPACE_FENCE_INF_FILE: &str = "ElonComputeNamespaceFence.inf";
pub(crate) const WINDOWS_NAMESPACE_FENCE_CATALOG_FILE: &str = "ElonComputeNamespaceFence.cat";

pub(crate) const PRIVILEGED_COMPONENT_PLAN_ACTION_INSTALL: &str = "install";
pub(crate) const PRIVILEGED_COMPONENT_PLAN_ACTION_UPGRADE: &str = "upgrade";

/// Signature fields are metadata until a Bootstrap-pinned first-party resolver verifies them.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct PrivilegedComponentSignature {
    pub algorithm: String,
    pub signing_key_id: String,
    pub signature_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct SignedPrivilegedComponentManifest {
    pub schema: String,
    pub manifest: PrivilegedComponentManifest,
    pub canonicalization: String,
    pub manifest_digest_algorithm: String,
    pub manifest_digest: String,
    pub signature: PrivilegedComponentSignature,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct PrivilegedComponentManifest {
    pub schema: String,
    pub component_id: String,
    pub component_version: String,
    pub release_identity: String,
    pub build_git_sha: String,
    pub target: PrivilegedComponentTarget,
    pub minifilter: WindowsMinifilterIdentity,
    pub protocol: PrivilegedComponentProtocol,
    pub package: PrivilegedComponentPackage,
    pub windows_signing: WindowsDriverSigningPolicy,
    pub node_compatibility: PrivilegedComponentNodeVersionRange,
    pub rollback_generation: i64,
    pub generated_at: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct PrivilegedComponentTarget {
    pub operating_system: String,
    pub architecture: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct WindowsMinifilterIdentity {
    pub backend_kind: String,
    pub service_name: String,
    pub filter_name: String,
    pub instance_name: String,
    pub filter_altitude: String,
    pub communication_port_name: String,
    pub supported_filesystems: Vec<String>,
    pub single_client_connection_required: bool,
    pub reject_unload_with_active_grants: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct PrivilegedComponentProtocol {
    pub protocol_id: String,
    pub protocol_revision: u32,
    pub wire_magic_ascii: String,
    pub wire_major_revision: u16,
    pub wire_minor_revision: u16,
    pub wire_byte_order: String,
    pub wire_schema_sha256: String,
    pub driver_build_digest: String,
    pub required_feature_mask: u64,
    pub commands: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct PrivilegedComponentPackage {
    pub media_type: String,
    pub archive_format: String,
    pub digest_algorithm: String,
    pub package_digest: String,
    pub package_size_bytes: i64,
    pub unpacked_size_bytes: i64,
    pub files: Vec<PrivilegedComponentPackageFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct PrivilegedComponentPackageFile {
    pub role: PrivilegedComponentFileRole,
    pub relative_path: String,
    pub digest_algorithm: String,
    pub digest: String,
    pub size_bytes: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PrivilegedComponentFileRole {
    DriverBinary,
    DriverInf,
    DriverCatalog,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct WindowsDriverSigningPolicy {
    pub catalog_relative_path: String,
    pub catalog_digest_algorithm: String,
    pub catalog_digest: String,
    pub expected_catalog_publisher: String,
    pub expected_catalog_certificate_sha256: String,
    pub microsoft_kernel_trust_required: bool,
    pub test_signing_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct PrivilegedComponentNodeVersionRange {
    pub minimum_node_version: String,
    pub maximum_node_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct SignedPrivilegedComponentInstallPlan {
    pub schema: String,
    pub plan: PrivilegedComponentInstallPlan,
    pub canonicalization: String,
    pub plan_digest_algorithm: String,
    pub plan_digest: String,
    pub signature: PrivilegedComponentSignature,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct PrivilegedComponentInstallPlan {
    pub schema: String,
    pub plan_id: String,
    pub component_id: String,
    pub action: String,
    pub node_version: String,
    pub node_release_identity: String,
    pub target_architecture: String,
    pub target_manifest_digest: String,
    pub target_release_identity: String,
    pub target_package_digest: String,
    pub target_rollback_generation: i64,
    pub expected_installed_manifest_digest: Option<String>,
    pub expected_installed_release_identity: Option<String>,
    pub expected_installed_rollback_generation: Option<i64>,
    pub explicit_user_consent_required: bool,
    pub elevation_required: bool,
    pub requires_no_active_fences: bool,
    pub background_install_allowed: bool,
    pub test_signing_allowed: bool,
    pub generated_at: String,
    pub expires_at: String,
}
