use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{params, TransactionBehavior};

use crate::compute_federation::external_pool_onboarding::{
    COMPUTE_EXTERNAL_POOL_ONBOARDING_CANONICALIZATION,
    COMPUTE_EXTERNAL_POOL_ONBOARDING_DIGEST_ALGORITHM,
};

use super::{
    canonical::canonical_review_json_and_digest,
    read::{request_by_id_on, review_by_idempotency_on, review_by_request_on, review_receipt},
    types::{
        ExternalPoolOnboardingReviewReceipt, ReviewExternalPoolOnboardingRequest,
        StoredReviewEnvelope, StoredReviewMaterial, EXTERNAL_POOL_ONBOARDING_REVIEW_SCHEMA,
        REVIEW_DECISION_APPROVED, REVIEW_DECISION_CHANGES_REQUESTED, REVIEW_DECISION_REJECTED,
    },
};
use crate::store::{new_id, Store};

impl Store {
    pub(in crate::store) fn review_external_pool_onboarding_request(
        &self,
        mut input: ReviewExternalPoolOnboardingRequest,
    ) -> Result<ExternalPoolOnboardingReviewReceipt> {
        validate_input(&mut input)?;
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(stored) = review_by_idempotency_on(
            &transaction,
            &input.idempotency_scope,
            &input.idempotency_key,
        )? {
            ensure_replay(&stored, &input)?;
            let receipt = review_receipt(stored, true);
            transaction.commit()?;
            return Ok(receipt);
        }

        let request = request_by_id_on(&transaction, &input.request_id)?
            .ok_or_else(|| anyhow::anyhow!("external-pool onboarding request does not exist"))?;
        if request.envelope.request_digest != input.expected_request_digest {
            bail!("external-pool onboarding review request digest is stale");
        }
        if let Some(stored) = review_by_request_on(&transaction, &input.request_id)? {
            ensure_replay(&stored, &input)?;
            let receipt = review_receipt(stored, true);
            transaction.commit()?;
            return Ok(receipt);
        }
        if request.status != "submitted" {
            bail!("only a submitted external-pool onboarding request can be reviewed");
        }
        if request.envelope.request.requested_by_owner_user_id == input.reviewed_by_user_id {
            bail!("external-pool onboarding owner cannot review the same request");
        }

        let review_id = new_id("compute_external_pool_onboarding_review");
        let reviewed_at = now_nanos();
        let mut envelope = StoredReviewEnvelope {
            schema: EXTERNAL_POOL_ONBOARDING_REVIEW_SCHEMA.to_string(),
            review_id,
            review_digest: String::new(),
            canonicalization: COMPUTE_EXTERNAL_POOL_ONBOARDING_CANONICALIZATION.to_string(),
            digest_algorithm: COMPUTE_EXTERNAL_POOL_ONBOARDING_DIGEST_ALGORITHM.to_string(),
            review: StoredReviewMaterial {
                request_id: request.envelope.request_id.clone(),
                request_digest: request.envelope.request_digest.clone(),
                provider_id: request.envelope.request.target_provider.provider_id.clone(),
                provider_owner_account_id: request
                    .envelope
                    .request
                    .target_provider
                    .owner_account_id
                    .clone(),
                decision: input.decision.clone(),
                review_reason: input.review_reason.clone(),
                reviewed_by_user_id: input.reviewed_by_user_id.clone(),
                reviewed_at,
            },
        };
        let (_, digest) = canonical_review_json_and_digest(&envelope)?;
        envelope.review_digest = digest;
        let (review_json, _) = canonical_review_json_and_digest(&envelope)?;
        transaction.execute(
            "INSERT INTO compute_external_pool_onboarding_reviews (
                review_id, review_schema, review_digest, review_json,
                canonicalization, digest_algorithm, request_id, request_digest,
                provider_id, provider_owner_account_id, decision, review_reason,
                reviewed_by_user_id, reviewed_at, idempotency_scope,
                idempotency_key, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                       ?13, ?14, ?15, ?16, ?14)",
            params![
                envelope.review_id,
                envelope.schema,
                envelope.review_digest,
                review_json,
                envelope.canonicalization,
                envelope.digest_algorithm,
                envelope.review.request_id,
                envelope.review.request_digest,
                envelope.review.provider_id,
                envelope.review.provider_owner_account_id,
                envelope.review.decision,
                envelope.review.review_reason,
                envelope.review.reviewed_by_user_id,
                envelope.review.reviewed_at,
                input.idempotency_scope,
                input.idempotency_key,
            ],
        )?;
        let stored = review_by_request_on(&transaction, &input.request_id)?
            .ok_or_else(|| anyhow::anyhow!("external-pool review is absent after insert"))?;
        let receipt = review_receipt(stored, false);
        transaction.commit()?;
        Ok(receipt)
    }
}

fn ensure_replay(
    stored: &super::types::StoredReview,
    input: &ReviewExternalPoolOnboardingRequest,
) -> Result<()> {
    let review = &stored.envelope.review;
    if review.request_id != input.request_id
        || review.request_digest != input.expected_request_digest
        || review.decision != input.decision
        || review.review_reason != input.review_reason
        || review.reviewed_by_user_id != input.reviewed_by_user_id
        || stored.idempotency_scope != input.idempotency_scope
        || stored.idempotency_key != input.idempotency_key
    {
        bail!("external-pool onboarding review replay conflicts with immutable history");
    }
    Ok(())
}

fn validate_input(input: &mut ReviewExternalPoolOnboardingRequest) -> Result<()> {
    validate_exact(&input.request_id, "request ID", 160)?;
    validate_digest(&input.expected_request_digest, "request digest")?;
    validate_exact(&input.reviewed_by_user_id, "reviewer user ID", 160)?;
    validate_exact(&input.idempotency_scope, "review idempotency scope", 200)?;
    validate_exact(&input.idempotency_key, "review idempotency key", 160)?;
    if !matches!(
        input.decision.as_str(),
        REVIEW_DECISION_APPROVED | REVIEW_DECISION_CHANGES_REQUESTED | REVIEW_DECISION_REJECTED
    ) {
        bail!("external-pool onboarding review decision is unsupported");
    }
    input.review_reason = normalize_reason(input.review_reason.take())?;
    if input.decision != REVIEW_DECISION_APPROVED && input.review_reason.is_none() {
        bail!("non-approved external-pool onboarding review requires a reason");
    }
    Ok(())
}

fn normalize_reason(value: Option<String>) -> Result<Option<String>> {
    let Some(value) = value else { return Ok(None) };
    let value = value.trim().to_string();
    if value.is_empty()
        || value.chars().count() > 1_000
        || value
            .chars()
            .any(|character| character.is_control() && character != '\n' && character != '\t')
    {
        bail!("external-pool onboarding review reason is invalid");
    }
    Ok(Some(value))
}

pub(super) fn validate_exact(value: &str, label: &str, max: usize) -> Result<()> {
    if value.trim().is_empty()
        || value != value.trim()
        || value.chars().count() > max
        || value.chars().any(char::is_control)
    {
        bail!("external-pool onboarding {label} is invalid");
    }
    Ok(())
}

pub(super) fn validate_digest(value: &str, label: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("external-pool onboarding {label} is not lowercase SHA-256");
    }
    Ok(())
}

pub(super) fn now_nanos() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
