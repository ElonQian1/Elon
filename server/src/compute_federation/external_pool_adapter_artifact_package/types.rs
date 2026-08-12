use serde::{Deserialize, Serialize};

use crate::compute_federation::external_pool_adapter_artifact_source::CurrentQuarantinedExternalPoolAdapterArtifactBytes;
use crate::compute_federation::external_pool_adapter_release::{
    ComputeExternalPoolAdapterReleaseCapability, ComputeExternalPoolAdapterReleaseVerifierIntent,
};

pub(crate) const ARTIFACT_PACKAGE_MANIFEST_PATH: &str = "elon-adapter-manifest.json";
pub(crate) const ARTIFACT_PACKAGE_MANIFEST_SCHEMA: &str =
    "compute_federation.external_pool_adapter_artifact_manifest.v1";
pub(crate) const ARTIFACT_PACKAGE_RECEIPT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_artifact_package_receipt.v1";
pub(crate) const ARTIFACT_PACKAGE_CURRENTNESS_SCHEMA: &str =
    "compute_federation.external_pool_adapter_artifact_package_currentness.v1";
pub(crate) const ARTIFACT_PACKAGE_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const ARTIFACT_PACKAGE_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const ARTIFACT_PACKAGE_FORMAT: &str = "zip";
pub(crate) const ARTIFACT_PACKAGE_RUNTIME_KIND: &str = "server_sidecar_v1";
pub(crate) const ARTIFACT_PACKAGE_ENTRYPOINT_ROLE: &str = "entrypoint";
pub(crate) const ARTIFACT_PACKAGE_RESOURCE_ROLE: &str = "resource";
pub(crate) const ARTIFACT_PACKAGE_CONFIRMATION: &str =
    "confirm_external_pool_adapter_artifact_package_inspection";
pub(crate) const ARTIFACT_PACKAGE_EVIDENCE_SCOPE: &str = "bounded_static_zip_manifest_match";
pub(crate) const ARTIFACT_PACKAGE_FORMAT_EFFECT: &str = "static_format_verified";
pub(crate) const ARTIFACT_PACKAGE_NO_EFFECT: &str = "none";
pub(crate) const MAX_ARTIFACT_PACKAGE_ENTRIES: usize = 128;
pub(crate) const MAX_ARTIFACT_PACKAGE_MANIFEST_BYTES: u64 = 64 * 1024;
pub(crate) const MAX_ARTIFACT_PACKAGE_ENTRY_BYTES: u64 = 32 * 1024 * 1024;
pub(crate) const MAX_ARTIFACT_PACKAGE_UNCOMPRESSED_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterArtifactManifest {
    pub schema: String,
    pub adapter_id: String,
    pub release_version: String,
    pub package_format: String,
    pub runtime: ExternalPoolAdapterArtifactRuntime,
    pub supported_capabilities: Vec<ComputeExternalPoolAdapterReleaseCapability>,
    pub capability_set_digest: String,
    pub credential_verifier: ComputeExternalPoolAdapterReleaseVerifierIntent,
    pub files: Vec<ExternalPoolAdapterArtifactManifestFile>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterArtifactRuntime {
    pub kind: String,
    pub entrypoint: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterArtifactManifestFile {
    pub path: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub role: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterArtifactPackageInspection {
    pub archive_sha256: String,
    pub archive_size_bytes: u64,
    pub manifest: ExternalPoolAdapterArtifactManifest,
    pub manifest_canonical_json: String,
    pub manifest_digest: String,
    pub entry_inventory_digest: String,
    pub entry_count: u64,
    pub total_uncompressed_bytes: u64,
    pub inspection_digest: String,
}

/// Non-forgeable static inspection evidence retaining the exact verified CAS handle.
///
/// This value is intentionally non-Clone and non-Serde. Its only constructor lives in the
/// bounded ZIP inspector, and the Store consumes it before recording a receipt.
pub(crate) struct InspectedExternalPoolAdapterArtifactPackage {
    pub(super) artifact: CurrentQuarantinedExternalPoolAdapterArtifactBytes,
    pub(super) inspection: ExternalPoolAdapterArtifactPackageInspection,
}

impl InspectedExternalPoolAdapterArtifactPackage {
    pub(crate) fn inspection(&self) -> &ExternalPoolAdapterArtifactPackageInspection {
        &self.inspection
    }

    pub(crate) fn artifact_digest(&self) -> &str {
        self.artifact.content_address_digest()
    }

    pub(crate) fn artifact_size_bytes(&self) -> u64 {
        self.artifact.artifact_size_bytes()
    }

    pub(crate) fn artifact_reader(&mut self) -> &mut std::fs::File {
        self.artifact.reader()
    }

    pub(crate) fn into_artifact(self) -> CurrentQuarantinedExternalPoolAdapterArtifactBytes {
        self.artifact
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterArtifactPackageReceiptMaterial {
    pub admission_id: String,
    pub admission_digest: String,
    pub source_receipt_digest: String,
    pub provenance_receipt_id: String,
    pub provenance_receipt_digest: String,
    pub archive_sha256: String,
    pub archive_size_bytes: u64,
    pub manifest: ExternalPoolAdapterArtifactManifest,
    pub manifest_canonical_json: String,
    pub manifest_digest: String,
    pub entry_inventory_digest: String,
    pub entry_count: u64,
    pub total_uncompressed_bytes: u64,
    pub inspection_digest: String,
    pub inspected_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub inspected_at: String,
    pub recorded_at: String,
    pub evidence_scope: String,
    pub artifact_format_effect: String,
    pub artifact_security_effect: String,
    pub conformance_effect: String,
    pub adapter_effect: String,
    pub route_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterArtifactPackageReceipt {
    pub schema: String,
    pub package_receipt_id: String,
    pub package_receipt_digest: String,
    pub package_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub package: ExternalPoolAdapterArtifactPackageReceiptMaterial,
}

pub(crate) struct ExternalPoolAdapterArtifactPackageExpected<'a> {
    pub adapter_id: &'a str,
    pub release_version: &'a str,
    pub artifact_sha256: &'a str,
    pub artifact_size_bytes: u64,
    pub supported_capabilities: &'a [ComputeExternalPoolAdapterReleaseCapability],
    pub capability_set_digest: &'a str,
    pub credential_verifier: &'a ComputeExternalPoolAdapterReleaseVerifierIntent,
}
