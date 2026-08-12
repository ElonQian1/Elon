use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat};
use rusqlite::{params, types::Type, Connection, OptionalExtension};

use crate::store::{
    compute_external_pool_adapter_release::admission_by_id_on,
    compute_external_pool_adapter_release_lifecycle::current_external_pool_adapter_release_admission_authority_on,
    Store,
};

use super::{
    canonical::{canonical_intake_material_digest, canonical_receipt_json_and_digest},
    types::{
        ExternalPoolAdapterArtifactIntakeAuthority, ExternalPoolAdapterArtifactSourceAuthority,
        ExternalPoolAdapterArtifactSourceReceipt, StoredArtifactSourceEnvelope,
        StoredArtifactSourceReceipt, EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_CANONICALIZATION,
        EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_CUSTODY_STATE,
        EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_DIGEST_ALGORITHM,
        EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_EVIDENCE_SCOPE,
        EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_INTAKE_CONFIRMATION,
        EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_INTAKE_KIND,
        EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_NO_EFFECT,
        EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_RECEIPT_SCHEMA,
        EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_STORAGE_NAMESPACE,
        EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_STORAGE_ROOT_KIND,
        MAX_EXTERNAL_POOL_ADAPTER_ARTIFACT_SIZE_BYTES,
    },
};

pub(super) fn receipt_by_admission_on(
    conn: &Connection,
    admission_id: &str,
) -> Result<Option<StoredArtifactSourceReceipt>> {
    receipt_on(conn, "WHERE admission_id=?1", params![admission_id])
}

pub(in crate::store) fn external_pool_adapter_artifact_source_authority_on(
    conn: &Connection,
    admission_id: &str,
    expected_admission_digest: &str,
    expected_source_receipt_digest: &str,
) -> Result<Option<ExternalPoolAdapterArtifactSourceAuthority>> {
    let Some(stored) = receipt_by_admission_on(conn, admission_id)? else {
        return Ok(None);
    };
    if stored.envelope.source.admission_digest != expected_admission_digest
        || stored.envelope.source_receipt_digest != expected_source_receipt_digest
    {
        bail!("artifact source authority is not exact");
    }
    Ok(Some(ExternalPoolAdapterArtifactSourceAuthority::new(
        &stored,
    )))
}

pub(super) fn receipt_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredArtifactSourceReceipt>> {
    receipt_on(
        conn,
        "WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

fn receipt_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredArtifactSourceReceipt>> {
    let stored = conn
        .query_row(
            &format!(
                "SELECT source_receipt_json
                   FROM compute_external_pool_adapter_artifact_source_receipts {filter}"
            ),
            values,
            |row| {
                let source_receipt_json: String = row.get(0)?;
                let envelope = serde_json::from_str(&source_receipt_json).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error))
                })?;
                Ok(StoredArtifactSourceReceipt {
                    envelope,
                    source_receipt_json,
                })
            },
        )
        .optional()?;
    stored.map(|row| audit_receipt(conn, row)).transpose()
}

fn audit_receipt(
    conn: &Connection,
    stored: StoredArtifactSourceReceipt,
) -> Result<StoredArtifactSourceReceipt> {
    validate_stored_receipt(&stored)?;
    let (receipt_json, receipt_digest) = canonical_receipt_json_and_digest(&stored.envelope)?;
    let material_digest = canonical_intake_material_digest(&stored.envelope.source)?;
    let source = &stored.envelope.source;
    let admission = admission_by_id_on(conn, &source.admission_id)?.ok_or_else(|| {
        anyhow::anyhow!("external-pool Adapter artifact source lost its staged admission")
    })?;
    let projected = conn
        .query_row(
            "SELECT 1 FROM compute_external_pool_adapter_artifact_source_receipts
              WHERE source_receipt_id=?1 AND source_receipt_schema=?2
                AND source_receipt_digest=?3 AND source_receipt_json=?4
                AND canonicalization=?5 AND digest_algorithm=?6
                AND admission_id=?7 AND admission_digest=?8
                AND request_id=?9 AND request_digest=?10
                AND request_material_digest=?11 AND review_id=?12
                AND review_digest=?13 AND adapter_id=?14 AND release_version=?15
                AND candidate_artifact_ref=?16 AND declared_implementation_sha256=?17
                AND intake_sha256=?18 AND reopened_sha256=?19
                AND artifact_size_bytes=?20 AND storage_root_kind=?21
                AND storage_namespace=?22 AND content_address_algorithm=?23
                AND content_address_digest=?24 AND custody_state=?25
                AND intake_kind=?26 AND evidence_scope=?27
                AND artifact_ref_resolution_effect=?28 AND adapter_effect=?29
                AND route_effect=?30 AND recorded_by_admin_user_id=?31
                AND intake_confirmation=?32 AND recorded_at=?33
                AND intake_material_digest=?34 AND idempotency_scope=?35
                AND idempotency_key=?36 AND created_at=?37",
            params![
                stored.envelope.source_receipt_id,
                stored.envelope.schema,
                stored.envelope.source_receipt_digest,
                stored.source_receipt_json,
                stored.envelope.canonicalization,
                stored.envelope.digest_algorithm,
                source.admission_id,
                source.admission_digest,
                source.request_id,
                source.request_digest,
                source.request_material_digest,
                source.review_id,
                source.review_digest,
                source.adapter_id,
                source.release_version,
                source.candidate_artifact_ref,
                source.declared_implementation_sha256,
                source.intake_sha256,
                source.reopened_sha256,
                source.artifact_size_bytes,
                source.storage_root_kind,
                source.storage_namespace,
                source.content_address_algorithm,
                source.content_address_digest,
                source.custody_state,
                source.intake_kind,
                source.evidence_scope,
                source.artifact_ref_resolution_effect,
                source.adapter_effect,
                source.route_effect,
                source.recorded_by_admin_user_id,
                source.intake_confirmation,
                source.recorded_at,
                stored.envelope.intake_material_digest,
                source.idempotency_scope,
                source.idempotency_key,
                source.created_at,
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if stored.envelope.schema != EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_RECEIPT_SCHEMA
        || stored.envelope.canonicalization
            != EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_CANONICALIZATION
        || stored.envelope.digest_algorithm
            != EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_DIGEST_ALGORITHM
        || receipt_json != stored.source_receipt_json
        || receipt_digest != stored.envelope.source_receipt_digest
        || material_digest != stored.envelope.intake_material_digest
        || source.admission_id != admission.admission_id
        || source.admission_digest != admission.admission_digest
        || source.request_id != admission.request_id
        || source.request_digest != admission.request_digest
        || source.request_material_digest != admission.request_material_digest
        || source.review_id != admission.review_id
        || source.review_digest != admission.review_digest
        || source.adapter_id != admission.adapter_id
        || source.release_version != admission.release_version
        || source.candidate_artifact_ref != admission.candidate_artifact_ref
        || source.declared_implementation_sha256 != admission.declared_implementation_sha256
        || admission.status != "staged"
        || !projected
    {
        bail!("external-pool Adapter artifact source failed exact readback audit");
    }
    Ok(stored)
}

fn validate_stored_receipt(stored: &StoredArtifactSourceReceipt) -> Result<()> {
    let envelope = &stored.envelope;
    let source = &envelope.source;
    for (value, label, max) in [
        (&envelope.source_receipt_id, "stored source receipt ID", 160),
        (&source.admission_id, "stored source admission ID", 160),
        (&source.request_id, "stored source request ID", 160),
        (&source.review_id, "stored source review ID", 160),
        (&source.adapter_id, "stored source Adapter ID", 160),
        (&source.release_version, "stored source release version", 80),
        (
            &source.recorded_by_admin_user_id,
            "stored source administrator",
            160,
        ),
        (
            &source.idempotency_scope,
            "stored source idempotency scope",
            200,
        ),
        (
            &source.idempotency_key,
            "stored source idempotency key",
            160,
        ),
    ] {
        validate_exact(value, label, max)?;
    }
    for (value, label) in [
        (
            &envelope.source_receipt_digest,
            "stored source receipt digest",
        ),
        (
            &envelope.intake_material_digest,
            "stored intake material digest",
        ),
        (&source.admission_digest, "stored source admission digest"),
        (&source.request_digest, "stored source request digest"),
        (
            &source.request_material_digest,
            "stored source request material digest",
        ),
        (&source.review_digest, "stored source review digest"),
        (
            &source.declared_implementation_sha256,
            "stored source declared digest",
        ),
        (&source.intake_sha256, "stored source intake digest"),
        (&source.reopened_sha256, "stored source reopened digest"),
        (
            &source.content_address_digest,
            "stored source content address digest",
        ),
    ] {
        validate_digest(value, label)?;
    }
    canonical_nanos(&source.recorded_at)?;
    if source.candidate_artifact_ref.len() < 14
        || source.candidate_artifact_ref.len() > 173
        || !source.candidate_artifact_ref.starts_with("artifact-ref:")
        || source.artifact_size_bytes < 1
        || source.artifact_size_bytes > MAX_EXTERNAL_POOL_ADAPTER_ARTIFACT_SIZE_BYTES as i64
        || source.declared_implementation_sha256 != source.intake_sha256
        || source.intake_sha256 != source.reopened_sha256
        || source.reopened_sha256 != source.content_address_digest
        || source.storage_root_kind != EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_STORAGE_ROOT_KIND
        || source.storage_namespace != EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_STORAGE_NAMESPACE
        || source.content_address_algorithm
            != EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_DIGEST_ALGORITHM
        || source.custody_state != EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_CUSTODY_STATE
        || source.intake_kind != EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_INTAKE_KIND
        || source.evidence_scope != EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_EVIDENCE_SCOPE
        || source.artifact_ref_resolution_effect != EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_NO_EFFECT
        || source.adapter_effect != EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_NO_EFFECT
        || source.route_effect != EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_NO_EFFECT
        || source.intake_confirmation != EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_INTAKE_CONFIRMATION
        || source.created_at != source.recorded_at
    {
        bail!("external-pool Adapter artifact source stored material is invalid");
    }
    Ok(())
}

impl Store {
    pub(crate) fn external_pool_adapter_artifact_intake_authority(
        &self,
        admission_id: &str,
        expected_admission_digest: &str,
    ) -> Result<Option<ExternalPoolAdapterArtifactIntakeAuthority>> {
        validate_exact(admission_id, "artifact intake admission ID", 160)?;
        validate_digest(
            expected_admission_digest,
            "artifact intake expected admission digest",
        )?;
        let mut connection = self.conn()?;
        let transaction = connection.transaction()?;
        let authority = current_external_pool_adapter_release_admission_authority_on(
            &transaction,
            admission_id,
            expected_admission_digest,
        )?
        .map(|current| {
            ExternalPoolAdapterArtifactIntakeAuthority::new(
                current.admission_id().to_string(),
                current.admission_digest().to_string(),
                current.declared_implementation_sha256().to_string(),
            )
        });
        transaction.commit()?;
        Ok(authority)
    }

    pub(crate) fn external_pool_adapter_artifact_source_for_admission(
        &self,
        admission_id: &str,
    ) -> Result<Option<ExternalPoolAdapterArtifactSourceReceipt>> {
        validate_exact(admission_id, "artifact source admission ID", 160)?;
        let connection = self.conn()?;
        receipt_by_admission_on(&connection, admission_id)
            .map(|stored| stored.map(|value| value.into_receipt(false)))
    }
}

pub(super) fn validate_exact(value: &str, label: &str, max: usize) -> Result<()> {
    if value.is_empty() || value.len() > max || value.trim() != value {
        bail!("{label} must be non-empty, bounded, and exact");
    }
    Ok(())
}

pub(super) fn validate_digest(value: &str, label: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
    {
        bail!("{label} must be a lowercase SHA-256 digest");
    }
    Ok(())
}

fn canonical_nanos(value: &str) -> Result<()> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("artifact source timestamp is not canonical UTC nanoseconds");
    }
    Ok(())
}
