use serde::Serialize;

use crate::compute_federation::external_pool_adapter_artifact_security::{
    ExternalPoolAdapterArtifactSecurityExpected, ExternalPoolAdapterArtifactSecurityReceipt,
    ScannedExternalPoolAdapterArtifactSecurity,
};

pub(crate) struct CreateExternalPoolAdapterArtifactSecurityReceipt {
    pub expected: ExternalPoolAdapterArtifactSecurityExpected,
    pub scanned_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub scanned: ScannedExternalPoolAdapterArtifactSecurity,
}

pub(crate) type ExternalPoolAdapterArtifactSecurityScanTarget =
    ExternalPoolAdapterArtifactSecurityExpected;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterArtifactSecuritySummary {
    pub security_receipt_id: String,
    pub security_receipt_digest: String,
    pub security_material_digest: String,
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
    pub scanned_at: String,
    pub evidence_scope: String,
    pub artifact_format_effect: String,
    pub artifact_security_effect: String,
    pub vulnerability_intelligence_effect: String,
    pub conformance_effect: String,
    pub adapter_effect: String,
    pub route_effect: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterArtifactSecurityWriteReceipt {
    pub security: ExternalPoolAdapterArtifactSecuritySummary,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterArtifactSecurityCurrentnessReceipt {
    pub schema: &'static str,
    pub security: ExternalPoolAdapterArtifactSecuritySummary,
    pub current_status: String,
    pub admission_current_status: String,
    pub signer_current_status: String,
}

pub(super) struct StoredArtifactSecurityReceipt {
    pub receipt: ExternalPoolAdapterArtifactSecurityReceipt,
    pub receipt_json: String,
}

/// Same-transaction authority over one exact V233 receipt. It is deliberately
/// non-serializable so later gates cannot substitute a client projection.
pub(in crate::store) struct ExternalPoolAdapterArtifactSecurityAuthority {
    receipt: ExternalPoolAdapterArtifactSecurityReceipt,
}

impl ExternalPoolAdapterArtifactSecurityAuthority {
    pub(super) fn new(receipt: ExternalPoolAdapterArtifactSecurityReceipt) -> Self {
        Self { receipt }
    }

    pub(in crate::store) fn receipt(&self) -> &ExternalPoolAdapterArtifactSecurityReceipt {
        &self.receipt
    }
}

impl StoredArtifactSecurityReceipt {
    pub(super) fn summary(&self) -> ExternalPoolAdapterArtifactSecuritySummary {
        let item = &self.receipt.security;
        ExternalPoolAdapterArtifactSecuritySummary {
            security_receipt_id: self.receipt.security_receipt_id.clone(),
            security_receipt_digest: self.receipt.security_receipt_digest.clone(),
            security_material_digest: self.receipt.security_material_digest.clone(),
            admission_id: item.admission_id.clone(),
            admission_digest: item.admission_digest.clone(),
            source_receipt_digest: item.source_receipt_digest.clone(),
            provenance_receipt_digest: item.provenance_receipt_digest.clone(),
            package_receipt_id: item.package_receipt_id.clone(),
            package_receipt_digest: item.package_receipt_digest.clone(),
            archive_sha256: item.archive_sha256.clone(),
            archive_size_bytes: item.archive_size_bytes,
            package_inspection_digest: item.package_inspection_digest.clone(),
            manifest_digest: item.manifest_digest.clone(),
            sbom_digest: item.sbom_digest.clone(),
            component_inventory_digest: item.component_inventory_digest.clone(),
            component_count: item.component_count,
            license_inventory_digest: item.license_inventory_digest.clone(),
            license_count: item.license_count,
            scanned_file_inventory_digest: item.scanned_file_inventory_digest.clone(),
            scanned_file_count: item.scanned_file_count,
            scanner_rule_set_id: item.scanner_rule_set_id.clone(),
            scanner_rule_set_digest: item.scanner_rule_set_digest.clone(),
            license_policy_id: item.license_policy_id.clone(),
            finding_count: item.finding_count,
            inspection_digest: item.inspection_digest.clone(),
            scanned_by_admin_user_id: item.scanned_by_admin_user_id.clone(),
            scanned_at: item.scanned_at.clone(),
            evidence_scope: item.evidence_scope.clone(),
            artifact_format_effect: item.artifact_format_effect.clone(),
            artifact_security_effect: item.artifact_security_effect.clone(),
            vulnerability_intelligence_effect: item.vulnerability_intelligence_effect.clone(),
            conformance_effect: item.conformance_effect.clone(),
            adapter_effect: item.adapter_effect.clone(),
            route_effect: item.route_effect.clone(),
        }
    }
}
