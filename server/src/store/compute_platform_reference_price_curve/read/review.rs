use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::compute_federation::platform_reference_price_curve::{
    COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_CANONICALIZATION,
    COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_DIGEST_ALGORITHM,
};

use super::super::{
    canonical::canonical_review_json_and_digest,
    review::{validate_digest, validate_exact, validate_optional_note},
    types::{
        canonical_nanos, StoredReview, PLATFORM_REFERENCE_PRICE_CURVE_REVIEW_CONFIRMATION,
        PLATFORM_REFERENCE_PRICE_CURVE_REVIEW_SCHEMA, REVIEW_DECISION_APPROVED,
        REVIEW_DECISION_CHANGES_REQUESTED, REVIEW_DECISION_REJECTED,
    },
};
use super::{batch_by_id_on, decode};

pub(super) fn review_by_batch_on(
    conn: &Connection,
    batch_id: &str,
) -> Result<Option<StoredReview>> {
    review_on(conn, "WHERE batch_id=?1", params![batch_id])
}

pub(super) fn review_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredReview>> {
    review_on(
        conn,
        "WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

fn review_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredReview>> {
    let stored = conn
        .query_row(
            &format!(
                "SELECT review_json, idempotency_scope, idempotency_key
                   FROM compute_platform_reference_price_curve_reviews {filter}"
            ),
            values,
            |row| {
                let review_json: String = row.get(0)?;
                Ok(StoredReview {
                    envelope: decode(&review_json, 0)?,
                    review_json,
                    idempotency_scope: row.get(1)?,
                    idempotency_key: row.get(2)?,
                })
            },
        )
        .optional()?;
    stored.map(|row| audit_review(conn, row)).transpose()
}

fn audit_review(conn: &Connection, stored: StoredReview) -> Result<StoredReview> {
    validate_review_material(&stored)?;
    let (review_json, review_digest) = canonical_review_json_and_digest(&stored.envelope)?;
    let review = &stored.envelope.review;
    let batch = batch_by_id_on(conn, &review.batch_id)?
        .ok_or_else(|| anyhow::anyhow!("reference price curve review lost its batch"))?;
    let batch_material = &batch.envelope.batch;
    let projected = conn
        .query_row(
            "SELECT 1 FROM compute_platform_reference_price_curve_reviews
              WHERE review_id=?1 AND review_schema=?2 AND review_digest=?3 AND review_json=?4
                AND canonicalization=?5 AND digest_algorithm=?6 AND batch_id=?7
                AND batch_digest=?8 AND batch_material_digest=?9 AND curve_id=?10
                AND curve_version=?11 AND entry_set_digest=?12 AND decision=?13
                AND review_confirmation=?14 AND review_note IS ?15
                AND reviewed_by_admin_user_id=?16 AND reviewed_at=?17
                AND idempotency_scope=?18 AND idempotency_key=?19 AND created_at=?17",
            params![
                stored.envelope.review_id,
                stored.envelope.schema,
                stored.envelope.review_digest,
                stored.review_json,
                stored.envelope.canonicalization,
                stored.envelope.digest_algorithm,
                review.batch_id,
                review.batch_digest,
                review.batch_material_digest,
                review.curve_id,
                review.curve_version,
                review.entry_set_digest,
                review.decision,
                review.review_confirmation,
                review.review_note,
                review.reviewed_by_admin_user_id,
                review.reviewed_at,
                stored.idempotency_scope,
                stored.idempotency_key,
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    let status_matches = match review.decision.as_str() {
        REVIEW_DECISION_APPROVED => matches!(batch.status.as_str(), "approved" | "applied"),
        REVIEW_DECISION_CHANGES_REQUESTED | REVIEW_DECISION_REJECTED => {
            batch.status == review.decision
        }
        _ => false,
    };
    if stored.envelope.schema != PLATFORM_REFERENCE_PRICE_CURVE_REVIEW_SCHEMA
        || stored.envelope.canonicalization
            != COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_CANONICALIZATION
        || stored.envelope.digest_algorithm
            != COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_DIGEST_ALGORITHM
        || review_json != stored.review_json
        || review_digest != stored.envelope.review_digest
        || review.batch_digest != batch.envelope.batch_digest
        || review.batch_material_digest != batch.envelope.batch_material_digest
        || review.curve_id != batch_material.curve_id
        || review.curve_version != batch_material.curve_version
        || review.entry_set_digest != batch_material.entry_set_digest
        || review.reviewed_by_admin_user_id == batch_material.submitted_by_admin_user_id
        || batch.reviewed_by_admin_user_id.as_deref()
            != Some(review.reviewed_by_admin_user_id.as_str())
        || batch.reviewed_at.as_deref() != Some(review.reviewed_at.as_str())
        || batch_material.submitted_at > review.reviewed_at
        || !status_matches
        || !projected
    {
        bail!("platform reference price curve review failed exact readback audit");
    }
    Ok(stored)
}

fn validate_review_material(stored: &StoredReview) -> Result<()> {
    let envelope = &stored.envelope;
    let review = &envelope.review;
    validate_exact(&envelope.review_id, "stored review ID", 160)?;
    validate_digest(&envelope.review_digest, "stored review digest")?;
    validate_exact(&review.batch_id, "stored review batch ID", 160)?;
    validate_digest(&review.batch_digest, "stored review batch digest")?;
    validate_digest(
        &review.batch_material_digest,
        "stored review batch material digest",
    )?;
    validate_exact(&review.curve_id, "stored review curve ID", 160)?;
    validate_digest(&review.entry_set_digest, "stored review entry-set digest")?;
    validate_exact(
        &review.reviewed_by_admin_user_id,
        "stored reviewing administrator",
        160,
    )?;
    validate_exact(
        &stored.idempotency_scope,
        "stored review idempotency scope",
        200,
    )?;
    validate_exact(
        &stored.idempotency_key,
        "stored review idempotency key",
        160,
    )?;
    canonical_nanos(&review.reviewed_at)?;
    if review.curve_version <= 0
        || review.review_confirmation != PLATFORM_REFERENCE_PRICE_CURVE_REVIEW_CONFIRMATION
        || !matches!(
            review.decision.as_str(),
            REVIEW_DECISION_APPROVED | REVIEW_DECISION_CHANGES_REQUESTED | REVIEW_DECISION_REJECTED
        )
    {
        bail!("platform reference price curve stored review authority is invalid");
    }
    if let Some(note) = &review.review_note {
        validate_optional_note(note, "stored review note", 2_000)?;
        if note.is_empty() {
            bail!("platform reference price curve stored review note is empty");
        }
    }
    if review.decision != REVIEW_DECISION_APPROVED && review.review_note.is_none() {
        bail!("platform reference price curve stored non-approval lacks a note");
    }
    Ok(())
}
