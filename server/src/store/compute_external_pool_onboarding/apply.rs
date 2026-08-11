use anyhow::{bail, Result};
use rusqlite::{params, TransactionBehavior};

use crate::compute_federation::{
    external_pool_onboarding::{
        COMPUTE_EXTERNAL_POOL_ONBOARDING_CANONICALIZATION,
        COMPUTE_EXTERNAL_POOL_ONBOARDING_DIGEST_ALGORITHM,
    },
    provider::{PROVIDER_KIND_EXTERNAL_POOL, PROVIDER_STATUS_REGISTERING},
};

use super::{
    canonical::canonical_application_json_and_digest,
    read::{
        application_by_idempotency_on, application_by_request_on, application_receipt,
        request_by_id_on, review_by_request_on,
    },
    review::{now_nanos, validate_digest, validate_exact},
    types::{
        ApplyExternalPoolOnboarding, ExternalPoolOnboardingApplicationReceipt,
        StoredApplicationEnvelope, StoredApplicationMaterial,
        EXTERNAL_POOL_ONBOARDING_APPLICATION_SCHEMA, EXTERNAL_POOL_ONBOARDING_APPLY_CONFIRMATION,
        REVIEW_DECISION_APPROVED,
    },
};
use crate::store::{
    compute_provider_registry::{current_registered_provider_on, register_compute_provider_on},
    new_id, Store,
};

impl Store {
    pub(crate) fn apply_external_pool_onboarding(
        &self,
        input: ApplyExternalPoolOnboarding,
    ) -> Result<ExternalPoolOnboardingApplicationReceipt> {
        validate_input(&input)?;
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(stored) = application_by_idempotency_on(
            &transaction,
            &input.idempotency_scope,
            &input.idempotency_key,
        )? {
            ensure_replay(&stored, &input)?;
            let receipt = application_receipt(stored, true);
            transaction.commit()?;
            return Ok(receipt);
        }
        let request = request_by_id_on(&transaction, &input.request_id)?
            .ok_or_else(|| anyhow::anyhow!("external-pool onboarding request does not exist"))?;
        if let Some(stored) = application_by_request_on(&transaction, &input.request_id)? {
            ensure_replay(&stored, &input)?;
            let receipt = application_receipt(stored, true);
            transaction.commit()?;
            return Ok(receipt);
        }
        if request.status != REVIEW_DECISION_APPROVED
            || request.envelope.request_digest != input.expected_request_digest
        {
            bail!("only the exact approved external-pool onboarding request can be applied");
        }
        let review = review_by_request_on(&transaction, &input.request_id)?
            .ok_or_else(|| anyhow::anyhow!("external-pool onboarding review does not exist"))?;
        if review.envelope.review.decision != REVIEW_DECISION_APPROVED
            || review.envelope.review_digest != input.expected_review_digest
            || review.envelope.review.request_digest != input.expected_request_digest
        {
            bail!("external-pool onboarding approved review is stale or conflicts");
        }
        let provider = &request.envelope.request.target_provider;
        if current_registered_provider_on(&transaction, &provider.provider_id)?.is_some() {
            bail!("external-pool onboarding Provider ID is already registered");
        }
        let registered = register_compute_provider_on(&transaction, provider)?;
        if registered.replayed
            || registered.provider_digest != request.target_provider_digest
            || registered.provider.provider_kind != PROVIDER_KIND_EXTERNAL_POOL
            || registered.provider.status != PROVIDER_STATUS_REGISTERING
            || registered.provider.policy_revision != 1
        {
            bail!("external-pool Provider was not registered from the exact approved request");
        }

        let request_material = &request.envelope.request;
        let adapter = &request_material.adapter_intent;
        let credential = &request_material.credential_intent;
        let application_id = new_id("compute_external_pool_onboarding_application");
        let applied_at = now_nanos();
        let settlement_account_id = provider
            .settlement_account_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("external-pool settlement account disappeared"))?;
        let mut envelope = StoredApplicationEnvelope {
            schema: EXTERNAL_POOL_ONBOARDING_APPLICATION_SCHEMA.to_string(),
            application_id,
            application_digest: String::new(),
            canonicalization: COMPUTE_EXTERNAL_POOL_ONBOARDING_CANONICALIZATION.to_string(),
            digest_algorithm: COMPUTE_EXTERNAL_POOL_ONBOARDING_DIGEST_ALGORITHM.to_string(),
            application: StoredApplicationMaterial {
                request_id: request.envelope.request_id.clone(),
                request_digest: request.envelope.request_digest.clone(),
                review_id: review.envelope.review_id.clone(),
                review_digest: review.envelope.review_digest.clone(),
                provider_id: provider.provider_id.clone(),
                provider_kind: provider.provider_kind.clone(),
                provider_owner_account_id: provider.owner_account_id.clone(),
                settlement_account_id,
                target_provider_policy_revision: provider.policy_revision,
                target_provider_digest: request.target_provider_digest.clone(),
                adapter_id: adapter.expected_adapter_id.clone(),
                adapter_release_version: adapter.expected_release_version.clone(),
                adapter_config_revision: adapter.expected_config_revision,
                adapter_config_digest: adapter.expected_config_digest.clone(),
                non_bearer_credential_ref: credential.non_bearer_credential_ref.clone(),
                credential_hint: credential.credential_hint.clone(),
                external_evidence_ref: request_material.external_evidence_ref.clone(),
                external_evidence_sha256: request_material.external_evidence_sha256.clone(),
                approved_by_user_id: provider.owner_account_id.clone(),
                reviewed_by_user_id: review.envelope.review.reviewed_by_user_id.clone(),
                applied_by_user_id: input.applied_by_user_id.clone(),
                apply_confirmation: input.apply_confirmation.clone(),
                applied_at,
            },
        };
        let (_, digest) = canonical_application_json_and_digest(&envelope)?;
        envelope.application_digest = digest;
        let (application_json, _) = canonical_application_json_and_digest(&envelope)?;
        let application = &envelope.application;
        transaction.execute(
            "INSERT INTO compute_external_pool_onboarding_applications (
                application_id, application_schema, application_digest, application_json,
                canonicalization, digest_algorithm, request_id, request_digest,
                review_id, review_digest, provider_id, provider_kind,
                provider_owner_account_id, settlement_account_id,
                target_provider_policy_revision, target_provider_digest,
                target_provider_jcs, target_provider_registry_json, adapter_id,
                adapter_release_version, adapter_config_revision, adapter_config_digest,
                non_bearer_credential_ref, credential_hint, external_evidence_ref,
                external_evidence_sha256, approved_by_user_id, reviewed_by_user_id,
                apply_confirmation, applied_by_user_id, applied_at,
                idempotency_scope, idempotency_key, created_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                ?13, ?14, 1, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23,
                ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?30
             )",
            params![
                envelope.application_id,
                envelope.schema,
                envelope.application_digest,
                application_json,
                envelope.canonicalization,
                envelope.digest_algorithm,
                application.request_id,
                application.request_digest,
                application.review_id,
                application.review_digest,
                application.provider_id,
                application.provider_kind,
                application.provider_owner_account_id,
                application.settlement_account_id,
                application.target_provider_digest,
                request.target_provider_jcs,
                request.target_provider_registry_json,
                application.adapter_id,
                application.adapter_release_version,
                application.adapter_config_revision,
                application.adapter_config_digest,
                application.non_bearer_credential_ref,
                application.credential_hint,
                application.external_evidence_ref,
                application.external_evidence_sha256,
                application.approved_by_user_id,
                application.reviewed_by_user_id,
                application.apply_confirmation,
                application.applied_by_user_id,
                application.applied_at,
                input.idempotency_scope,
                input.idempotency_key,
            ],
        )?;
        let stored = application_by_request_on(&transaction, &input.request_id)?
            .ok_or_else(|| anyhow::anyhow!("external-pool application is absent after insert"))?;
        let receipt = application_receipt(stored, false);
        transaction.commit()?;
        Ok(receipt)
    }
}

fn validate_input(input: &ApplyExternalPoolOnboarding) -> Result<()> {
    validate_exact(&input.request_id, "apply request ID", 160)?;
    validate_digest(&input.expected_request_digest, "apply request digest")?;
    validate_digest(&input.expected_review_digest, "apply review digest")?;
    validate_exact(&input.applied_by_user_id, "apply user ID", 160)?;
    validate_exact(&input.idempotency_scope, "apply idempotency scope", 200)?;
    validate_exact(&input.idempotency_key, "apply idempotency key", 160)?;
    if input.apply_confirmation != EXTERNAL_POOL_ONBOARDING_APPLY_CONFIRMATION {
        bail!("external-pool onboarding apply confirmation is not exact");
    }
    Ok(())
}

fn ensure_replay(
    stored: &super::types::StoredApplication,
    input: &ApplyExternalPoolOnboarding,
) -> Result<()> {
    let application = &stored.envelope.application;
    if application.request_id != input.request_id
        || application.request_digest != input.expected_request_digest
        || application.review_digest != input.expected_review_digest
        || application.applied_by_user_id != input.applied_by_user_id
        || application.apply_confirmation != input.apply_confirmation
        || stored.idempotency_scope != input.idempotency_scope
        || stored.idempotency_key != input.idempotency_key
    {
        bail!("external-pool onboarding apply replay conflicts with immutable history");
    }
    Ok(())
}
