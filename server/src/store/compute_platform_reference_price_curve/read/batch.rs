use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::compute_federation::platform_reference_price_curve::{
    canonical_platform_reference_price_curve_batch_json_and_digest,
    canonical_platform_reference_price_curve_batch_material_digest,
    canonical_platform_reference_price_curve_entry_json_and_digest,
    validate_platform_reference_price_curve_batch_envelope,
    validate_platform_reference_price_curve_entry_against_batch,
    ComputePlatformReferencePriceCurveBatchEnvelope,
    COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_BATCH_SCHEMA,
    COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_ENTRY_SCHEMA,
};

use super::super::{
    canonical::canonical_json,
    types::{canonical_nanos, StoredBatch, StoredEntry, BATCH_STATUS_SUBMITTED},
};
use super::decode;

pub(super) fn batch_by_id_on(conn: &Connection, id: &str) -> Result<Option<StoredBatch>> {
    batch_on(conn, "WHERE batch_id=?1", params![id])
}

pub(super) fn batch_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredBatch>> {
    batch_on(
        conn,
        "WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

pub(super) fn batch_by_curve_on(
    conn: &Connection,
    curve_id: &str,
    curve_version: i64,
) -> Result<Option<StoredBatch>> {
    batch_on(
        conn,
        "WHERE curve_id=?1 AND curve_version=?2",
        params![curve_id, curve_version],
    )
}

pub(super) fn entries_by_batch_on(conn: &Connection, batch_id: &str) -> Result<Vec<StoredEntry>> {
    let batch = batch_by_id_on(conn, batch_id)?
        .ok_or_else(|| anyhow::anyhow!("reference price curve entries lost their batch"))?;
    entries_for_batch_on(conn, &batch.envelope)
}

pub(super) fn entry_by_id_on(conn: &Connection, entry_id: &str) -> Result<Option<StoredEntry>> {
    let stored = raw_entry_on(conn, "WHERE entry_id=?1", params![entry_id])?;
    let Some(stored) = stored else {
        return Ok(None);
    };
    let batch = batch_by_id_on(conn, &stored.envelope.batch_id)?
        .ok_or_else(|| anyhow::anyhow!("reference price curve entry lost its batch"))?;
    audit_entry(conn, stored, &batch.envelope).map(Some)
}

fn batch_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredBatch>> {
    let stored = conn
        .query_row(
            &format!(
                "SELECT batch_json, status, reviewed_by_admin_user_id, reviewed_at,
                        applied_by_admin_user_id, applied_at, idempotency_scope,
                        idempotency_key, created_at, updated_at
                   FROM compute_platform_reference_price_curve_batches {filter}"
            ),
            values,
            |row| {
                let batch_json: String = row.get(0)?;
                Ok(StoredBatch {
                    envelope: decode(&batch_json, 0)?,
                    batch_json,
                    status: row.get(1)?,
                    reviewed_by_admin_user_id: row.get(2)?,
                    reviewed_at: row.get(3)?,
                    applied_by_admin_user_id: row.get(4)?,
                    applied_at: row.get(5)?,
                    idempotency_scope: row.get(6)?,
                    idempotency_key: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            },
        )
        .optional()?;
    stored.map(|row| audit_batch(conn, row)).transpose()
}

fn audit_batch(conn: &Connection, stored: StoredBatch) -> Result<StoredBatch> {
    validate_platform_reference_price_curve_batch_envelope(&stored.envelope)?;
    let (batch_json, batch_digest) =
        canonical_platform_reference_price_curve_batch_json_and_digest(&stored.envelope)?;
    let material_digest =
        canonical_platform_reference_price_curve_batch_material_digest(&stored.envelope.batch)?;
    let entries = entries_for_batch_on(conn, &stored.envelope)?;
    let batch = &stored.envelope.batch;
    let projected = conn
        .query_row(
            "SELECT 1 FROM compute_platform_reference_price_curve_batches
              WHERE batch_id=?1 AND batch_schema=?2 AND batch_digest=?3 AND batch_json=?4
                AND canonicalization=?5 AND digest_algorithm=?6 AND batch_material_digest=?7
                AND curve_id=?8 AND curve_version=?9 AND methodology_kind=?10
                AND valid_from=?11 AND valid_until=?12 AND quote_ttl_seconds=?13
                AND rounding_mode=?14 AND entry_count=?15 AND entry_set_digest=?16
                AND confirmation=?17 AND submission_note=?18
                AND submitted_by_admin_user_id=?19 AND submitted_at=?20 AND status=?21
                AND reviewed_by_admin_user_id IS ?22 AND reviewed_at IS ?23
                AND applied_by_admin_user_id IS ?24 AND applied_at IS ?25
                AND idempotency_scope=?26 AND idempotency_key=?27
                AND created_at=?28 AND updated_at=?29",
            params![
                stored.envelope.batch_id,
                stored.envelope.schema,
                stored.envelope.batch_digest,
                stored.batch_json,
                stored.envelope.canonicalization,
                stored.envelope.digest_algorithm,
                stored.envelope.batch_material_digest,
                batch.curve_id,
                batch.curve_version,
                batch.methodology_kind,
                batch.valid_from,
                batch.valid_until,
                batch.quote_ttl_seconds,
                batch.rounding_mode,
                i64::try_from(batch.entries.len())?,
                batch.entry_set_digest,
                batch.confirmation,
                batch.submission_note,
                batch.submitted_by_admin_user_id,
                batch.submitted_at,
                stored.status,
                stored.reviewed_by_admin_user_id,
                stored.reviewed_at,
                stored.applied_by_admin_user_id,
                stored.applied_at,
                stored.idempotency_scope,
                stored.idempotency_key,
                stored.created_at,
                stored.updated_at,
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if stored.envelope.schema != COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_BATCH_SCHEMA
        || batch_json != stored.batch_json
        || batch_digest != stored.envelope.batch_digest
        || material_digest != stored.envelope.batch_material_digest
        || stored.idempotency_key != batch.idempotency_key
        || entries.len() != batch.entries.len()
        || entries
            .iter()
            .zip(&batch.entries)
            .any(|(stored_entry, intent)| &stored_entry.envelope.entry != intent)
        || !state_is_exact(&stored)
        || !projected
    {
        bail!("platform reference price curve batch failed exact readback audit");
    }
    Ok(stored)
}

fn entries_for_batch_on(
    conn: &Connection,
    batch: &ComputePlatformReferencePriceCurveBatchEnvelope,
) -> Result<Vec<StoredEntry>> {
    let mut statement = conn.prepare(
        "SELECT entry_json, components_json, fee_rules_json
           FROM compute_platform_reference_price_curve_entries
          WHERE batch_id=?1 ORDER BY ordinal ASC, entry_id ASC",
    )?;
    let rows = statement
        .query_map(params![batch.batch_id], |row| {
            let entry_json: String = row.get(0)?;
            Ok(StoredEntry {
                envelope: decode(&entry_json, 0)?,
                entry_json,
                components_json: row.get(1)?,
                fee_rules_json: row.get(2)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    rows.into_iter()
        .map(|entry| audit_entry(conn, entry, batch))
        .collect()
}

fn raw_entry_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredEntry>> {
    conn.query_row(
        &format!(
            "SELECT entry_json, components_json, fee_rules_json
               FROM compute_platform_reference_price_curve_entries {filter}"
        ),
        values,
        |row| {
            let entry_json: String = row.get(0)?;
            Ok(StoredEntry {
                envelope: decode(&entry_json, 0)?,
                entry_json,
                components_json: row.get(1)?,
                fee_rules_json: row.get(2)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

fn audit_entry(
    conn: &Connection,
    stored: StoredEntry,
    batch: &ComputePlatformReferencePriceCurveBatchEnvelope,
) -> Result<StoredEntry> {
    validate_platform_reference_price_curve_entry_against_batch(&stored.envelope, batch)?;
    let (entry_json, entry_digest) =
        canonical_platform_reference_price_curve_entry_json_and_digest(&stored.envelope)?;
    let entry = &stored.envelope.entry;
    let components_json = canonical_json(&entry.components)?;
    let fee_rules_json = canonical_json(&entry.fee_rules)?;
    let projected = conn
        .query_row(
            "SELECT 1 FROM compute_platform_reference_price_curve_entries
              WHERE entry_id=?1 AND entry_schema=?2 AND entry_digest=?3 AND entry_json=?4
                AND batch_id=?5 AND batch_digest=?6 AND ordinal=?7 AND entry_key=?8
                AND provider_id=?9 AND offer_id=?10 AND offer_version=?11 AND offer_digest=?12
                AND sku_id=?13 AND sku_digest=?14 AND delivery_window_id=?15
                AND delivery_window_digest=?16 AND pricing_mode=?17 AND currency=?18
                AND offer_curve_id IS ?19 AND offer_curve_version IS ?20
                AND instrument_id IS ?21 AND components_json=?22 AND fee_rules_json=?23
                AND consumer_max_amount_micros=?24 AND provider_max_amount_micros=?25
                AND created_at=?26",
            params![
                stored.envelope.entry_id,
                stored.envelope.schema,
                stored.envelope.entry_digest,
                stored.entry_json,
                stored.envelope.batch_id,
                stored.envelope.batch_digest,
                stored.envelope.ordinal,
                entry.entry_key,
                entry.provider_id,
                entry.offer_id,
                entry.offer_version,
                entry.offer_digest,
                entry.sku_id,
                entry.sku_digest,
                entry.delivery_window_id,
                entry.delivery_window_digest,
                entry.pricing_mode,
                entry.currency,
                entry.offer_curve_id,
                entry.offer_curve_version,
                entry.instrument_id,
                components_json,
                fee_rules_json,
                entry.consumer_max_amount_micros,
                entry.provider_max_amount_micros,
                batch.batch.submitted_at,
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if stored.envelope.schema != COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_ENTRY_SCHEMA
        || entry_json != stored.entry_json
        || entry_digest != stored.envelope.entry_digest
        || components_json != stored.components_json
        || fee_rules_json != stored.fee_rules_json
        || !projected
    {
        bail!("platform reference price curve entry failed exact readback audit");
    }
    Ok(stored)
}

fn state_is_exact(stored: &StoredBatch) -> bool {
    let submitted = &stored.envelope.batch.submitted_at;
    if canonical_nanos(submitted).is_err() || stored.created_at != *submitted {
        return false;
    }
    let reviewed_by_other = stored
        .reviewed_by_admin_user_id
        .as_deref()
        .is_some_and(|reviewer| reviewer != stored.envelope.batch.submitted_by_admin_user_id);
    match stored.status.as_str() {
        BATCH_STATUS_SUBMITTED => {
            stored.reviewed_by_admin_user_id.is_none()
                && stored.reviewed_at.is_none()
                && stored.applied_by_admin_user_id.is_none()
                && stored.applied_at.is_none()
                && stored.updated_at == *submitted
        }
        "approved" | "changes_requested" | "rejected" => {
            stored.reviewed_at.as_ref().is_some_and(|reviewed| {
                canonical_nanos(reviewed).is_ok()
                    && submitted <= reviewed
                    && reviewed_by_other
                    && stored.applied_by_admin_user_id.is_none()
                    && stored.applied_at.is_none()
                    && stored.updated_at == *reviewed
            })
        }
        "applied" => match (&stored.reviewed_at, &stored.applied_at) {
            (Some(reviewed), Some(applied)) => {
                canonical_nanos(reviewed).is_ok()
                    && canonical_nanos(applied).is_ok()
                    && submitted <= reviewed
                    && reviewed <= applied
                    && reviewed_by_other
                    && stored.applied_by_admin_user_id.is_some()
                    && stored.updated_at == *applied
            }
            _ => false,
        },
        _ => false,
    }
}
