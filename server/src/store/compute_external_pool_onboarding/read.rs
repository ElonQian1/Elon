use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension, Row};
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::external_pool_onboarding::{
        canonical_external_pool_onboarding_request_json_and_digest,
        validate_external_pool_onboarding_request_envelope,
        ComputeExternalPoolOnboardingRequestEnvelope,
    },
    compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256,
};

use super::{
    canonical::{canonical_application_json_and_digest, canonical_review_json_and_digest},
    types::{
        ExternalPoolOnboardingApplicationReceipt, ExternalPoolOnboardingRequestReceipt,
        ExternalPoolOnboardingReviewReceipt, StoredApplication, StoredApplicationEnvelope,
        StoredRequest, StoredReview, StoredReviewEnvelope,
        EXTERNAL_POOL_ONBOARDING_APPLICATION_SCHEMA, EXTERNAL_POOL_ONBOARDING_REVIEW_SCHEMA,
    },
};
use crate::store::compute_provider_registry::registered_provider_version_on;

const MAX_TARGET_PROVIDER_JSON_BYTES: usize = 256 * 1024;

pub(super) fn request_by_id_on(
    connection: &Connection,
    request_id: &str,
) -> Result<Option<StoredRequest>> {
    request_on(connection, "WHERE request_id=?1", params![request_id])
}

pub(super) fn request_by_idempotency_on(
    connection: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredRequest>> {
    request_on(
        connection,
        "WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

fn request_on<P: rusqlite::Params>(
    connection: &Connection,
    filter: &str,
    parameters: P,
) -> Result<Option<StoredRequest>> {
    let stored = connection
        .query_row(
            &format!(
                "SELECT request_json, target_provider_digest, target_provider_jcs,
                        target_provider_registry_json, status, idempotency_scope, idempotency_key
                   FROM compute_external_pool_onboarding_requests {filter}"
            ),
            parameters,
            |row| {
                let request_json: String = row.get(0)?;
                let envelope =
                    serde_json::from_str::<ComputeExternalPoolOnboardingRequestEnvelope>(
                        &request_json,
                    )
                    .map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            request_json.len(),
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })?;
                Ok(StoredRequest {
                    envelope,
                    request_json,
                    target_provider_digest: row.get(1)?,
                    target_provider_jcs: row.get(2)?,
                    target_provider_registry_json: row.get(3)?,
                    status: row.get(4)?,
                    idempotency_scope: row.get(5)?,
                    idempotency_key: row.get(6)?,
                })
            },
        )
        .optional()?;
    stored
        .map(|stored| audit_request(connection, stored))
        .transpose()
}

fn audit_request(connection: &Connection, stored: StoredRequest) -> Result<StoredRequest> {
    validate_external_pool_onboarding_request_envelope(&stored.envelope)?;
    let (request_json, request_digest) =
        canonical_external_pool_onboarding_request_json_and_digest(&stored.envelope)?;
    let provider = &stored.envelope.request.target_provider;
    let provider_registry_json = serde_json::to_string(provider)?;
    let provider_digest = hex::encode(Sha256::digest(provider_registry_json.as_bytes()));
    let (provider_jcs, _) =
        canonical_compute_plugin_ijson_and_sha256(provider, MAX_TARGET_PROVIDER_JSON_BYTES)?;
    let request = &stored.envelope.request;
    let adapter = &request.adapter_intent;
    let credential = &request.credential_intent;
    let projection_matches = connection
        .query_row(
            "SELECT 1 FROM compute_external_pool_onboarding_requests
              WHERE request_id=?1 AND request_schema=?2 AND request_digest=?3
                AND request_json=?4 AND canonicalization=?5 AND digest_algorithm=?6
                AND target_provider_policy_revision=1 AND target_provider_digest=?7
                AND target_provider_jcs=?8 AND target_provider_registry_json=?9
                AND provider_id=?10 AND provider_kind=?11
                AND provider_owner_account_id=?12 AND settlement_account_id=?13
                AND adapter_id=?14 AND adapter_release_version=?15
                AND adapter_config_revision=?16 AND adapter_config_digest=?17
                AND non_bearer_credential_ref IS ?18 AND credential_hint IS ?19
                AND external_evidence_ref IS ?20 AND external_evidence_sha256 IS ?21
                AND confirmation=?22 AND owner_note=?23
                AND requested_by_owner_user_id=?24 AND requested_at=?25
                AND idempotency_scope=?26 AND idempotency_key=?27",
            params![
                stored.envelope.request_id,
                stored.envelope.schema,
                stored.envelope.request_digest,
                stored.request_json,
                stored.envelope.canonicalization,
                stored.envelope.digest_algorithm,
                stored.target_provider_digest,
                stored.target_provider_jcs,
                stored.target_provider_registry_json,
                provider.provider_id,
                provider.provider_kind,
                provider.owner_account_id,
                provider.settlement_account_id,
                adapter.expected_adapter_id,
                adapter.expected_release_version,
                adapter.expected_config_revision,
                adapter.expected_config_digest,
                credential.non_bearer_credential_ref,
                credential.credential_hint,
                request.external_evidence_ref,
                request.external_evidence_sha256,
                request.confirmation,
                request.owner_note,
                request.requested_by_owner_user_id,
                request.submitted_at,
                stored.idempotency_scope,
                stored.idempotency_key,
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if request_json != stored.request_json
        || request_digest != stored.envelope.request_digest
        || provider_digest != stored.target_provider_digest
        || provider_jcs != stored.target_provider_jcs
        || provider_registry_json != stored.target_provider_registry_json
        || stored.idempotency_key != request.idempotency_key
        || !matches!(
            stored.status.as_str(),
            "submitted" | "approved" | "changes_requested" | "rejected" | "canceled" | "applied"
        )
        || !projection_matches
    {
        bail!("external-pool onboarding request failed exact readback audit");
    }
    Ok(stored)
}

pub(super) fn review_by_request_on(
    connection: &Connection,
    request_id: &str,
) -> Result<Option<StoredReview>> {
    review_on(connection, "WHERE request_id=?1", params![request_id])
}

pub(super) fn review_by_idempotency_on(
    connection: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredReview>> {
    review_on(
        connection,
        "WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

fn review_on<P: rusqlite::Params>(
    connection: &Connection,
    filter: &str,
    parameters: P,
) -> Result<Option<StoredReview>> {
    let stored = connection
        .query_row(
            &format!(
                "SELECT review_json, idempotency_scope, idempotency_key
                   FROM compute_external_pool_onboarding_reviews {filter}"
            ),
            parameters,
            stored_review_from_row,
        )
        .optional()?;
    stored
        .map(|stored| audit_review(connection, stored))
        .transpose()
}

fn stored_review_from_row(row: &Row<'_>) -> rusqlite::Result<StoredReview> {
    let review_json: String = row.get(0)?;
    let envelope = serde_json::from_str::<StoredReviewEnvelope>(&review_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            review_json.len(),
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })?;
    Ok(StoredReview {
        envelope,
        review_json,
        idempotency_scope: row.get(1)?,
        idempotency_key: row.get(2)?,
    })
}

fn audit_review(connection: &Connection, stored: StoredReview) -> Result<StoredReview> {
    let (review_json, digest) = canonical_review_json_and_digest(&stored.envelope)?;
    let review = &stored.envelope.review;
    let request = request_by_id_on(connection, &review.request_id)?
        .ok_or_else(|| anyhow::anyhow!("external-pool review lost its request"))?;
    let projection_matches = connection
        .query_row(
            "SELECT 1 FROM compute_external_pool_onboarding_reviews
              WHERE review_id=?1 AND review_schema=?2 AND review_digest=?3
                AND review_json=?4 AND canonicalization=?5 AND digest_algorithm=?6
                AND request_id=?7 AND request_digest=?8 AND provider_id=?9
                AND provider_owner_account_id=?10 AND decision=?11
                AND review_reason IS ?12 AND reviewed_by_user_id=?13
                AND reviewed_at=?14 AND idempotency_scope=?15
                AND idempotency_key=?16 AND created_at=?14",
            params![
                stored.envelope.review_id,
                stored.envelope.schema,
                stored.envelope.review_digest,
                stored.review_json,
                stored.envelope.canonicalization,
                stored.envelope.digest_algorithm,
                review.request_id,
                review.request_digest,
                review.provider_id,
                review.provider_owner_account_id,
                review.decision,
                review.review_reason,
                review.reviewed_by_user_id,
                review.reviewed_at,
                stored.idempotency_scope,
                stored.idempotency_key,
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if stored.envelope.schema != EXTERNAL_POOL_ONBOARDING_REVIEW_SCHEMA
        || review_json != stored.review_json
        || digest != stored.envelope.review_digest
        || review.request_digest != request.envelope.request_digest
        || review.provider_id != request.envelope.request.target_provider.provider_id
        || review.provider_owner_account_id
            != request.envelope.request.target_provider.owner_account_id
        || review.reviewed_by_user_id == review.provider_owner_account_id
        || !(request.status == review.decision
            || (review.decision == "approved" && request.status == "applied"))
        || !projection_matches
    {
        bail!("external-pool onboarding review failed exact readback audit");
    }
    Ok(stored)
}

pub(super) fn application_by_request_on(
    connection: &Connection,
    request_id: &str,
) -> Result<Option<StoredApplication>> {
    application_on(connection, "WHERE request_id=?1", params![request_id])
}

pub(super) fn application_by_idempotency_on(
    connection: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredApplication>> {
    application_on(
        connection,
        "WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

fn application_on<P: rusqlite::Params>(
    connection: &Connection,
    filter: &str,
    parameters: P,
) -> Result<Option<StoredApplication>> {
    let stored = connection
        .query_row(
            &format!(
                "SELECT application_json, target_provider_jcs, target_provider_registry_json,
                        idempotency_scope, idempotency_key
                   FROM compute_external_pool_onboarding_applications {filter}"
            ),
            parameters,
            |row| {
                let application_json: String = row.get(0)?;
                let envelope = serde_json::from_str::<StoredApplicationEnvelope>(&application_json)
                    .map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            application_json.len(),
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })?;
                Ok(StoredApplication {
                    envelope,
                    application_json,
                    target_provider_jcs: row.get(1)?,
                    target_provider_registry_json: row.get(2)?,
                    idempotency_scope: row.get(3)?,
                    idempotency_key: row.get(4)?,
                })
            },
        )
        .optional()?;
    stored
        .map(|stored| audit_application(connection, stored))
        .transpose()
}

fn audit_application(
    connection: &Connection,
    stored: StoredApplication,
) -> Result<StoredApplication> {
    let (application_json, digest) = canonical_application_json_and_digest(&stored.envelope)?;
    let application = &stored.envelope.application;
    let request = request_by_id_on(connection, &application.request_id)?
        .ok_or_else(|| anyhow::anyhow!("external-pool application lost its request"))?;
    let review = review_by_request_on(connection, &application.request_id)?
        .ok_or_else(|| anyhow::anyhow!("external-pool application lost its review"))?;
    let registered = registered_provider_version_on(
        connection,
        &application.provider_id,
        application.target_provider_policy_revision,
    )?
    .ok_or_else(|| anyhow::anyhow!("external-pool application lost its Provider revision"))?;
    if stored.envelope.schema != EXTERNAL_POOL_ONBOARDING_APPLICATION_SCHEMA
        || application_json != stored.application_json
        || digest != stored.envelope.application_digest
        || application.request_digest != request.envelope.request_digest
        || application.review_id != review.envelope.review_id
        || application.review_digest != review.envelope.review_digest
        || review.envelope.review.decision != "approved"
        || application.provider_id != request.envelope.request.target_provider.provider_id
        || application.target_provider_digest != request.target_provider_digest
        || stored.target_provider_jcs != request.target_provider_jcs
        || stored.target_provider_registry_json != request.target_provider_registry_json
        || application.approved_by_user_id
            != request.envelope.request.target_provider.owner_account_id
        || application.reviewed_by_user_id != review.envelope.review.reviewed_by_user_id
        || request.status != "applied"
        || registered.provider_digest != application.target_provider_digest
        || serde_json::to_string(&registered.provider)? != stored.target_provider_registry_json
    {
        bail!("external-pool onboarding application failed exact readback audit");
    }
    Ok(stored)
}

pub(super) fn request_receipt(
    stored: StoredRequest,
    replayed: bool,
) -> ExternalPoolOnboardingRequestReceipt {
    let request = &stored.envelope.request;
    ExternalPoolOnboardingRequestReceipt {
        schema: crate::compute_federation::external_pool_onboarding::COMPUTE_EXTERNAL_POOL_ONBOARDING_REQUEST_SCHEMA,
        request_id: stored.envelope.request_id,
        request_digest: stored.envelope.request_digest,
        provider_id: request.target_provider.provider_id.clone(),
        provider_owner_account_id: request.target_provider.owner_account_id.clone(),
        target_provider_digest: stored.target_provider_digest,
        status: stored.status,
        credential_ref_present: request
            .credential_intent
            .non_bearer_credential_ref
            .is_some(),
        credential_hint: request.credential_intent.credential_hint.clone(),
        requested_at: request.submitted_at.clone(),
        replayed,
        onboarding_effect: "none",
    }
}

pub(super) fn review_receipt(
    stored: StoredReview,
    replayed: bool,
) -> ExternalPoolOnboardingReviewReceipt {
    let review = stored.envelope.review;
    ExternalPoolOnboardingReviewReceipt {
        schema: EXTERNAL_POOL_ONBOARDING_REVIEW_SCHEMA,
        review_id: stored.envelope.review_id,
        review_digest: stored.envelope.review_digest,
        request_id: review.request_id,
        request_digest: review.request_digest,
        provider_id: review.provider_id,
        provider_owner_account_id: review.provider_owner_account_id,
        decision: review.decision,
        review_reason: review.review_reason,
        reviewed_by_user_id: review.reviewed_by_user_id,
        reviewed_at: review.reviewed_at,
        replayed,
        onboarding_effect: "none",
    }
}

pub(super) fn application_receipt(
    stored: StoredApplication,
    replayed: bool,
) -> ExternalPoolOnboardingApplicationReceipt {
    let application = stored.envelope.application;
    ExternalPoolOnboardingApplicationReceipt {
        schema: EXTERNAL_POOL_ONBOARDING_APPLICATION_SCHEMA,
        application_id: stored.envelope.application_id,
        application_digest: stored.envelope.application_digest,
        request_id: application.request_id,
        request_digest: application.request_digest,
        review_id: application.review_id,
        review_digest: application.review_digest,
        provider_id: application.provider_id,
        provider_digest: application.target_provider_digest,
        approved_by_user_id: application.approved_by_user_id,
        reviewed_by_user_id: application.reviewed_by_user_id,
        applied_by_user_id: application.applied_by_user_id,
        apply_confirmation: application.apply_confirmation,
        applied_at: application.applied_at,
        replayed,
        onboarding_effect: "provider_registered_only",
    }
}
