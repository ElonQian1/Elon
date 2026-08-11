use anyhow::{bail, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::types::{StoredArtifactSource, StoredArtifactSourceEnvelope};

const MAX_ARTIFACT_SOURCE_JSON_BYTES: usize = 512 * 1024;
const RECEIPT_DIGEST_DOMAIN: &[u8] =
    b"ELON-COMPUTE-EXTERNAL-POOL-ADAPTER-ARTIFACT-SOURCE-RECEIPT-V1";
const INTAKE_MATERIAL_DIGEST_DOMAIN: &[u8] =
    b"ELON-COMPUTE-EXTERNAL-POOL-ADAPTER-ARTIFACT-SOURCE-INTAKE-MATERIAL-V1";

pub(super) fn canonical_receipt_json_and_digest(
    envelope: &StoredArtifactSourceEnvelope,
) -> Result<(String, String)> {
    let value = serde_json::to_value(envelope)?;
    let object = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("artifact source receipt envelope is not an object"))?;
    let mut projection = object.clone();
    if projection
        .insert(
            "source_receipt_digest".to_string(),
            serde_json::Value::String(String::new()),
        )
        .is_none()
    {
        bail!("artifact source receipt envelope lacks source_receipt_digest");
    }
    let digest = domain_digest(RECEIPT_DIGEST_DOMAIN, &projection)?;
    let json = canonical_json(envelope)?;
    if !envelope.source_receipt_digest.is_empty() && envelope.source_receipt_digest != digest {
        bail!("artifact source receipt digest mismatch");
    }
    Ok((json, digest))
}

pub(super) fn canonical_intake_material_digest(source: &StoredArtifactSource) -> Result<String> {
    #[derive(Serialize)]
    struct IntakeMaterial<'a> {
        admission_id: &'a str,
        admission_digest: &'a str,
        request_id: &'a str,
        request_digest: &'a str,
        request_material_digest: &'a str,
        review_id: &'a str,
        review_digest: &'a str,
        adapter_id: &'a str,
        release_version: &'a str,
        candidate_artifact_ref: &'a str,
        declared_implementation_sha256: &'a str,
        intake_sha256: &'a str,
        reopened_sha256: &'a str,
        artifact_size_bytes: i64,
        storage_root_kind: &'a str,
        storage_namespace: &'a str,
        content_address_algorithm: &'a str,
        content_address_digest: &'a str,
        custody_state: &'a str,
        intake_kind: &'a str,
        evidence_scope: &'a str,
        artifact_ref_resolution_effect: &'a str,
        adapter_effect: &'a str,
        route_effect: &'a str,
        recorded_by_admin_user_id: &'a str,
        intake_confirmation: &'a str,
        idempotency_scope: &'a str,
        idempotency_key: &'a str,
    }

    domain_digest(
        INTAKE_MATERIAL_DIGEST_DOMAIN,
        &IntakeMaterial {
            admission_id: &source.admission_id,
            admission_digest: &source.admission_digest,
            request_id: &source.request_id,
            request_digest: &source.request_digest,
            request_material_digest: &source.request_material_digest,
            review_id: &source.review_id,
            review_digest: &source.review_digest,
            adapter_id: &source.adapter_id,
            release_version: &source.release_version,
            candidate_artifact_ref: &source.candidate_artifact_ref,
            declared_implementation_sha256: &source.declared_implementation_sha256,
            intake_sha256: &source.intake_sha256,
            reopened_sha256: &source.reopened_sha256,
            artifact_size_bytes: source.artifact_size_bytes,
            storage_root_kind: &source.storage_root_kind,
            storage_namespace: &source.storage_namespace,
            content_address_algorithm: &source.content_address_algorithm,
            content_address_digest: &source.content_address_digest,
            custody_state: &source.custody_state,
            intake_kind: &source.intake_kind,
            evidence_scope: &source.evidence_scope,
            artifact_ref_resolution_effect: &source.artifact_ref_resolution_effect,
            adapter_effect: &source.adapter_effect,
            route_effect: &source.route_effect,
            recorded_by_admin_user_id: &source.recorded_by_admin_user_id,
            intake_confirmation: &source.intake_confirmation,
            idempotency_scope: &source.idempotency_scope,
            idempotency_key: &source.idempotency_key,
        },
    )
}

pub(super) fn canonical_json<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    canonical_compute_plugin_ijson_and_sha256(value, MAX_ARTIFACT_SOURCE_JSON_BYTES)
        .map(|(json, _)| json)
}

fn domain_digest<T: Serialize + ?Sized>(domain: &[u8], value: &T) -> Result<String> {
    let json = canonical_json(value)?;
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(json.as_bytes());
    Ok(hex::encode(digest.finalize()))
}
