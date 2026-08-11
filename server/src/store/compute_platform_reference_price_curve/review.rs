use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{params, TransactionBehavior};

use crate::{
    compute_federation::platform_reference_price_curve::{
        COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_CANONICALIZATION,
        COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_DIGEST_ALGORITHM,
    },
    store::{new_id, Store},
};

use super::{
    canonical::canonical_review_json_and_digest,
    read::{batch_by_id_on, review_by_batch_on, review_by_idempotency_on},
    types::{
        ComputePlatformReferencePriceCurveReviewReceipt,
        ReviewComputePlatformReferencePriceCurveBatch, StoredReviewEnvelope, StoredReviewMaterial,
        PLATFORM_REFERENCE_PRICE_CURVE_REVIEW_CONFIRMATION,
        PLATFORM_REFERENCE_PRICE_CURVE_REVIEW_SCHEMA, REVIEW_DECISION_APPROVED,
        REVIEW_DECISION_CHANGES_REQUESTED, REVIEW_DECISION_REJECTED,
    },
};

impl Store {
    pub(crate) fn review_compute_platform_reference_price_curve_batch(
        &self,
        mut input: ReviewComputePlatformReferencePriceCurveBatch,
    ) -> Result<ComputePlatformReferencePriceCurveReviewReceipt> {
        validate_input(&mut input)?;
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(stored) = review_by_idempotency_on(
            &transaction,
            &input.idempotency_scope,
            &input.idempotency_key,
        )? {
            ensure_replay(&stored, &input)?;
            let receipt = stored.into_receipt(true);
            transaction.commit()?;
            return Ok(receipt);
        }

        let batch = batch_by_id_on(&transaction, &input.batch_id)?
            .ok_or_else(|| anyhow::anyhow!("platform reference price curve batch is absent"))?;
        if let Some(stored) = review_by_batch_on(&transaction, &input.batch_id)? {
            ensure_replay(&stored, &input)?;
            let receipt = stored.into_receipt(true);
            transaction.commit()?;
            return Ok(receipt);
        }
        if batch.status != "submitted"
            || batch.envelope.batch_digest != input.expected_batch_digest
            || batch.envelope.batch_material_digest != input.expected_batch_material_digest
        {
            bail!("only the exact submitted reference price curve batch can be reviewed");
        }
        if batch.envelope.batch.submitted_by_admin_user_id == input.reviewed_by_admin_user_id {
            bail!("reference price curve submitter cannot review the same batch");
        }

        let material = &batch.envelope.batch;
        let mut envelope = StoredReviewEnvelope {
            schema: PLATFORM_REFERENCE_PRICE_CURVE_REVIEW_SCHEMA.to_string(),
            review_id: new_id("compute_platform_reference_price_curve_review"),
            review_digest: String::new(),
            canonicalization: COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_CANONICALIZATION.to_string(),
            digest_algorithm: COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_DIGEST_ALGORITHM.to_string(),
            review: StoredReviewMaterial {
                batch_id: batch.envelope.batch_id.clone(),
                batch_digest: batch.envelope.batch_digest.clone(),
                batch_material_digest: batch.envelope.batch_material_digest.clone(),
                curve_id: material.curve_id.clone(),
                curve_version: material.curve_version,
                entry_set_digest: material.entry_set_digest.clone(),
                decision: input.decision.clone(),
                review_confirmation: input.review_confirmation.clone(),
                review_note: input.review_note.clone(),
                reviewed_by_admin_user_id: input.reviewed_by_admin_user_id.clone(),
                reviewed_at: now_nanos(),
            },
        };
        let (_, review_digest) = canonical_review_json_and_digest(&envelope)?;
        envelope.review_digest = review_digest;
        let (review_json, digest) = canonical_review_json_and_digest(&envelope)?;
        if digest != envelope.review_digest {
            bail!("platform reference price curve review digest changed before persistence");
        }
        let review = &envelope.review;
        transaction.execute(
            "INSERT INTO compute_platform_reference_price_curve_reviews (
                review_id, review_schema, review_digest, review_json,
                canonicalization, digest_algorithm, batch_id, batch_digest,
                batch_material_digest, curve_id, curve_version, entry_set_digest,
                decision, review_confirmation, review_note, reviewed_by_admin_user_id,
                reviewed_at, idempotency_scope, idempotency_key, created_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                ?14, ?15, ?16, ?17, ?18, ?19, ?17
             )",
            params![
                envelope.review_id,
                envelope.schema,
                envelope.review_digest,
                review_json,
                envelope.canonicalization,
                envelope.digest_algorithm,
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
                input.idempotency_scope,
                input.idempotency_key,
            ],
        )?;
        let stored = review_by_batch_on(&transaction, &input.batch_id)?.ok_or_else(|| {
            anyhow::anyhow!("reference price curve review is absent after insert")
        })?;
        let receipt = stored.into_receipt(false);
        transaction.commit()?;
        Ok(receipt)
    }
}

fn ensure_replay(
    stored: &super::types::StoredReview,
    input: &ReviewComputePlatformReferencePriceCurveBatch,
) -> Result<()> {
    let review = &stored.envelope.review;
    if review.batch_id != input.batch_id
        || review.batch_digest != input.expected_batch_digest
        || review.batch_material_digest != input.expected_batch_material_digest
        || review.decision != input.decision
        || review.review_confirmation != input.review_confirmation
        || review.review_note != input.review_note
        || review.reviewed_by_admin_user_id != input.reviewed_by_admin_user_id
        || stored.idempotency_scope != input.idempotency_scope
        || stored.idempotency_key != input.idempotency_key
    {
        bail!("platform reference price curve review replay conflicts with immutable history");
    }
    Ok(())
}

fn validate_input(input: &mut ReviewComputePlatformReferencePriceCurveBatch) -> Result<()> {
    validate_exact(&input.batch_id, "review batch ID", 160)?;
    validate_digest(&input.expected_batch_digest, "review batch digest")?;
    validate_digest(
        &input.expected_batch_material_digest,
        "review batch material digest",
    )?;
    validate_exact(
        &input.reviewed_by_admin_user_id,
        "reviewing administrator",
        160,
    )?;
    validate_exact(&input.idempotency_scope, "review idempotency scope", 200)?;
    validate_exact(&input.idempotency_key, "review idempotency key", 160)?;
    if input.review_confirmation != PLATFORM_REFERENCE_PRICE_CURVE_REVIEW_CONFIRMATION {
        bail!("platform reference price curve review confirmation is not exact");
    }
    if !matches!(
        input.decision.as_str(),
        REVIEW_DECISION_APPROVED | REVIEW_DECISION_CHANGES_REQUESTED | REVIEW_DECISION_REJECTED
    ) {
        bail!("platform reference price curve review decision is unsupported");
    }
    input.review_note = normalize_note(input.review_note.take())?;
    if input.decision != REVIEW_DECISION_APPROVED && input.review_note.is_none() {
        bail!("non-approved reference price curve review requires a note");
    }
    Ok(())
}

fn normalize_note(value: Option<String>) -> Result<Option<String>> {
    let Some(value) = value else { return Ok(None) };
    let value = value.trim().to_string();
    if value.is_empty()
        || value.chars().count() > 2_000
        || value
            .chars()
            .any(|character| character.is_control() && character != '\n' && character != '\t')
    {
        bail!("platform reference price curve review note is invalid");
    }
    Ok(Some(value))
}

pub(super) fn validate_exact(value: &str, label: &str, max: usize) -> Result<()> {
    if value.trim().is_empty()
        || value != value.trim()
        || value.chars().count() > max
        || value.chars().any(char::is_control)
    {
        bail!("platform reference price curve {label} is invalid");
    }
    Ok(())
}

pub(super) fn validate_optional_note(value: &str, label: &str, max: usize) -> Result<()> {
    if value != value.trim()
        || value.chars().count() > max
        || value
            .chars()
            .any(|character| character.is_control() && character != '\n' && character != '\t')
    {
        bail!("platform reference price curve {label} is invalid");
    }
    Ok(())
}

pub(super) fn validate_digest(value: &str, label: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("platform reference price curve {label} is not lowercase SHA-256");
    }
    Ok(())
}

pub(super) fn now_nanos() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
