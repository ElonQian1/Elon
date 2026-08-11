use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::compute_federation::external_pool_adapter_release::{
    COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_CANONICALIZATION,
    COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_DIGEST_ALGORITHM,
};

use super::super::{
    canonical::canonical_review_json_and_digest,
    review::{validate_digest, validate_exact, validate_optional_note},
    types::{
        canonical_nanos, StoredReview, EXTERNAL_POOL_ADAPTER_RELEASE_REVIEW_CONFIRMATION,
        EXTERNAL_POOL_ADAPTER_RELEASE_REVIEW_SCHEMA, REVIEW_DECISION_APPROVED,
        REVIEW_DECISION_CHANGES_REQUESTED, REVIEW_DECISION_REJECTED,
    },
};
use super::{decode, request_by_id_on};

pub(super) fn review_by_request_on(
    conn: &Connection,
    request_id: &str,
) -> Result<Option<StoredReview>> {
    review_on(conn, "WHERE request_id=?1", params![request_id])
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
                   FROM compute_external_pool_adapter_release_reviews {filter}"
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
    let request = request_by_id_on(conn, &review.request_id)?
        .ok_or_else(|| anyhow::anyhow!("external-pool Adapter release review lost its request"))?;
    let request_material = &request.envelope.request;
    let release = &request_material.release;
    let projected = conn
        .query_row(
            "SELECT 1 FROM compute_external_pool_adapter_release_reviews
              WHERE review_id=?1 AND review_schema=?2 AND review_digest=?3
                AND review_json=?4 AND canonicalization=?5 AND digest_algorithm=?6
                AND request_id=?7 AND request_digest=?8
                AND request_material_digest=?9 AND adapter_id=?10
                AND release_version=?11 AND decision=?12
                AND review_confirmation=?13 AND review_note IS ?14
                AND reviewed_by_admin_user_id=?15 AND reviewed_at=?16
                AND idempotency_scope=?17 AND idempotency_key=?18
                AND created_at=?16",
            params![
                stored.envelope.review_id,
                stored.envelope.schema,
                stored.envelope.review_digest,
                stored.review_json,
                stored.envelope.canonicalization,
                stored.envelope.digest_algorithm,
                review.request_id,
                review.request_digest,
                review.request_material_digest,
                review.adapter_id,
                review.release_version,
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
        REVIEW_DECISION_APPROVED => matches!(request.status.as_str(), "approved" | "staged"),
        REVIEW_DECISION_CHANGES_REQUESTED | REVIEW_DECISION_REJECTED => {
            request.status == review.decision
        }
        _ => false,
    };
    if stored.envelope.schema != EXTERNAL_POOL_ADAPTER_RELEASE_REVIEW_SCHEMA
        || stored.envelope.canonicalization
            != COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_CANONICALIZATION
        || stored.envelope.digest_algorithm
            != COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_DIGEST_ALGORITHM
        || review_json != stored.review_json
        || review_digest != stored.envelope.review_digest
        || review.request_digest != request.envelope.request_digest
        || review.request_material_digest != request.envelope.request_material_digest
        || review.adapter_id != release.adapter_id
        || review.release_version != release.release_version
        || review.reviewed_by_admin_user_id == request_material.submitted_by_admin_user_id
        || request.reviewed_by_admin_user_id.as_deref()
            != Some(review.reviewed_by_admin_user_id.as_str())
        || request.reviewed_at.as_deref() != Some(review.reviewed_at.as_str())
        || request_material.submitted_at > review.reviewed_at
        || !status_matches
        || !projected
    {
        bail!("external-pool Adapter release review failed exact readback audit");
    }
    Ok(stored)
}

fn validate_review_material(stored: &StoredReview) -> Result<()> {
    let envelope = &stored.envelope;
    let review = &envelope.review;
    validate_exact(&envelope.review_id, "stored review ID", 160)?;
    validate_digest(&envelope.review_digest, "stored review digest")?;
    validate_exact(&review.request_id, "stored review request ID", 160)?;
    validate_digest(&review.request_digest, "stored review request digest")?;
    validate_digest(
        &review.request_material_digest,
        "stored review request material digest",
    )?;
    validate_exact(&review.adapter_id, "stored review Adapter ID", 160)?;
    validate_exact(&review.release_version, "stored review release version", 80)?;
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
    if review.review_confirmation != EXTERNAL_POOL_ADAPTER_RELEASE_REVIEW_CONFIRMATION
        || !matches!(
            review.decision.as_str(),
            REVIEW_DECISION_APPROVED | REVIEW_DECISION_CHANGES_REQUESTED | REVIEW_DECISION_REJECTED
        )
    {
        bail!("external-pool Adapter release stored review authority is invalid");
    }
    if let Some(note) = &review.review_note {
        validate_optional_note(note, "stored review note", 2_000)?;
        if note.is_empty() {
            bail!("external-pool Adapter release stored review note is empty");
        }
    }
    if review.decision != REVIEW_DECISION_APPROVED && review.review_note.is_none() {
        bail!("external-pool Adapter release stored non-approval lacks a note");
    }
    Ok(())
}
