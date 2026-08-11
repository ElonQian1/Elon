use serde::{Deserialize, Serialize};

use crate::compute_federation::external_pool_adapter_artifact_source::QuarantinedExternalPoolAdapterArtifactBytes;

pub(crate) const EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_INTAKE_CONFIRMATION: &str =
    "confirm_external_pool_adapter_artifact_source_intake";
pub(super) const EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_RECEIPT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_artifact_source_receipt.v1";
pub(super) const EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_CANONICALIZATION: &str = "rfc8785_jcs";
pub(super) const EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_DIGEST_ALGORITHM: &str = "sha256";
pub(super) const EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_STORAGE_ROOT_KIND: &str = "server_data_dir";
pub(super) const EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_STORAGE_NAMESPACE: &str =
    "compute-federation/external-pool-adapter-artifacts/v1/quarantine";
pub(super) const EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_CUSTODY_STATE: &str = "quarantined";
pub(super) const EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_INTAKE_KIND: &str =
    "admin_authenticated_raw_body";
pub(super) const EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_EVIDENCE_SCOPE: &str =
    "byte_digest_match_only";
pub(super) const EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_NO_EFFECT: &str = "none";
pub(super) const MAX_EXTERNAL_POOL_ADAPTER_ARTIFACT_SIZE_BYTES: u64 = 32 * 1024 * 1024;

pub(crate) struct RecordExternalPoolAdapterArtifactSource {
    pub admission_id: String,
    pub expected_admission_digest: String,
    pub recorded_by_admin_user_id: String,
    pub intake_confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub artifact: QuarantinedExternalPoolAdapterArtifactBytes,
}

/// Read-only v222 intake authority. It is not artifact or execution authority.
pub(crate) struct ExternalPoolAdapterArtifactIntakeAuthority {
    admission_id: String,
    admission_digest: String,
    declared_implementation_sha256: String,
}

impl ExternalPoolAdapterArtifactIntakeAuthority {
    pub(super) fn new(
        admission_id: String,
        admission_digest: String,
        declared_implementation_sha256: String,
    ) -> Self {
        Self {
            admission_id,
            admission_digest,
            declared_implementation_sha256,
        }
    }

    pub(crate) fn admission_id(&self) -> &str {
        &self.admission_id
    }

    pub(crate) fn admission_digest(&self) -> &str {
        &self.admission_digest
    }

    pub(crate) fn declared_implementation_sha256(&self) -> &str {
        &self.declared_implementation_sha256
    }
}

#[derive(Clone, Serialize)]
pub(crate) struct ExternalPoolAdapterArtifactSourceReceipt {
    pub schema: &'static str,
    pub source_receipt_id: String,
    pub source_receipt_digest: String,
    pub intake_material_digest: String,
    pub admission_id: String,
    pub admission_digest: String,
    pub request_id: String,
    pub request_digest: String,
    pub request_material_digest: String,
    pub review_id: String,
    pub review_digest: String,
    pub adapter_id: String,
    pub release_version: String,
    pub declared_implementation_sha256: String,
    pub intake_sha256: String,
    pub reopened_sha256: String,
    #[serde(skip)]
    artifact_size_bytes: u64,
    pub storage_root_kind: &'static str,
    pub storage_namespace: &'static str,
    pub content_address_algorithm: &'static str,
    #[serde(skip)]
    content_address_digest: String,
    pub custody_state: &'static str,
    pub intake_kind: &'static str,
    pub evidence_scope: &'static str,
    pub artifact_ref_resolution_effect: &'static str,
    pub adapter_effect: &'static str,
    pub route_effect: &'static str,
    pub recorded_by_admin_user_id: String,
    pub recorded_at: String,
    pub replayed: bool,
}

impl ExternalPoolAdapterArtifactSourceReceipt {
    pub(crate) fn content_address_digest(&self) -> &str {
        &self.content_address_digest
    }

    pub(crate) fn artifact_size_bytes(&self) -> u64 {
        self.artifact_size_bytes
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct StoredArtifactSourceEnvelope {
    pub schema: String,
    pub source_receipt_id: String,
    pub source_receipt_digest: String,
    pub intake_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub source: StoredArtifactSource,
}

#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct StoredArtifactSource {
    pub admission_id: String,
    pub admission_digest: String,
    pub request_id: String,
    pub request_digest: String,
    pub request_material_digest: String,
    pub review_id: String,
    pub review_digest: String,
    pub adapter_id: String,
    pub release_version: String,
    pub candidate_artifact_ref: String,
    pub declared_implementation_sha256: String,
    pub intake_sha256: String,
    pub reopened_sha256: String,
    pub artifact_size_bytes: i64,
    pub storage_root_kind: String,
    pub storage_namespace: String,
    pub content_address_algorithm: String,
    pub content_address_digest: String,
    pub custody_state: String,
    pub intake_kind: String,
    pub evidence_scope: String,
    pub artifact_ref_resolution_effect: String,
    pub adapter_effect: String,
    pub route_effect: String,
    pub recorded_by_admin_user_id: String,
    pub intake_confirmation: String,
    pub recorded_at: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub created_at: String,
}

pub(super) struct StoredArtifactSourceReceipt {
    pub envelope: StoredArtifactSourceEnvelope,
    pub source_receipt_json: String,
}

impl StoredArtifactSourceReceipt {
    pub(super) fn into_receipt(self, replayed: bool) -> ExternalPoolAdapterArtifactSourceReceipt {
        let source = self.envelope.source;
        ExternalPoolAdapterArtifactSourceReceipt {
            schema: EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_RECEIPT_SCHEMA,
            source_receipt_id: self.envelope.source_receipt_id,
            source_receipt_digest: self.envelope.source_receipt_digest,
            intake_material_digest: self.envelope.intake_material_digest,
            admission_id: source.admission_id,
            admission_digest: source.admission_digest,
            request_id: source.request_id,
            request_digest: source.request_digest,
            request_material_digest: source.request_material_digest,
            review_id: source.review_id,
            review_digest: source.review_digest,
            adapter_id: source.adapter_id,
            release_version: source.release_version,
            declared_implementation_sha256: source.declared_implementation_sha256,
            intake_sha256: source.intake_sha256,
            reopened_sha256: source.reopened_sha256,
            artifact_size_bytes: source.artifact_size_bytes as u64,
            storage_root_kind: EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_STORAGE_ROOT_KIND,
            storage_namespace: EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_STORAGE_NAMESPACE,
            content_address_algorithm: EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_DIGEST_ALGORITHM,
            content_address_digest: source.content_address_digest,
            custody_state: EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_CUSTODY_STATE,
            intake_kind: EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_INTAKE_KIND,
            evidence_scope: EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_EVIDENCE_SCOPE,
            artifact_ref_resolution_effect: EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_NO_EFFECT,
            adapter_effect: EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_NO_EFFECT,
            route_effect: EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_NO_EFFECT,
            recorded_by_admin_user_id: source.recorded_by_admin_user_id,
            recorded_at: source.recorded_at,
            replayed,
        }
    }
}
