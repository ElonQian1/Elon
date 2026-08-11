use super::super::{
    canonical::{
        canonical_application_json_and_digest, canonical_json,
        canonical_snapshot_binding_json_and_digest, canonical_snapshot_binding_set_digest,
    },
    types::{
        ComputePlatformReferencePriceCurveSnapshotBindingReceipt, StoredApplication,
        StoredSnapshotBinding, APPLICATION_STATUS_APPLIED,
        PLATFORM_REFERENCE_PRICE_CURVE_APPLICATION_SCHEMA,
        PLATFORM_REFERENCE_PRICE_CURVE_SNAPSHOT_BINDING_SCHEMA, REVIEW_DECISION_APPROVED,
    },
};
use super::{batch_by_id_on, decode, entry_by_id_on, review_by_batch_on};
use crate::{
    compute_federation::platform_reference_price_curve::{
        COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_CANONICALIZATION,
        COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_DIGEST_ALGORITHM,
    },
    store::compute_price_snapshot_registry::registered_price_snapshot_on,
};
use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension};

mod validation;
use validation::{snapshot_matches, validate_application_material, validate_binding_material};
pub(in crate::store::compute_platform_reference_price_curve) fn application_by_batch_on(
    conn: &Connection,
    batch_id: &str,
) -> Result<Option<StoredApplication>> {
    application_on(conn, "WHERE batch_id=?1", params![batch_id])
}
pub(in crate::store::compute_platform_reference_price_curve) fn application_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredApplication>> {
    application_on(
        conn,
        "WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}
pub(in crate::store::compute_platform_reference_price_curve) fn bindings_by_application_on(
    conn: &Connection,
    application_id: &str,
) -> Result<Vec<StoredSnapshotBinding>> {
    let application = application_on(conn, "WHERE application_id=?1", params![application_id])?
        .ok_or_else(|| anyhow::anyhow!("reference price curve bindings lost their application"))?;
    bindings_for_application_on(conn, &application)
}
pub(in crate::store::compute_platform_reference_price_curve) fn snapshot_binding_by_snapshot_on(
    conn: &Connection,
    snapshot_id: &str,
) -> Result<Option<ComputePlatformReferencePriceCurveSnapshotBindingReceipt>> {
    let stored = conn
        .query_row(
            "SELECT binding_json
               FROM compute_platform_reference_price_curve_snapshot_bindings
              WHERE snapshot_id=?1",
            params![snapshot_id],
            |row| {
                let binding_json: String = row.get(0)?;
                Ok(StoredSnapshotBinding {
                    envelope: decode(&binding_json, 0)?,
                    binding_json,
                })
            },
        )
        .optional()?;
    let Some(stored) = stored else {
        return Ok(None);
    };
    let application = application_on(
        conn,
        "WHERE application_id=?1",
        params![stored.envelope.binding.application_id.as_str()],
    )?
    .ok_or_else(|| anyhow::anyhow!("reference price curve binding lost its application"))?;
    audit_binding(conn, stored, &application).map(|binding| Some(binding.into_receipt()))
}
fn application_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredApplication>> {
    raw_application_on(conn, filter, values)?
        .map(|row| audit_application(conn, row))
        .transpose()
}
fn raw_application_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredApplication>> {
    conn.query_row(
        &format!(
            "SELECT application_json, binding_digests_json, idempotency_scope, idempotency_key
               FROM compute_platform_reference_price_curve_applications {filter}"
        ),
        values,
        |row| {
            let application_json: String = row.get(0)?;
            Ok(StoredApplication {
                envelope: decode(&application_json, 0)?,
                application_json,
                binding_digests_json: row.get(1)?,
                idempotency_scope: row.get(2)?,
                idempotency_key: row.get(3)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}
fn audit_application(conn: &Connection, stored: StoredApplication) -> Result<StoredApplication> {
    validate_application_material(&stored)?;
    let (application_json, application_digest) =
        canonical_application_json_and_digest(&stored.envelope)?;
    let application = &stored.envelope.application;
    let binding_digests_json = canonical_json(&application.binding_digests)?;
    let binding_set_digest = canonical_snapshot_binding_set_digest(&application.binding_digests)?;
    let batch = batch_by_id_on(conn, &application.batch_id)?
        .ok_or_else(|| anyhow::anyhow!("reference price curve application lost its batch"))?;
    let review = review_by_batch_on(conn, &application.batch_id)?
        .ok_or_else(|| anyhow::anyhow!("reference price curve application lost its review"))?;
    let bindings = bindings_for_application_on(conn, &stored)?;
    let projected = conn
        .query_row(
            "SELECT 1 FROM compute_platform_reference_price_curve_applications
              WHERE application_id=?1 AND application_schema=?2 AND application_digest=?3
                AND application_json=?4 AND canonicalization=?5 AND digest_algorithm=?6
                AND batch_id=?7 AND batch_digest=?8 AND batch_material_digest=?9
                AND review_id=?10 AND review_digest=?11 AND curve_id=?12 AND curve_version=?13
                AND binding_digests_json=?14 AND binding_count=?15 AND binding_set_digest=?16
                AND submitted_by_admin_user_id=?17 AND reviewed_by_admin_user_id=?18
                AND applied_by_admin_user_id=?19 AND apply_confirmation=?20 AND apply_note=?21
                AND applied_at=?22 AND status=?23 AND idempotency_scope=?24
                AND idempotency_key=?25 AND created_at=?22",
            params![
                stored.envelope.application_id,
                stored.envelope.schema,
                stored.envelope.application_digest,
                stored.application_json,
                stored.envelope.canonicalization,
                stored.envelope.digest_algorithm,
                application.batch_id,
                application.batch_digest,
                application.batch_material_digest,
                application.review_id,
                application.review_digest,
                application.curve_id,
                application.curve_version,
                binding_digests_json,
                i64::try_from(application.binding_digests.len())?,
                application.binding_set_digest,
                application.submitted_by_admin_user_id,
                application.reviewed_by_admin_user_id,
                application.applied_by_admin_user_id,
                application.apply_confirmation,
                application.apply_note,
                application.applied_at,
                application.status,
                stored.idempotency_scope,
                stored.idempotency_key,
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    let batch_material = &batch.envelope.batch;
    let review_material = &review.envelope.review;
    if stored.envelope.schema != PLATFORM_REFERENCE_PRICE_CURVE_APPLICATION_SCHEMA
        || stored.envelope.canonicalization
            != COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_CANONICALIZATION
        || stored.envelope.digest_algorithm
            != COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_DIGEST_ALGORITHM
        || application_json != stored.application_json
        || application_digest != stored.envelope.application_digest
        || binding_digests_json != stored.binding_digests_json
        || binding_set_digest != application.binding_set_digest
        || !bindings
            .iter()
            .map(|binding| binding.envelope.binding_digest.as_str())
            .eq(application.binding_digests.iter().map(String::as_str))
        || bindings.len() != batch_material.entries.len()
        || application.batch_digest != batch.envelope.batch_digest
        || application.batch_material_digest != batch.envelope.batch_material_digest
        || application.review_id != review.envelope.review_id
        || application.review_digest != review.envelope.review_digest
        || application.curve_id != batch_material.curve_id
        || application.curve_version != batch_material.curve_version
        || application.submitted_by_admin_user_id != batch_material.submitted_by_admin_user_id
        || application.reviewed_by_admin_user_id != review_material.reviewed_by_admin_user_id
        || review_material.decision != REVIEW_DECISION_APPROVED
        || application.reviewed_by_admin_user_id == application.submitted_by_admin_user_id
        || review_material.reviewed_at > application.applied_at
        || batch.status != APPLICATION_STATUS_APPLIED
        || batch.applied_by_admin_user_id.as_deref()
            != Some(application.applied_by_admin_user_id.as_str())
        || batch.applied_at.as_deref() != Some(application.applied_at.as_str())
        || !projected
    {
        bail!("platform reference price curve application failed exact readback audit");
    }
    Ok(stored)
}
fn bindings_for_application_on(
    conn: &Connection,
    application: &StoredApplication,
) -> Result<Vec<StoredSnapshotBinding>> {
    let mut statement = conn.prepare(
        "SELECT binding_json FROM compute_platform_reference_price_curve_snapshot_bindings
          WHERE application_id=?1 ORDER BY ordinal ASC, binding_id ASC",
    )?;
    let rows = statement
        .query_map(params![application.envelope.application_id], |row| {
            let binding_json: String = row.get(0)?;
            Ok(StoredSnapshotBinding {
                envelope: decode(&binding_json, 0)?,
                binding_json,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    rows.into_iter()
        .map(|binding| audit_binding(conn, binding, application))
        .collect()
}
fn audit_binding(
    conn: &Connection,
    stored: StoredSnapshotBinding,
    application: &StoredApplication,
) -> Result<StoredSnapshotBinding> {
    validate_binding_material(&stored)?;
    let (binding_json, binding_digest) =
        canonical_snapshot_binding_json_and_digest(&stored.envelope)?;
    let binding = &stored.envelope.binding;
    let app = &application.envelope.application;
    let entry = entry_by_id_on(conn, &binding.entry_id)?
        .ok_or_else(|| anyhow::anyhow!("reference price curve binding lost its entry"))?;
    let batch = batch_by_id_on(conn, &binding.batch_id)?
        .ok_or_else(|| anyhow::anyhow!("reference price curve binding lost its batch"))?;
    let snapshot = registered_price_snapshot_on(conn, &binding.snapshot_id)?
        .ok_or_else(|| anyhow::anyhow!("reference price curve binding lost its v171 Snapshot"))?;
    let projected = conn
        .query_row(
            "SELECT 1 FROM compute_platform_reference_price_curve_snapshot_bindings
              WHERE binding_id=?1 AND binding_schema=?2 AND binding_digest=?3 AND binding_json=?4
                AND canonicalization=?5 AND digest_algorithm=?6 AND application_id=?7
                AND batch_id=?8 AND batch_digest=?9 AND review_id=?10 AND review_digest=?11
                AND entry_id=?12 AND entry_digest=?13 AND ordinal=?14 AND entry_key=?15
                AND curve_id=?16 AND curve_version=?17 AND snapshot_id=?18
                AND snapshot_digest=?19 AND quote_id=?20 AND source_kind=?21
                AND source_id=?22 AND source_version=?23 AND source_digest=?24
                AND quoted_at=?25 AND expires_at=?26 AND status=?27 AND created_at=?25",
            params![
                stored.envelope.binding_id,
                stored.envelope.schema,
                stored.envelope.binding_digest,
                stored.binding_json,
                stored.envelope.canonicalization,
                stored.envelope.digest_algorithm,
                binding.application_id,
                binding.batch_id,
                binding.batch_digest,
                binding.review_id,
                binding.review_digest,
                binding.entry_id,
                binding.entry_digest,
                binding.ordinal,
                binding.entry_key,
                binding.curve_id,
                binding.curve_version,
                binding.snapshot_id,
                binding.snapshot_digest,
                binding.quote_id,
                binding.source_kind,
                binding.source_id,
                binding.source_version,
                binding.source_digest,
                binding.quoted_at,
                binding.expires_at,
                binding.status,
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if stored.envelope.schema != PLATFORM_REFERENCE_PRICE_CURVE_SNAPSHOT_BINDING_SCHEMA
        || stored.envelope.canonicalization
            != COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_CANONICALIZATION
        || stored.envelope.digest_algorithm
            != COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_DIGEST_ALGORITHM
        || binding_json != stored.binding_json
        || binding_digest != stored.envelope.binding_digest
        || binding.application_id != application.envelope.application_id
        || binding.batch_id != app.batch_id
        || binding.batch_digest != app.batch_digest
        || binding.review_id != app.review_id
        || binding.review_digest != app.review_digest
        || binding.entry_id != entry.envelope.entry_id
        || binding.entry_digest != entry.envelope.entry_digest
        || entry.envelope.batch_id != binding.batch_id
        || entry.envelope.batch_digest != binding.batch_digest
        || binding.ordinal != entry.envelope.ordinal
        || binding.entry_key != entry.envelope.entry.entry_key
        || binding.curve_id != app.curve_id
        || binding.curve_version != app.curve_version
        || !snapshot_matches(&snapshot, binding, &entry.envelope.entry, &batch.envelope)
        || !projected
    {
        bail!("platform reference price curve Snapshot binding failed exact readback audit");
    }
    Ok(stored)
}
