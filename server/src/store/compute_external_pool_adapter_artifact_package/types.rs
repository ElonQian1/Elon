use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_federation::{
    external_pool_adapter_artifact_package::{
        ExternalPoolAdapterArtifactPackageExpected, ExternalPoolAdapterArtifactPackageReceipt,
        InspectedExternalPoolAdapterArtifactPackage,
    },
    external_pool_adapter_release::{
        ComputeExternalPoolAdapterReleaseCapability,
        ComputeExternalPoolAdapterReleaseVerifierIntent,
    },
};

pub(crate) struct CreateExternalPoolAdapterArtifactPackageReceipt {
    pub expected_admission_id: String,
    pub expected_admission_digest: String,
    pub expected_source_receipt_digest: String,
    pub expected_provenance_receipt_digest: String,
    pub inspected_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub inspected: InspectedExternalPoolAdapterArtifactPackage,
}

#[derive(Clone)]
pub(crate) struct ExternalPoolAdapterArtifactPackageInspectionTarget {
    pub admission_id: String,
    pub admission_digest: String,
    pub adapter_id: String,
    pub release_version: String,
    pub source_receipt_digest: String,
    pub provenance_receipt_id: String,
    pub provenance_receipt_digest: String,
    pub artifact_sha256: String,
    pub artifact_size_bytes: u64,
    pub supported_capabilities: Vec<ComputeExternalPoolAdapterReleaseCapability>,
    pub capability_set_digest: String,
    pub credential_verifier: ComputeExternalPoolAdapterReleaseVerifierIntent,
}

impl ExternalPoolAdapterArtifactPackageInspectionTarget {
    pub(crate) fn expected(&self) -> ExternalPoolAdapterArtifactPackageExpected<'_> {
        ExternalPoolAdapterArtifactPackageExpected {
            adapter_id: &self.adapter_id,
            release_version: &self.release_version,
            artifact_sha256: &self.artifact_sha256,
            artifact_size_bytes: self.artifact_size_bytes,
            supported_capabilities: &self.supported_capabilities,
            capability_set_digest: &self.capability_set_digest,
            credential_verifier: &self.credential_verifier,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterArtifactPackageSummary {
    pub package_receipt_id: String,
    pub package_receipt_digest: String,
    pub package_material_digest: String,
    pub admission_id: String,
    pub admission_digest: String,
    pub source_receipt_digest: String,
    pub provenance_receipt_id: String,
    pub provenance_receipt_digest: String,
    pub archive_sha256: String,
    pub archive_size_bytes: u64,
    pub manifest_digest: String,
    pub entry_inventory_digest: String,
    pub entry_count: u64,
    pub total_uncompressed_bytes: u64,
    pub inspection_digest: String,
    pub adapter_id: String,
    pub release_version: String,
    pub runtime_kind: String,
    pub entrypoint_path_digest: String,
    pub capability_set_digest: String,
    pub credential_verifier_digest: String,
    pub inspected_by_admin_user_id: String,
    pub inspected_at: String,
    pub evidence_scope: String,
    pub artifact_format_effect: String,
    pub artifact_security_effect: String,
    pub conformance_effect: String,
    pub adapter_effect: String,
    pub route_effect: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterArtifactPackageWriteReceipt {
    pub package: ExternalPoolAdapterArtifactPackageSummary,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterArtifactPackageCurrentnessReceipt {
    pub schema: &'static str,
    pub package: ExternalPoolAdapterArtifactPackageSummary,
    pub current_status: String,
    pub admission_current_status: String,
    pub signer_current_status: String,
}

pub(super) struct StoredArtifactPackageReceipt {
    pub receipt: ExternalPoolAdapterArtifactPackageReceipt,
    pub receipt_json: String,
}

/// Sealed, same-transaction proof for one immutable V232 package receipt.
pub(in crate::store) struct ExternalPoolAdapterArtifactPackageAuthority {
    receipt: ExternalPoolAdapterArtifactPackageReceipt,
}

impl ExternalPoolAdapterArtifactPackageAuthority {
    pub(super) fn new(receipt: ExternalPoolAdapterArtifactPackageReceipt) -> Self {
        Self { receipt }
    }

    pub(in crate::store) fn receipt(&self) -> &ExternalPoolAdapterArtifactPackageReceipt {
        &self.receipt
    }
}

impl StoredArtifactPackageReceipt {
    pub(super) fn summary(&self) -> ExternalPoolAdapterArtifactPackageSummary {
        let package = &self.receipt.package;
        ExternalPoolAdapterArtifactPackageSummary {
            package_receipt_id: self.receipt.package_receipt_id.clone(),
            package_receipt_digest: self.receipt.package_receipt_digest.clone(),
            package_material_digest: self.receipt.package_material_digest.clone(),
            admission_id: package.admission_id.clone(),
            admission_digest: package.admission_digest.clone(),
            source_receipt_digest: package.source_receipt_digest.clone(),
            provenance_receipt_id: package.provenance_receipt_id.clone(),
            provenance_receipt_digest: package.provenance_receipt_digest.clone(),
            archive_sha256: package.archive_sha256.clone(),
            archive_size_bytes: package.archive_size_bytes,
            manifest_digest: package.manifest_digest.clone(),
            entry_inventory_digest: package.entry_inventory_digest.clone(),
            entry_count: package.entry_count,
            total_uncompressed_bytes: package.total_uncompressed_bytes,
            inspection_digest: package.inspection_digest.clone(),
            adapter_id: package.manifest.adapter_id.clone(),
            release_version: package.manifest.release_version.clone(),
            runtime_kind: package.manifest.runtime.kind.clone(),
            entrypoint_path_digest: entrypoint_path_digest(&package.manifest.runtime.entrypoint),
            capability_set_digest: package.manifest.capability_set_digest.clone(),
            credential_verifier_digest: package
                .manifest
                .credential_verifier
                .verifier_digest
                .clone(),
            inspected_by_admin_user_id: package.inspected_by_admin_user_id.clone(),
            inspected_at: package.inspected_at.clone(),
            evidence_scope: package.evidence_scope.clone(),
            artifact_format_effect: package.artifact_format_effect.clone(),
            artifact_security_effect: package.artifact_security_effect.clone(),
            conformance_effect: package.conformance_effect.clone(),
            adapter_effect: package.adapter_effect.clone(),
            route_effect: package.route_effect.clone(),
        }
    }
}

fn entrypoint_path_digest(path: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(b"ELON-EXTERNAL-POOL-ADAPTER-ENTRYPOINT-PATH-V1");
    digest.update([0]);
    digest.update(path.as_bytes());
    hex::encode(digest.finalize())
}
