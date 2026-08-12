use serde::{Deserialize, Serialize};

use crate::compute_federation::{
    external_pool_adapter_artifact_package::{
        ExternalPoolAdapterArtifactManifest, ExternalPoolAdapterArtifactPackageExpected,
        ExternalPoolAdapterArtifactPackageInspection, InspectedExternalPoolAdapterArtifactPackage,
    },
    external_pool_adapter_artifact_source::CurrentQuarantinedExternalPoolAdapterArtifactBytes,
};

pub(crate) const ARTIFACT_SBOM_PATH: &str = "elon-adapter-sbom.json";
pub(crate) const ARTIFACT_SBOM_SCHEMA: &str = "compute_federation.external_pool_adapter_sbom.v1";
pub(crate) const ARTIFACT_SECURITY_RECEIPT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_artifact_security_receipt.v1";
pub(crate) const ARTIFACT_SECURITY_CURRENTNESS_SCHEMA: &str =
    "compute_federation.external_pool_adapter_artifact_security_currentness.v1";
pub(crate) const ARTIFACT_SECURITY_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const ARTIFACT_SECURITY_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const ARTIFACT_SECURITY_CONFIRMATION: &str =
    "confirm_external_pool_adapter_artifact_static_security_scan";
pub(crate) const ARTIFACT_SECURITY_RULE_SET_ID: &str = "elon_adapter_static_safety_v1";
pub(crate) const ARTIFACT_SECURITY_RULES: &[&str] = &[
    "deny_embedded_private_key_pem_v1",
    "deny_known_cloud_access_token_prefix_v1",
    "deny_nested_zip_payload_v1",
    "require_exact_manifest_rehash_v1",
];
pub(crate) const ARTIFACT_SECURITY_LICENSE_POLICY_ID: &str = "declared_single_spdx_identifier_v1";
pub(crate) const ARTIFACT_SECURITY_EVIDENCE_SCOPE: &str =
    "exact_sbom_license_and_local_static_rules";
pub(crate) const ARTIFACT_SECURITY_EFFECT: &str = "static_policy_verified";
pub(crate) const ARTIFACT_SECURITY_NO_EFFECT: &str = "none";
pub(crate) const MAX_ARTIFACT_SBOM_BYTES: u64 = 256 * 1024;
pub(crate) const MAX_ARTIFACT_SBOM_COMPONENTS: usize = 128;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterArtifactSbom {
    pub schema: String,
    pub adapter_id: String,
    pub release_version: String,
    pub components: Vec<ExternalPoolAdapterArtifactSbomComponent>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterArtifactSbomComponent {
    pub component_id: String,
    pub name: String,
    pub version: String,
    pub supplier: String,
    pub package_url: String,
    pub license_spdx_id: String,
    pub file_paths: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterArtifactSecurityInspection {
    pub archive_sha256: String,
    pub archive_size_bytes: u64,
    pub package_receipt_digest: String,
    pub package_inspection_digest: String,
    pub manifest_digest: String,
    pub sbom_canonical_json: String,
    pub sbom_digest: String,
    pub component_inventory_digest: String,
    pub component_count: u64,
    pub license_inventory_digest: String,
    pub license_count: u64,
    pub scanned_file_inventory_digest: String,
    pub scanned_file_count: u64,
    pub scanner_rule_set_id: String,
    pub scanner_rule_set_digest: String,
    pub finding_count: u64,
    pub inspection_digest: String,
}

/// Non-forgeable scan evidence retaining the exact V232-reinspected CAS handle.
pub(crate) struct ScannedExternalPoolAdapterArtifactSecurity {
    pub(super) artifact: CurrentQuarantinedExternalPoolAdapterArtifactBytes,
    pub(super) package_inspection: ExternalPoolAdapterArtifactPackageInspection,
    pub(super) inspection: ExternalPoolAdapterArtifactSecurityInspection,
}

impl ScannedExternalPoolAdapterArtifactSecurity {
    pub(crate) fn artifact_digest(&self) -> &str {
        self.artifact.content_address_digest()
    }

    pub(crate) fn artifact_size_bytes(&self) -> u64 {
        self.artifact.artifact_size_bytes()
    }

    pub(crate) fn package_inspection(&self) -> &ExternalPoolAdapterArtifactPackageInspection {
        &self.package_inspection
    }

    pub(crate) fn inspection(&self) -> &ExternalPoolAdapterArtifactSecurityInspection {
        &self.inspection
    }
}

#[derive(Clone)]
pub(crate) struct ExternalPoolAdapterArtifactSecurityExpected {
    pub admission_id: String,
    pub admission_digest: String,
    pub source_receipt_digest: String,
    pub provenance_receipt_digest: String,
    pub package_receipt_id: String,
    pub package_receipt_digest: String,
    pub archive_sha256: String,
    pub archive_size_bytes: u64,
    pub manifest: ExternalPoolAdapterArtifactManifest,
    pub manifest_digest: String,
    pub package_inspection_digest: String,
}

impl ExternalPoolAdapterArtifactSecurityExpected {
    pub(crate) fn package_expected(&self) -> ExternalPoolAdapterArtifactPackageExpected<'_> {
        ExternalPoolAdapterArtifactPackageExpected {
            adapter_id: &self.manifest.adapter_id,
            release_version: &self.manifest.release_version,
            artifact_sha256: &self.archive_sha256,
            artifact_size_bytes: self.archive_size_bytes,
            supported_capabilities: &self.manifest.supported_capabilities,
            capability_set_digest: &self.manifest.capability_set_digest,
            credential_verifier: &self.manifest.credential_verifier,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterArtifactSecurityReceiptMaterial {
    pub admission_id: String,
    pub admission_digest: String,
    pub source_receipt_digest: String,
    pub provenance_receipt_digest: String,
    pub package_receipt_id: String,
    pub package_receipt_digest: String,
    pub archive_sha256: String,
    pub archive_size_bytes: u64,
    pub package_inspection_digest: String,
    pub manifest_digest: String,
    pub sbom_canonical_json: String,
    pub sbom_digest: String,
    pub component_inventory_digest: String,
    pub component_count: u64,
    pub license_inventory_digest: String,
    pub license_count: u64,
    pub scanned_file_inventory_digest: String,
    pub scanned_file_count: u64,
    pub scanner_rule_set_id: String,
    pub scanner_rule_set_digest: String,
    pub license_policy_id: String,
    pub finding_count: u64,
    pub inspection_digest: String,
    pub scanned_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub scanned_at: String,
    pub recorded_at: String,
    pub evidence_scope: String,
    pub artifact_format_effect: String,
    pub artifact_security_effect: String,
    pub vulnerability_intelligence_effect: String,
    pub conformance_effect: String,
    pub adapter_effect: String,
    pub route_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterArtifactSecurityReceipt {
    pub schema: String,
    pub security_receipt_id: String,
    pub security_receipt_digest: String,
    pub security_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub security: ExternalPoolAdapterArtifactSecurityReceiptMaterial,
}

pub(crate) fn split_scanned(
    inspected: InspectedExternalPoolAdapterArtifactPackage,
    inspection: ExternalPoolAdapterArtifactSecurityInspection,
) -> ScannedExternalPoolAdapterArtifactSecurity {
    let package_inspection = inspected.inspection().clone();
    let artifact = inspected.into_artifact();
    ScannedExternalPoolAdapterArtifactSecurity {
        artifact,
        package_inspection,
        inspection,
    }
}
