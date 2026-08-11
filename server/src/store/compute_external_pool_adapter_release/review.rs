use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{params, TransactionBehavior};

use crate::{
    compute_federation::external_pool_adapter_release::{
        COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_CANONICALIZATION,
        COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_DIGEST_ALGORITHM,
    },
    store::{new_id, Store},
};

use super::{
    canonical::canonical_review_json_and_digest,
    read::{request_by_id_on, review_by_idempotency_on, review_by_request_on},
    types::{
        ExternalPoolAdapterReleaseReviewReceipt, ReviewExternalPoolAdapterReleaseRequest,
        StoredReviewEnvelope, StoredReviewMaterial,
        EXTERNAL_POOL_ADAPTER_RELEASE_REVIEW_CONFIRMATION,
        EXTERNAL_POOL_ADAPTER_RELEASE_REVIEW_SCHEMA, REVIEW_DECISION_APPROVED,
        REVIEW_DECISION_CHANGES_REQUESTED, REVIEW_DECISION_REJECTED,
    },
};

impl Store {
    pub(crate) fn review_external_pool_adapter_release_request(
        &self,
        mut input: ReviewExternalPoolAdapterReleaseRequest,
    ) -> Result<ExternalPoolAdapterReleaseReviewReceipt> {
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

        let request = request_by_id_on(&transaction, &input.request_id)?
            .ok_or_else(|| anyhow::anyhow!("external-pool Adapter release request is absent"))?;
        if let Some(stored) = review_by_request_on(&transaction, &input.request_id)? {
            ensure_replay(&stored, &input)?;
            let receipt = stored.into_receipt(true);
            transaction.commit()?;
            return Ok(receipt);
        }
        if request.status != "submitted"
            || request.envelope.request_digest != input.expected_request_digest
            || request.envelope.request_material_digest != input.expected_request_material_digest
        {
            bail!("only the exact submitted Adapter release request can be reviewed");
        }
        if request.envelope.request.submitted_by_admin_user_id == input.reviewed_by_admin_user_id {
            bail!("Adapter release submitter cannot review the same request");
        }

        let release = &request.envelope.request.release;
        let mut envelope = StoredReviewEnvelope {
            schema: EXTERNAL_POOL_ADAPTER_RELEASE_REVIEW_SCHEMA.to_string(),
            review_id: new_id("compute_external_pool_adapter_release_review"),
            review_digest: String::new(),
            canonicalization: COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_CANONICALIZATION.to_string(),
            digest_algorithm: COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_DIGEST_ALGORITHM.to_string(),
            review: StoredReviewMaterial {
                request_id: request.envelope.request_id.clone(),
                request_digest: request.envelope.request_digest.clone(),
                request_material_digest: request.envelope.request_material_digest.clone(),
                adapter_id: release.adapter_id.clone(),
                release_version: release.release_version.clone(),
                decision: input.decision.clone(),
                review_confirmation: input.review_confirmation.clone(),
                review_note: input.review_note.clone(),
                reviewed_by_admin_user_id: input.reviewed_by_admin_user_id.clone(),
                reviewed_at: now_nanos(),
            },
        };
        let (_, digest) = canonical_review_json_and_digest(&envelope)?;
        envelope.review_digest = digest;
        let (review_json, _) = canonical_review_json_and_digest(&envelope)?;
        let review = &envelope.review;
        transaction.execute(
            "INSERT INTO compute_external_pool_adapter_release_reviews (
                review_id, review_schema, review_digest, review_json,
                canonicalization, digest_algorithm, request_id, request_digest,
                request_material_digest, adapter_id, release_version, decision,
                review_confirmation, review_note, reviewed_by_admin_user_id, reviewed_at,
                idempotency_scope, idempotency_key, created_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                ?13, ?14, ?15, ?16, ?17, ?18, ?16
             )",
            params![
                envelope.review_id,
                envelope.schema,
                envelope.review_digest,
                review_json,
                envelope.canonicalization,
                envelope.digest_algorithm,
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
                input.idempotency_scope,
                input.idempotency_key,
            ],
        )?;
        let stored = review_by_request_on(&transaction, &input.request_id)?
            .ok_or_else(|| anyhow::anyhow!("Adapter release review is absent after insert"))?;
        let receipt = stored.into_receipt(false);
        transaction.commit()?;
        Ok(receipt)
    }
}

fn ensure_replay(
    stored: &super::types::StoredReview,
    input: &ReviewExternalPoolAdapterReleaseRequest,
) -> Result<()> {
    let review = &stored.envelope.review;
    if review.request_id != input.request_id
        || review.request_digest != input.expected_request_digest
        || review.request_material_digest != input.expected_request_material_digest
        || review.decision != input.decision
        || review.review_confirmation != input.review_confirmation
        || review.review_note != input.review_note
        || review.reviewed_by_admin_user_id != input.reviewed_by_admin_user_id
        || stored.idempotency_scope != input.idempotency_scope
        || stored.idempotency_key != input.idempotency_key
    {
        bail!("external-pool Adapter release review replay conflicts with immutable history");
    }
    Ok(())
}

fn validate_input(input: &mut ReviewExternalPoolAdapterReleaseRequest) -> Result<()> {
    validate_exact(&input.request_id, "review request ID", 160)?;
    validate_digest(&input.expected_request_digest, "review request digest")?;
    validate_digest(
        &input.expected_request_material_digest,
        "review request material digest",
    )?;
    validate_exact(
        &input.reviewed_by_admin_user_id,
        "reviewing administrator",
        160,
    )?;
    validate_exact(&input.idempotency_scope, "review idempotency scope", 200)?;
    validate_exact(&input.idempotency_key, "review idempotency key", 160)?;
    if input.review_confirmation != EXTERNAL_POOL_ADAPTER_RELEASE_REVIEW_CONFIRMATION {
        bail!("external-pool Adapter release review confirmation is not exact");
    }
    if !matches!(
        input.decision.as_str(),
        REVIEW_DECISION_APPROVED | REVIEW_DECISION_CHANGES_REQUESTED | REVIEW_DECISION_REJECTED
    ) {
        bail!("external-pool Adapter release review decision is unsupported");
    }
    input.review_note = normalize_note(input.review_note.take())?;
    if input.decision != REVIEW_DECISION_APPROVED && input.review_note.is_none() {
        bail!("non-approved Adapter release review requires a note");
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
        bail!("external-pool Adapter release review note is invalid");
    }
    Ok(Some(value))
}

pub(super) fn validate_exact(value: &str, label: &str, max: usize) -> Result<()> {
    if value.trim().is_empty()
        || value != value.trim()
        || value.chars().count() > max
        || value.chars().any(char::is_control)
    {
        bail!("external-pool Adapter release {label} is invalid");
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
        bail!("external-pool Adapter release {label} is invalid");
    }
    Ok(())
}

pub(super) fn validate_digest(value: &str, label: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("external-pool Adapter release {label} is not lowercase SHA-256");
    }
    Ok(())
}

pub(super) fn now_nanos() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
