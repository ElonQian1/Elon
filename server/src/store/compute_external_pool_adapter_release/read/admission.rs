use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::compute_federation::external_pool_adapter_release::{
    COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_CANONICALIZATION,
    COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_DIGEST_ALGORITHM,
};

use super::super::{
    canonical::{
        canonical_admission_json_and_digest, canonical_capabilities_json_and_digest, canonical_json,
    },
    review::{validate_digest, validate_exact, validate_optional_note},
    types::{
        canonical_nanos, StoredAdmission, ADMISSION_STATUS_STAGED,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_SCHEMA,
        EXTERNAL_POOL_ADAPTER_RELEASE_APPLY_CONFIRMATION, REVIEW_DECISION_APPROVED,
    },
};
use super::{decode, request_by_id_on, review_by_request_on};

pub(super) fn admission_by_request_on(
    conn: &Connection,
    request_id: &str,
) -> Result<Option<StoredAdmission>> {
    admission_on(conn, "WHERE request_id=?1", params![request_id])
}

pub(super) fn admission_by_adapter_release_on(
    conn: &Connection,
    adapter_id: &str,
    release_version: &str,
) -> Result<Option<StoredAdmission>> {
    admission_on(
        conn,
        "WHERE adapter_id=?1 AND release_version=?2",
        params![adapter_id, release_version],
    )
}

pub(super) fn admission_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredAdmission>> {
    admission_on(
        conn,
        "WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

fn admission_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredAdmission>> {
    let stored = conn
        .query_row(
            &format!(
                "SELECT admission_json, supported_provider_kinds_json, capabilities_json,
                        idempotency_scope, idempotency_key
                   FROM compute_external_pool_adapter_release_admissions {filter}"
            ),
            values,
            |row| {
                let admission_json: String = row.get(0)?;
                Ok(StoredAdmission {
                    envelope: decode(&admission_json, 0)?,
                    admission_json,
                    supported_provider_kinds_json: row.get(1)?,
                    capabilities_json: row.get(2)?,
                    idempotency_scope: row.get(3)?,
                    idempotency_key: row.get(4)?,
                })
            },
        )
        .optional()?;
    stored.map(|row| audit_admission(conn, row)).transpose()
}

fn audit_admission(conn: &Connection, stored: StoredAdmission) -> Result<StoredAdmission> {
    validate_admission_material(&stored)?;
    let (admission_json, admission_digest) = canonical_admission_json_and_digest(&stored.envelope)?;
    let admission = &stored.envelope.admission;
    let request = request_by_id_on(conn, &admission.request_id)?.ok_or_else(|| {
        anyhow::anyhow!("external-pool Adapter release admission lost its request")
    })?;
    let review = review_by_request_on(conn, &admission.request_id)?.ok_or_else(|| {
        anyhow::anyhow!("external-pool Adapter release admission lost its review")
    })?;
    let request_material = &request.envelope.request;
    let release = &request_material.release;
    let review_material = &review.envelope.review;
    let verifier = &admission.expected_credential_verifier;
    let provider_kinds_json = canonical_json(&admission.supported_provider_kinds)?;
    let (capabilities_json, capability_set_digest) =
        canonical_capabilities_json_and_digest(&admission.supported_capabilities)?;
    let projected = conn
        .query_row(
            "SELECT 1 FROM compute_external_pool_adapter_release_admissions
              WHERE admission_id=?1 AND admission_schema=?2 AND admission_digest=?3
                AND admission_json=?4 AND canonicalization=?5 AND digest_algorithm=?6
                AND request_id=?7 AND request_digest=?8
                AND request_material_digest=?9 AND review_id=?10
                AND review_digest=?11 AND adapter_id=?12 AND release_version=?13
                AND route_kind=?14 AND supported_provider_kinds_json=?15
                AND candidate_artifact_ref=?16 AND declared_implementation_sha256=?17
                AND capabilities_json=?18 AND capability_set_digest=?19
                AND verifier_verification_kind=?20 AND verifier_id=?21
                AND verifier_revision=?22 AND verifier_digest=?23
                AND submitted_by_admin_user_id=?24
                AND reviewed_by_admin_user_id=?25 AND applied_by_admin_user_id=?26
                AND apply_confirmation=?27 AND apply_note=?28 AND applied_at=?29
                AND status=?30 AND idempotency_scope=?31 AND idempotency_key=?32
                AND created_at=?29",
            params![
                stored.envelope.admission_id,
                stored.envelope.schema,
                stored.envelope.admission_digest,
                stored.admission_json,
                stored.envelope.canonicalization,
                stored.envelope.digest_algorithm,
                admission.request_id,
                admission.request_digest,
                admission.request_material_digest,
                admission.review_id,
                admission.review_digest,
                admission.adapter_id,
                admission.release_version,
                admission.route_kind,
                provider_kinds_json,
                admission.candidate_artifact_ref,
                admission.declared_implementation_sha256,
                capabilities_json,
                admission.capability_set_digest,
                verifier.verification_kind,
                verifier.verifier_id,
                verifier.verifier_revision,
                verifier.verifier_digest,
                admission.submitted_by_admin_user_id,
                admission.reviewed_by_admin_user_id,
                admission.applied_by_admin_user_id,
                admission.apply_confirmation,
                admission.apply_note,
                admission.applied_at,
                admission.status,
                stored.idempotency_scope,
                stored.idempotency_key,
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if stored.envelope.schema != EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_SCHEMA
        || stored.envelope.canonicalization
            != COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_CANONICALIZATION
        || stored.envelope.digest_algorithm
            != COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_DIGEST_ALGORITHM
        || admission_json != stored.admission_json
        || admission_digest != stored.envelope.admission_digest
        || provider_kinds_json != stored.supported_provider_kinds_json
        || capabilities_json != stored.capabilities_json
        || capability_set_digest != admission.capability_set_digest
        || admission.request_digest != request.envelope.request_digest
        || admission.request_material_digest != request.envelope.request_material_digest
        || admission.review_id != review.envelope.review_id
        || admission.review_digest != review.envelope.review_digest
        || admission.adapter_id != release.adapter_id
        || admission.release_version != release.release_version
        || admission.route_kind != release.route_kind
        || admission.supported_provider_kinds != release.supported_provider_kinds
        || admission.candidate_artifact_ref != release.candidate_artifact_ref
        || admission.declared_implementation_sha256 != release.declared_implementation_sha256
        || admission.supported_capabilities != release.supported_capabilities
        || admission.capability_set_digest != release.capability_set_digest
        || admission.expected_credential_verifier != release.expected_credential_verifier
        || admission.submitted_by_admin_user_id != request_material.submitted_by_admin_user_id
        || admission.reviewed_by_admin_user_id != review_material.reviewed_by_admin_user_id
        || admission.submitted_by_admin_user_id == admission.reviewed_by_admin_user_id
        || admission.request_digest != review_material.request_digest
        || admission.request_material_digest != review_material.request_material_digest
        || admission.adapter_id != review_material.adapter_id
        || admission.release_version != review_material.release_version
        || review_material.decision != REVIEW_DECISION_APPROVED
        || request.status != ADMISSION_STATUS_STAGED
        || request.reviewed_by_admin_user_id.as_deref()
            != Some(admission.reviewed_by_admin_user_id.as_str())
        || request.applied_by_admin_user_id.as_deref()
            != Some(admission.applied_by_admin_user_id.as_str())
        || request.applied_at.as_deref() != Some(admission.applied_at.as_str())
        || review_material.reviewed_at > admission.applied_at
        || !projected
    {
        bail!("external-pool Adapter release admission failed exact readback audit");
    }
    Ok(stored)
}

fn validate_admission_material(stored: &StoredAdmission) -> Result<()> {
    let envelope = &stored.envelope;
    let admission = &envelope.admission;
    validate_exact(&envelope.admission_id, "stored admission ID", 160)?;
    validate_digest(&envelope.admission_digest, "stored admission digest")?;
    validate_exact(&admission.request_id, "stored admission request ID", 160)?;
    validate_digest(&admission.request_digest, "stored admission request digest")?;
    validate_digest(
        &admission.request_material_digest,
        "stored admission request material digest",
    )?;
    validate_exact(&admission.review_id, "stored admission review ID", 160)?;
    validate_digest(&admission.review_digest, "stored admission review digest")?;
    validate_exact(
        &admission.applied_by_admin_user_id,
        "stored applying administrator",
        160,
    )?;
    validate_exact(
        &stored.idempotency_scope,
        "stored admission idempotency scope",
        200,
    )?;
    validate_exact(
        &stored.idempotency_key,
        "stored admission idempotency key",
        160,
    )?;
    validate_optional_note(&admission.apply_note, "stored admission apply note", 2_000)?;
    canonical_nanos(&admission.applied_at)?;
    if admission.apply_confirmation != EXTERNAL_POOL_ADAPTER_RELEASE_APPLY_CONFIRMATION
        || admission.status != ADMISSION_STATUS_STAGED
    {
        bail!("external-pool Adapter release stored admission authority is invalid");
    }
    Ok(())
}
